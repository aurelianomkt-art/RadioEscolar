//! Comandos chamados pela interface (invoke).

use crate::audio::{self, Cmd, InfoDispositivos};
use crate::biblioteca;
use crate::config::{Ajustes, AjustesPublicos, Backup};
use crate::db::{Db, Faixa, Pasta, Playlist, Registro};
use crate::motor::{agora_momento, painel, Estado, ItemFila, Nucleo, Origem, Painel, Repetir};
use radio_core::*;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, State};

type R<T> = Result<T, String>;

pub const PIN_NECESSARIO: &str = "PIN_NECESSARIO";

// ----------------------------------------------------------------- utilidades

pub fn persistir(db: &Db, n: &Nucleo) {
    if let Ok(s) = serde_json::to_string(&n.agenda) {
        let _ = db.gravar_config("agenda", &s);
    }
    if let Ok(s) = serde_json::to_string(&n.ajustes) {
        let _ = db.gravar_config("ajustes", &s);
    }
    if let Ok(s) = serde_json::to_string(&n.fila) {
        let _ = db.gravar_config("fila", &s);
    }
    let _ = db.gravar_config("modo", if n.modo == Modo::Livre { "Livre" } else { "Escolar" });
}

fn hash_pin(sal: &str, pin: &str) -> String {
    let mut h = Sha256::new();
    h.update(sal.as_bytes());
    h.update(b":");
    h.update(pin.trim().as_bytes());
    h.finalize().iter().map(|b| format!("{:02x}", b)).collect()
}

fn exigir_pin(est: &Estado) -> R<()> {
    let tem = !est.nucleo.lock().unwrap().ajustes.pin_hash.is_empty();
    if !tem {
        return Ok(());
    }
    match *est.desbloqueado_ate.lock().unwrap() {
        Some(t) if Instant::now() < t => Ok(()),
        _ => Err(PIN_NECESSARIO.into()),
    }
}

fn tocando(est: &Estado) -> bool {
    est.audio.status.lock().map(|s| s.tocando || s.em_anuncio).unwrap_or(false)
}

pub fn indexar_em_segundo_plano(app: AppHandle, est: Arc<Estado>) {
    if est.indexando.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(move || {
        let resumo = match Db::abrir(&est.db_path) {
            Ok(db) => {
                let r = biblioteca::indexar_tudo(&db);
                db.registrar(
                    "biblioteca",
                    if r.pastas_inacessiveis.is_empty() { "Info" } else { "Aviso" },
                    None,
                    &format!(
                        "Indexação: {} pasta(s), {} nova(s)/alterada(s), {} inalterada(s), {} ausente(s), {} inválida(s)",
                        r.pastas, r.novas_ou_alteradas, r.inalteradas, r.ausentes, r.invalidas
                    ),
                );
                Some(r)
            }
            Err(_) => None,
        };
        est.indexando.store(false, Ordering::SeqCst);
        let _ = app.emit("indexacao", resumo);
    });
}

// ----------------------------------------------------------------- painel / modo

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigCompleta {
    pub agenda: Agenda,
    pub ajustes: AjustesPublicos,
    pub versao: String,
    pub dados_dir: String,
    pub portatil: bool,
}

#[tauri::command]
pub fn painel_agora(est: State<'_, Arc<Estado>>) -> Painel {
    painel(&est)
}

#[tauri::command]
pub fn obter_config(est: State<'_, Arc<Estado>>) -> ConfigCompleta {
    let n = est.nucleo.lock().unwrap();
    ConfigCompleta {
        agenda: n.agenda.clone(),
        ajustes: n.ajustes.publico(),
        versao: env!("CARGO_PKG_VERSION").into(),
        dados_dir: est.dados_dir.to_string_lossy().to_string(),
        portatil: crate::modo_portatil(),
    }
}

/// Troca de modo. Se houver reprodução e `confirmar` = false, devolve erro "CONFIRMAR".
#[tauri::command]
pub fn definir_modo(est: State<'_, Arc<Estado>>, modo: Modo, confirmar: bool) -> R<()> {
    if !confirmar && tocando(&est) {
        return Err("CONFIRMAR".into());
    }
    let mut n = est.nucleo.lock().unwrap();
    if n.modo == modo {
        return Ok(());
    }
    let db = est.db.lock().unwrap();
    let _ = est.audio.tx.send(Cmd::Parar { fade_ms: 1500 });
    n.origem = Origem::Nenhuma;
    n.id_atual = 0;
    n.modo = modo;
    match modo {
        Modo::Livre => {
            n.agendador.entrar_livre();
            n.registrar(&db, "manual", "Info", None, "Modo Rádio Livre ativado: programação automática suspensa");
        }
        Modo::Escolar => {
            n.registrar(&db, "manual", "Info", None, "Modo Rádio Escolar ativado");
            let agenda = n.agenda.clone();
            let silenciado = n.silenciado;
            let acoes = n.agendador.reavaliar(&agenda, agora_momento(), Modo::Escolar, silenciado, "Retorno ao Rádio Escolar: eventos vencidos não serão tocados");
            n.aplicar(&db, &est.audio, acoes);
        }
    }
    persistir(&db, &n);
    Ok(())
}

#[tauri::command]
pub fn silenciar_agora(est: State<'_, Arc<Estado>>) {
    // Prioridade máxima: corta o áudio antes de qualquer outra coisa.
    let _ = est.audio.tx.send(Cmd::Silenciar);
    let mut n = est.nucleo.lock().unwrap();
    n.silenciado = true;
    n.agendador.silenciar();
    n.origem = Origem::Nenhuma;
    n.id_atual = 0;
    let db = est.db.lock().unwrap();
    n.registrar(&db, "manual", "Aviso", None, "SILENCIAR AGORA acionado pelo operador");
}

#[tauri::command]
pub fn retomar(est: State<'_, Arc<Estado>>) {
    let mut n = est.nucleo.lock().unwrap();
    if !n.silenciado {
        return;
    }
    n.silenciado = false;
    let db = est.db.lock().unwrap();
    n.registrar(&db, "manual", "Info", None, "Rádio retomada pelo operador");
    if n.modo == Modo::Escolar {
        let agenda = n.agenda.clone();
        let acoes = n.agendador.reavaliar(&agenda, agora_momento(), Modo::Escolar, false, "Retomada: agenda reavaliada (sem eventos vencidos)");
        n.aplicar(&db, &est.audio, acoes);
    }
}

// ----------------------------------------------------------------- player

#[tauri::command]
pub fn tocar_pausar(est: State<'_, Arc<Estado>>) -> R<()> {
    let st = est.audio.status.lock().map(|s| s.clone()).unwrap_or_default();
    let mut n = est.nucleo.lock().unwrap();
    if n.silenciado {
        return Err("A rádio está silenciada. Toque em \"Retomar\" primeiro.".into());
    }
    if st.pausado && st.item.is_some() {
        let _ = est.audio.tx.send(Cmd::Continuar { fade_ms: 800 });
        return Ok(());
    }
    if st.tocando {
        let _ = est.audio.tx.send(Cmd::Pausar { fade_ms: 800 });
        return Ok(());
    }
    if n.modo == Modo::Livre {
        let db = est.db.lock().unwrap();
        let i = n.indice_fila.unwrap_or(0);
        return n.tocar_fila(&db, &est.audio, i);
    }
    Err("Nada programado tocando agora. Para tocar manualmente, use o Rádio Livre.".into())
}

#[tauri::command]
pub fn proxima(est: State<'_, Arc<Estado>>) -> R<()> {
    let mut n = est.nucleo.lock().unwrap();
    let db = est.db.lock().unwrap();
    match n.origem.clone() {
        Origem::Livre { uid } => {
            let atual = n.fila.iter().position(|x| x.uid == uid).unwrap_or(0);
            n.rng.next_u64();
            match n.proximo_da_fila(atual, true) {
                Some(p) => n.tocar_fila(&db, &est.audio, p),
                None => Ok(()),
            }
        }
        Origem::Evento { evento_id, .. } => {
            let ev = n.agenda.evento(&evento_id).cloned();
            match ev {
                Some(ev) if ev.tipo.eh_janela() => {
                    n.registrar(&db, "manual", "Info", Some(&ev.id), "Faixa pulada pelo operador");
                    n.iniciar_evento(&db, &est.audio, &ev, false, true, true);
                    Ok(())
                }
                _ => {
                    let _ = est.audio.tx.send(Cmd::Parar { fade_ms: 1200 });
                    n.agendador.faixa_terminou(agora_momento());
                    n.origem = Origem::Nenhuma;
                    n.id_atual = 0;
                    Ok(())
                }
            }
        }
        Origem::Nenhuma => {
            if n.modo == Modo::Livre && !n.fila.is_empty() {
                let i = n.indice_fila.map(|i| (i + 1) % n.fila.len()).unwrap_or(0);
                n.tocar_fila(&db, &est.audio, i)
            } else {
                Ok(())
            }
        }
    }
}

#[tauri::command]
pub fn anterior(est: State<'_, Arc<Estado>>) -> R<()> {
    let pos = est.audio.status.lock().map(|s| s.posicao_ms).unwrap_or(0);
    let mut n = est.nucleo.lock().unwrap();
    if let Origem::Livre { uid } = n.origem.clone() {
        if pos < 4000 {
            let db = est.db.lock().unwrap();
            let atual = n.fila.iter().position(|x| x.uid == uid).unwrap_or(0);
            let i = if atual == 0 { n.fila.len().saturating_sub(1) } else { atual - 1 };
            return n.tocar_fila(&db, &est.audio, i);
        }
    }
    let _ = est.audio.tx.send(Cmd::Buscar { ms: 0 });
    Ok(())
}

#[tauri::command]
pub fn definir_volume(est: State<'_, Arc<Estado>>, volume: u8) -> u8 {
    let mut n = est.nucleo.lock().unwrap();
    let v = volume.min(n.ajustes.volume_maximo);
    n.ajustes.volume_geral = v;
    let _ = est.audio.tx.send(Cmd::Volume { geral: n.volume_master() });
    if let Ok(db) = est.db.lock() {
        if let Ok(s) = serde_json::to_string(&n.ajustes) {
            let _ = db.gravar_config("ajustes", &s);
        }
    }
    v
}

#[tauri::command]
pub fn buscar(est: State<'_, Arc<Estado>>, ms: u64) {
    let _ = est.audio.tx.send(Cmd::Buscar { ms });
}

// ----------------------------------------------------------------- fila (Rádio Livre)

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemFilaView {
    pub uid: u64,
    pub faixa_id: String,
    pub faixa: Option<Faixa>,
}

#[tauri::command]
pub fn fila_obter(est: State<'_, Arc<Estado>>) -> Vec<ItemFilaView> {
    let n = est.nucleo.lock().unwrap();
    let db = est.db.lock().unwrap();
    n.fila.iter().map(|i| ItemFilaView { uid: i.uid, faixa_id: i.faixa_id.clone(), faixa: db.faixa(&i.faixa_id) }).collect()
}

fn fila_mudou(app: &AppHandle, est: &Estado, n: &Nucleo) {
    if let Ok(db) = est.db.lock() {
        if let Ok(s) = serde_json::to_string(&n.fila) {
            let _ = db.gravar_config("fila", &s);
        }
    }
    let _ = app.emit("fila", n.fila.len());
}

#[tauri::command]
pub fn fila_adicionar(app: AppHandle, est: State<'_, Arc<Estado>>, ids: Vec<String>, tocar: bool) -> R<()> {
    let mut n = est.nucleo.lock().unwrap();
    let inicio = n.fila.len();
    for id in ids {
        n.uid_seq += 1;
        let uid = n.uid_seq;
        n.fila.push(ItemFila { uid, faixa_id: id });
    }
    fila_mudou(&app, &est, &n);
    if tocar && n.modo == Modo::Livre && inicio < n.fila.len() {
        let db = est.db.lock().unwrap();
        return n.tocar_fila(&db, &est.audio, inicio);
    }
    Ok(())
}

#[tauri::command]
pub fn fila_adicionar_playlist(app: AppHandle, est: State<'_, Arc<Estado>>, id: String, substituir: bool) -> R<usize> {
    let ids = est.db.lock().unwrap().faixas_playlist(&id);
    let mut n = est.nucleo.lock().unwrap();
    if substituir {
        n.fila.clear();
        n.indice_fila = None;
    }
    for f in &ids {
        n.uid_seq += 1;
        let uid = n.uid_seq;
        n.fila.push(ItemFila { uid, faixa_id: f.clone() });
    }
    fila_mudou(&app, &est, &n);
    Ok(ids.len())
}

#[tauri::command]
pub fn fila_reordenar(app: AppHandle, est: State<'_, Arc<Estado>>, uids: Vec<u64>) {
    let mut n = est.nucleo.lock().unwrap();
    let mut nova = Vec::new();
    for u in &uids {
        if let Some(i) = n.fila.iter().find(|x| x.uid == *u) {
            nova.push(i.clone());
        }
    }
    // Itens não citados ficam no fim (nunca se perde nada).
    for i in n.fila.iter() {
        if !uids.contains(&i.uid) {
            nova.push(i.clone());
        }
    }
    n.fila = nova;
    if let Origem::Livre { uid } = n.origem {
        n.indice_fila = n.fila.iter().position(|x| x.uid == uid);
    }
    fila_mudou(&app, &est, &n);
}

#[tauri::command]
pub fn fila_remover(app: AppHandle, est: State<'_, Arc<Estado>>, uid: u64) {
    let mut n = est.nucleo.lock().unwrap();
    n.fila.retain(|x| x.uid != uid);
    if let Origem::Livre { uid: u } = n.origem {
        n.indice_fila = n.fila.iter().position(|x| x.uid == u);
    }
    fila_mudou(&app, &est, &n);
}

#[tauri::command]
pub fn fila_limpar(app: AppHandle, est: State<'_, Arc<Estado>>) {
    let mut n = est.nucleo.lock().unwrap();
    n.fila.clear();
    n.indice_fila = None;
    fila_mudou(&app, &est, &n);
}

#[tauri::command]
pub fn fila_tocar(est: State<'_, Arc<Estado>>, uid: u64) -> R<()> {
    let mut n = est.nucleo.lock().unwrap();
    if n.modo != Modo::Livre {
        return Err("Ative o Rádio Livre para tocar itens da fila.".into());
    }
    let i = n.fila.iter().position(|x| x.uid == uid).ok_or("Item não está mais na fila")?;
    let db = est.db.lock().unwrap();
    n.tocar_fila(&db, &est.audio, i)
}

#[tauri::command]
pub fn definir_repetir(est: State<'_, Arc<Estado>>, repetir: Repetir) {
    est.nucleo.lock().unwrap().repetir = repetir;
}

#[tauri::command]
pub fn definir_aleatorio(est: State<'_, Arc<Estado>>, aleatorio: bool) {
    est.nucleo.lock().unwrap().aleatorio = aleatorio;
}

// ----------------------------------------------------------------- biblioteca

#[tauri::command]
pub fn listar_pastas(est: State<'_, Arc<Estado>>) -> R<Vec<Pasta>> {
    est.db.lock().unwrap().pastas()
}

#[tauri::command]
pub fn adicionar_pasta(app: AppHandle, est: State<'_, Arc<Estado>>, caminho: String, categoria: String, subpastas: bool) -> R<i64> {
    if !std::path::Path::new(&caminho).is_dir() {
        return Err("Pasta não encontrada ou sem permissão de leitura".into());
    }
    let cat = if ["musica", "vinheta", "recado"].contains(&categoria.as_str()) { categoria.clone() } else { "musica".to_string() };
    let id = {
        let db = est.db.lock().unwrap();
        let id = db.adicionar_pasta(&caminho, &cat, subpastas)?;
        db.registrar("biblioteca", "Info", None, &format!("Pasta adicionada ({}): {}", cat, caminho));
        id
    };
    indexar_em_segundo_plano(app, est.inner().clone());
    Ok(id)
}

#[tauri::command]
pub fn remover_pasta(est: State<'_, Arc<Estado>>, id: i64) -> R<()> {
    exigir_pin(&est)?;
    let db = est.db.lock().unwrap();
    db.remover_pasta(id)?;
    db.registrar("biblioteca", "Info", None, &format!("Pasta removida da biblioteca (arquivos originais intactos), id {}", id));
    Ok(())
}

#[tauri::command]
pub fn reindexar(app: AppHandle, est: State<'_, Arc<Estado>>) {
    indexar_em_segundo_plano(app, est.inner().clone());
}

#[tauri::command]
pub fn listar_faixas(est: State<'_, Arc<Estado>>, texto: String, categoria: String, status: String, limite: Option<i64>) -> R<Vec<Faixa>> {
    est.db.lock().unwrap().faixas(&texto, &categoria, &status, limite.unwrap_or(2000))
}

#[tauri::command]
pub fn definir_categoria_faixa(est: State<'_, Arc<Estado>>, id: String, categoria: String) -> R<()> {
    est.db.lock().unwrap().definir_categoria(&id, &categoria)
}

#[tauri::command]
pub fn definir_volume_faixa(est: State<'_, Arc<Estado>>, id: String, volume: u8) -> R<()> {
    est.db.lock().unwrap().definir_volume_faixa(&id, volume)
}

#[tauri::command]
pub fn resumo_biblioteca(est: State<'_, Arc<Estado>>) -> R<Vec<(String, String, i64)>> {
    est.db.lock().unwrap().contagem()
}

#[tauri::command]
pub fn listar_playlists(est: State<'_, Arc<Estado>>) -> R<Vec<Playlist>> {
    est.db.lock().unwrap().playlists()
}

#[tauri::command]
pub fn salvar_playlist(est: State<'_, Arc<Estado>>, playlist: Playlist) -> R<Playlist> {
    let mut p = playlist;
    if p.id.trim().is_empty() {
        p.id = format!("pl-{}", chrono::Local::now().format("%Y%m%d%H%M%S%3f"));
    }
    if p.nome.trim().is_empty() {
        return Err("Dê um nome à playlist".into());
    }
    est.db.lock().unwrap().salvar_playlist(&p)?;
    Ok(p)
}

#[tauri::command]
pub fn excluir_playlist(est: State<'_, Arc<Estado>>, id: String) -> R<()> {
    est.db.lock().unwrap().excluir_playlist(&id)
}

// ----------------------------------------------------------------- recados

/// Recado imediato: pausa (ou abaixa) a música suavemente, toca beep/vinheta → recado, e retoma.
#[tauri::command]
pub fn tocar_recado(est: State<'_, Arc<Estado>>, recado_id: Option<String>, vinheta_id: Option<String>, beep: bool) -> R<()> {
    let mut n = est.nucleo.lock().unwrap();
    if n.silenciado {
        return Err("A rádio está silenciada. Toque em \"Retomar\" antes de enviar um recado.".into());
    }
    let db = est.db.lock().unwrap();
    let rec = match &recado_id {
        Some(id) => Some(db.faixa(id).filter(|f| f.status == "ok").ok_or("Arquivo do recado ausente ou inválido")?),
        None => None,
    };
    let vin = vinheta_id.as_ref().and_then(|id| db.faixa(id)).filter(|f| f.status == "ok");
    if rec.is_none() && vin.is_none() && !beep {
        return Err("Escolha um recado ou vinheta".into());
    }
    n.anunciar(&db, &est.audio, rec.as_ref(), vin.as_ref(), beep && vin.is_none());
    let nome = rec.as_ref().map(|r| r.nome.clone()).or(vin.as_ref().map(|v| v.nome.clone())).unwrap_or_else(|| "beep".into());
    n.registrar(&db, "manual", "Info", None, &format!("Recado imediato: {}", nome));
    Ok(())
}

// ----------------------------------------------------------------- agenda

#[tauri::command]
pub fn validar_agenda(agenda: Agenda) -> Vec<Aviso> {
    validar(&agenda)
}

#[tauri::command]
pub fn salvar_agenda(est: State<'_, Arc<Estado>>, agenda: Agenda) -> R<Vec<Aviso>> {
    exigir_pin(&est)?;
    let av = validar(&agenda);
    if tem_erro(&av) {
        return Err("A programação tem erros. Corrija os itens em vermelho antes de salvar.".into());
    }
    let mut n = est.nucleo.lock().unwrap();
    n.agenda = agenda;
    let db = est.db.lock().unwrap();
    persistir(&db, &n);
    let msg = format!("Programação salva ({} eventos, {} exceções)", n.agenda.eventos.len(), n.agenda.excecoes.len());
    n.registrar(&db, "config", "Info", None, &msg);
    Ok(av)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiaView {
    pub perfil: PerfilDia,
    pub itens: Vec<ItemAgenda>,
}

#[tauri::command]
pub fn agenda_dia(est: State<'_, Arc<Estado>>, data: String, agenda: Option<Agenda>) -> R<DiaView> {
    let d = Data::parse(&data).ok_or("Data inválida")?;
    let a = match agenda {
        Some(a) => a,
        None => est.nucleo.lock().unwrap().agenda.clone(),
    };
    Ok(DiaView { perfil: a.perfil_do_dia(d), itens: agenda_do_dia(&a, d, agora_momento()) })
}

#[tauri::command]
pub fn agenda_inicial() -> Agenda {
    agenda_padrao()
}

// ----------------------------------------------------------------- ajustes / PIN

#[tauri::command]
pub fn salvar_ajustes(est: State<'_, Arc<Estado>>, ajustes: Ajustes) -> R<AjustesPublicos> {
    exigir_pin(&est)?;
    let mut n = est.nucleo.lock().unwrap();
    let mut a = ajustes;
    a.pin_hash = n.ajustes.pin_hash.clone();
    a.pin_sal = n.ajustes.pin_sal.clone();
    a.volume_maximo = a.volume_maximo.clamp(10, 100);
    a.volume_geral = a.volume_geral.min(a.volume_maximo);
    a.escala = a.escala.clamp(0.8, 1.6);
    let mudou_disp = a.dispositivo_saida != n.ajustes.dispositivo_saida;
    n.ajustes = a;
    let _ = est.audio.tx.send(Cmd::Volume { geral: n.volume_master() });
    if mudou_disp {
        let _ = est.audio.tx.send(Cmd::SelecionarDispositivo { nome: n.ajustes.dispositivo_saida.clone() });
    }
    let db = est.db.lock().unwrap();
    persistir(&db, &n);
    n.registrar(&db, "config", "Info", None, "Configurações salvas");
    Ok(n.ajustes.publico())
}

#[tauri::command]
pub fn definir_pin(est: State<'_, Arc<Estado>>, atual: String, novo: String) -> R<()> {
    let mut n = est.nucleo.lock().unwrap();
    if !n.ajustes.pin_hash.is_empty() && hash_pin(&n.ajustes.pin_sal, &atual) != n.ajustes.pin_hash {
        return Err("PIN atual incorreto".into());
    }
    let novo = novo.trim().to_string();
    if novo.is_empty() {
        n.ajustes.pin_hash.clear();
        n.ajustes.pin_sal.clear();
    } else {
        if novo.len() < 4 || !novo.chars().all(|c| c.is_ascii_digit()) {
            return Err("O PIN deve ter pelo menos 4 números".into());
        }
        let sal = format!("{:x}", chrono::Local::now().timestamp_nanos_opt().unwrap_or(0) ^ 0x5bd1e995);
        n.ajustes.pin_hash = hash_pin(&sal, &novo);
        n.ajustes.pin_sal = sal;
    }
    let db = est.db.lock().unwrap();
    persistir(&db, &n);
    n.registrar(&db, "config", "Info", None, if novo.is_empty() { "PIN removido" } else { "PIN definido/alterado" });
    Ok(())
}

#[tauri::command]
pub fn desbloquear(est: State<'_, Arc<Estado>>, pin: String) -> bool {
    let (ok, min) = {
        let n = est.nucleo.lock().unwrap();
        (n.ajustes.pin_hash.is_empty() || hash_pin(&n.ajustes.pin_sal, &pin) == n.ajustes.pin_hash, n.ajustes.pin_validade_min.max(1))
    };
    if ok {
        *est.desbloqueado_ate.lock().unwrap() = Some(Instant::now() + Duration::from_secs(min as u64 * 60));
    } else {
        est.db.lock().unwrap().registrar("config", "Aviso", None, "Tentativa de PIN incorreto");
    }
    ok
}

#[tauri::command]
pub fn bloquear(est: State<'_, Arc<Estado>>) {
    *est.desbloqueado_ate.lock().unwrap() = None;
}

#[tauri::command]
pub fn esta_desbloqueado(est: State<'_, Arc<Estado>>) -> bool {
    exigir_pin(&est).is_ok()
}

#[tauri::command]
pub fn definir_autostart(app: AppHandle, est: State<'_, Arc<Estado>>, ativo: bool) -> R<bool> {
    use tauri_plugin_autostart::ManagerExt;
    exigir_pin(&est)?;
    let al = app.autolaunch();
    let r = if ativo { al.enable() } else { al.disable() };
    r.map_err(|e| format!("Não foi possível alterar a inicialização com o Windows: {}", e))?;
    let real = al.is_enabled().unwrap_or(ativo);
    let mut n = est.nucleo.lock().unwrap();
    n.ajustes.iniciar_com_windows = real;
    let db = est.db.lock().unwrap();
    persistir(&db, &n);
    Ok(real)
}

#[tauri::command]
pub fn concluir_assistente(est: State<'_, Arc<Estado>>) {
    let mut n = est.nucleo.lock().unwrap();
    n.ajustes.assistente_concluido = true;
    let db = est.db.lock().unwrap();
    persistir(&db, &n);
    n.registrar(&db, "config", "Info", None, "Assistente de primeira configuração concluído");
}

fn data_url(caminho: &str) -> Option<String> {
    use base64::Engine;
    let bytes = std::fs::read(caminho).ok()?;
    let ext = std::path::Path::new(caminho).extension()?.to_str()?.to_lowercase();
    let mime = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        _ => return None,
    };
    Some(format!("data:{};base64,{}", mime, base64::engine::general_purpose::STANDARD.encode(bytes)))
}

#[tauri::command]
pub fn obter_logo(est: State<'_, Arc<Estado>>) -> Option<String> {
    let c = est.nucleo.lock().unwrap().ajustes.logo_arquivo.clone()?;
    data_url(&c)
}

/// Substitui a logo por um arquivo local (copiado para a pasta de dados). `None` volta à logo padrão.
#[tauri::command]
pub fn definir_logo(est: State<'_, Arc<Estado>>, caminho: Option<String>) -> R<Option<String>> {
    exigir_pin(&est)?;
    let destino = match caminho {
        Some(c) => {
            let ext = std::path::Path::new(&c).extension().and_then(|x| x.to_str()).unwrap_or("png").to_lowercase();
            if !["png", "jpg", "jpeg", "svg", "webp"].contains(&ext.as_str()) {
                return Err("Use uma imagem PNG, JPG, SVG ou WEBP".into());
            }
            let d = est.dados_dir.join(format!("logo.{}", ext));
            std::fs::copy(&c, &d).map_err(|e| format!("Não foi possível copiar a logo: {}", e))?;
            Some(d.to_string_lossy().to_string())
        }
        None => None,
    };
    let mut n = est.nucleo.lock().unwrap();
    n.ajustes.logo_arquivo = destino.clone();
    let db = est.db.lock().unwrap();
    persistir(&db, &n);
    Ok(destino.and_then(|d| data_url(&d)))
}

/// Capa embutida no arquivo (ID3/Vorbis/FLAC), se houver. Opcional: sem capa, a interface mostra um ícone.
#[tauri::command]
pub async fn capa(est: State<'_, Arc<Estado>>, faixa_id: String) -> R<Option<String>> {
    use base64::Engine;
    use lofty::prelude::*;
    let caminho = match est.db.lock().unwrap().faixa(&faixa_id) {
        Some(f) => f.caminho,
        None => return Ok(None),
    };
    let Ok(t) = lofty::read_from_path(&caminho) else { return Ok(None) };
    let Some(tag) = t.primary_tag().or_else(|| t.first_tag()) else { return Ok(None) };
    let Some(pic) = tag.pictures().first() else { return Ok(None) };
    if pic.data().len() > 3_000_000 {
        return Ok(None);
    }
    let mime = pic.mime_type().map(|m| m.as_str().to_string()).unwrap_or_else(|| "image/jpeg".into());
    Ok(Some(format!("data:{};base64,{}", mime, base64::engine::general_purpose::STANDARD.encode(pic.data()))))
}

// ----------------------------------------------------------------- áudio / diagnóstico

/// Assíncrono: a enumeração WASAPI/COM roda fora da thread principal da janela.
#[tauri::command]
pub async fn dispositivos() -> InfoDispositivos {
    audio::listar_dispositivos()
}

#[tauri::command]
pub fn selecionar_dispositivo(est: State<'_, Arc<Estado>>, nome: Option<String>) {
    let mut n = est.nucleo.lock().unwrap();
    n.ajustes.dispositivo_saida = nome.clone();
    let _ = est.audio.tx.send(Cmd::SelecionarDispositivo { nome: nome.clone() });
    let db = est.db.lock().unwrap();
    persistir(&db, &n);
    n.registrar(&db, "audio", "Info", None, &format!("Saída de áudio selecionada: {}", nome.unwrap_or_else(|| "padrão do Windows".into())));
}

/// Operador confirma a saída após queda (pode ser a mesma, reconectada, ou outra escolhida conscientemente).
#[tauri::command]
pub fn confirmar_dispositivo(est: State<'_, Arc<Estado>>, nome: Option<String>) {
    let mut n = est.nucleo.lock().unwrap();
    n.ajustes.dispositivo_saida = nome.clone();
    let _ = est.audio.tx.send(Cmd::Reabrir { nome });
    let db = est.db.lock().unwrap();
    persistir(&db, &n);
}

#[tauri::command]
pub fn tocar_teste(est: State<'_, Arc<Estado>>, volume: u8) {
    let n = est.nucleo.lock().unwrap();
    let _ = est.audio.tx.send(Cmd::Volume { geral: n.volume_master() });
    let g = (volume.min(n.ajustes.limite_vinheta) as f32) / 100.0;
    let _ = est.audio.tx.send(Cmd::Teste { ganho: g });
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostico {
    pub dispositivos: InfoDispositivos,
    pub audio: audio::StatusAudio,
    pub dica_hands_free: bool,
    pub parece_bluetooth: bool,
    pub versao: String,
    pub dados_dir: String,
    pub banco_bytes: u64,
    pub ultimo_tick: Option<String>,
    pub biblioteca: Vec<(String, String, i64)>,
    pub sistema: crate::sistema::StatusSistema,
    pub portatil: bool,
}

#[tauri::command]
pub async fn diagnostico(est: State<'_, Arc<Estado>>) -> R<Diagnostico> {
    let dispositivos = audio::listar_dispositivos();
    let audio_st = est.audio.status.lock().map(|s| s.clone()).unwrap_or_default();
    let n = est.nucleo.lock().unwrap();
    let nome = audio_st.dispositivo.to_lowercase();
    Ok(Diagnostico {
        dispositivos,
        dica_hands_free: nome.contains("hands-free") || nome.contains("headset") || nome.contains("viva-voz"),
        parece_bluetooth: nome.contains("bluetooth") || nome.contains("bt ") || nome.contains("hands-free") || nome.contains("stereo"),
        audio: audio_st,
        versao: env!("CARGO_PKG_VERSION").into(),
        dados_dir: est.dados_dir.to_string_lossy().to_string(),
        banco_bytes: std::fs::metadata(&est.db_path).map(|m| m.len()).unwrap_or(0),
        ultimo_tick: n.ultimo_tick.clone(),
        biblioteca: est.db.lock().map(|d| d.contagem().unwrap_or_default()).unwrap_or_default(),
        sistema: n.sistema.clone(),
        portatil: crate::modo_portatil(),
    })
}

#[tauri::command]
pub fn limpar_alertas(est: State<'_, Arc<Estado>>) {
    est.nucleo.lock().unwrap().alertas.clear();
}

// ----------------------------------------------------------------- microfone

#[tauri::command]
pub fn mic_abrir(est: State<'_, Arc<Estado>>, nome: Option<String>) {
    let mut n = est.nucleo.lock().unwrap();
    n.ajustes.mic_dispositivo = nome.clone();
    let (g, p) = (n.ajustes.mic_ganho as f32 / 100.0, n.ajustes.mic_portao as f32 / 200.0);
    let _ = est.audio.tx.send(Cmd::MicAjuste { ganho: g, portao: p });
    let _ = est.audio.tx.send(Cmd::MicAbrir { nome });
}

#[tauri::command]
pub fn mic_fechar(est: State<'_, Arc<Estado>>) {
    let _ = est.audio.tx.send(Cmd::MicFechar);
}

#[tauri::command]
pub fn mic_falar(est: State<'_, Arc<Estado>>, falar: bool) -> R<()> {
    let mut n = est.nucleo.lock().unwrap();
    if falar && n.silenciado {
        return Err("A rádio está silenciada. Toque em \"Retomar\" antes de usar o microfone.".into());
    }
    let pausar = n.ajustes.duck_modo == crate::config::ModoDuck::Pausar;
    let _ = est.audio.tx.send(Cmd::MicFalar { falar, pausar_musica: pausar });
    let db = est.db.lock().unwrap();
    n.registrar(&db, "manual", "Info", None, if falar { "Microfone ao vivo LIGADO" } else { "Microfone ao vivo desligado" });
    Ok(())
}

#[tauri::command]
pub fn mic_ajuste(est: State<'_, Arc<Estado>>, ganho: u8, portao: u8) {
    let mut n = est.nucleo.lock().unwrap();
    n.ajustes.mic_ganho = ganho.min(100);
    n.ajustes.mic_portao = portao.min(100);
    let _ = est.audio.tx.send(Cmd::MicAjuste { ganho: ganho.min(100) as f32 / 100.0, portao: portao.min(100) as f32 / 200.0 });
}

// ----------------------------------------------------------------- histórico / backup / demo

#[tauri::command]
pub fn listar_historico(est: State<'_, Arc<Estado>>, texto: String, nivel: String, data: String, limite: Option<i64>) -> R<Vec<Registro>> {
    est.db.lock().unwrap().historico(&texto, &nivel, &data, limite.unwrap_or(500))
}

#[tauri::command]
pub fn exportar_historico(est: State<'_, Arc<Estado>>, caminho: String) -> R<usize> {
    let regs = est.db.lock().unwrap().historico("", "", "", 1_000_000)?;
    let mut s = String::from("\u{feff}quando;origem;nivel;evento;mensagem\r\n");
    for r in regs.iter().rev() {
        let m = r.mensagem.replace('"', "\"\"");
        s.push_str(&format!("{};{};{};{};\"{}\"\r\n", r.quando, r.origem, r.nivel, r.evento_id.clone().unwrap_or_default(), m));
    }
    std::fs::write(&caminho, s).map_err(|e| e.to_string())?;
    Ok(regs.len())
}

#[tauri::command]
pub fn exportar_config(est: State<'_, Arc<Estado>>, caminho: String) -> R<()> {
    let n = est.nucleo.lock().unwrap();
    let db = est.db.lock().unwrap();
    let mut aj = n.ajustes.clone();
    aj.pin_hash.clear();
    aj.pin_sal.clear();
    let b = Backup {
        formato: "radio-escolar-mjs-backup".into(),
        versao_app: env!("CARGO_PKG_VERSION").into(),
        exportado_em: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        agenda: n.agenda.clone(),
        ajustes: aj,
        pastas: db.pastas()?,
        playlists: db.playlists()?,
    };
    let s = serde_json::to_string_pretty(&b).map_err(|e| e.to_string())?;
    std::fs::write(&caminho, s).map_err(|e| e.to_string())?;
    db.registrar("config", "Info", None, &format!("Backup exportado: {}", caminho));
    Ok(())
}

#[tauri::command]
pub fn importar_config(app: AppHandle, est: State<'_, Arc<Estado>>, caminho: String) -> R<Vec<Aviso>> {
    exigir_pin(&est)?;
    let txt = std::fs::read_to_string(&caminho).map_err(|e| e.to_string())?;
    let b: Backup = serde_json::from_str(&txt).map_err(|e| format!("Arquivo de backup inválido: {}", e))?;
    if b.formato != "radio-escolar-mjs-backup" {
        return Err("Este arquivo não é um backup da Rádio Escolar".into());
    }
    let av = validar(&b.agenda);
    if tem_erro(&av) {
        return Err("A programação do backup tem erros e não foi importada".into());
    }
    {
        let mut n = est.nucleo.lock().unwrap();
        let mut db = est.db.lock().unwrap();
        let (h, s) = (n.ajustes.pin_hash.clone(), n.ajustes.pin_sal.clone());
        n.agenda = b.agenda;
        n.ajustes = Ajustes { pin_hash: h, pin_sal: s, ..b.ajustes };
        for p in &b.pastas {
            let _ = db.adicionar_pasta(&p.caminho, &p.categoria, p.subpastas);
        }
        for p in &b.playlists {
            let _ = db.salvar_playlist(p);
        }
        persistir(&db, &n);
        n.registrar(&db, "config", "Info", None, &format!("Backup importado: {}", caminho));
        let _ = est.audio.tx.send(Cmd::Volume { geral: n.volume_master() });
    }
    indexar_em_segundo_plano(app, est.inner().clone());
    Ok(av)
}

#[tauri::command]
pub async fn criar_demo(app: AppHandle, est: State<'_, Arc<Estado>>) -> R<String> {
    let (m, v, r) = crate::demo::criar(&est.dados_dir).map_err(|e| format!("Falha ao criar áudios de demonstração: {}", e))?;
    {
        let db = est.db.lock().unwrap();
        db.adicionar_pasta(&m, "musica", true)?;
        db.adicionar_pasta(&v, "vinheta", true)?;
        db.adicionar_pasta(&r, "recado", true)?;
        db.registrar("biblioteca", "Info", None, "Áudios de DEMONSTRAÇÃO (sintetizados) criados e adicionados");
    }
    indexar_em_segundo_plano(app, est.inner().clone());
    Ok(est.dados_dir.join("DEMO - audios sintetizados").to_string_lossy().to_string())
}
