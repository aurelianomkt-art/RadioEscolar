//! Núcleo residente: liga o agendador (radio-core), a biblioteca e o motor de áudio.
//! Roda numa thread do processo nativo, independente da janela (funciona minimizado/na bandeja).

use crate::audio::{Audio, Cmd, EventoAudio, ItemAudio, StatusAudio};
use crate::config::{Ajustes, ModoDuck};
use crate::db::{Db, Faixa};
use crate::sistema::StatusSistema;
use radio_core::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Repetir {
    Nao,
    Fila,
    Faixa,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemFila {
    pub uid: u64,
    pub faixa_id: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "tipo", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Origem {
    Nenhuma,
    Evento { evento_id: String, nome: String, tipo_evento: TipoEvento, fim: Option<String>, faixas_tocadas: u32 },
    Livre { uid: u64 },
}

pub struct Nucleo {
    pub agenda: Agenda,
    pub ajustes: Ajustes,
    pub agendador: Agendador,
    pub modo: Modo,
    pub silenciado: bool,
    pub origem: Origem,
    pub id_seq: u64,
    pub id_atual: u64,
    pub rng: Rng,
    pub cursores: HashMap<String, usize>,
    pub fila: Vec<ItemFila>,
    pub indice_fila: Option<usize>,
    pub repetir: Repetir,
    pub aleatorio: bool,
    pub falhas_seguidas: u32,
    pub alertas: Vec<String>,
    pub ultimo_tick: Option<String>,
    pub ultima_acao: Option<String>,
    pub sistema: StatusSistema,
    pub alerta_bateria_dado: bool,
    pub uid_seq: u64,
}

pub struct Estado {
    pub nucleo: Mutex<Nucleo>,
    pub db: Mutex<Db>,
    pub db_path: PathBuf,
    pub dados_dir: PathBuf,
    pub audio: Audio,
    pub desbloqueado_ate: Mutex<Option<Instant>>,
    pub indexando: std::sync::atomic::AtomicBool,
}

pub fn agora_momento() -> Momento {
    use chrono::{Datelike, Timelike};
    let n = chrono::Local::now();
    let d = Data::new(n.year(), n.month() as u8, n.day() as u8).unwrap_or(Data { ano: 2000, mes: 1, dia: 1 });
    Momento::new(d, n.hour() * 3600 + n.minute() * 60 + n.second())
}

impl Nucleo {
    pub fn volume_master(&self) -> f32 {
        (self.ajustes.volume_geral.min(self.ajustes.volume_maximo) as f32) / 100.0
    }

    fn novo_id(&mut self) -> u64 {
        self.id_seq += 1;
        self.id_seq
    }

    pub fn registrar(&mut self, db: &Db, origem: &str, nivel: &str, evento: Option<&str>, msg: &str) {
        db.registrar(origem, nivel, evento, msg);
        self.ultima_acao = Some(format!("{} — {}", chrono::Local::now().format("%H:%M:%S"), msg));
    }

    pub fn alerta(&mut self, msg: String) {
        if !self.alertas.contains(&msg) {
            self.alertas.push(msg);
        }
        if self.alertas.len() > 8 {
            self.alertas.remove(0);
        }
    }

    /// Item de áudio para uma faixa do banco (ganho = volume do evento × volume individual, com tetos).
    fn item(&self, f: &Faixa, volume_evento: u8) -> ItemAudio {
        let mut g = (volume_evento as f32 / 100.0) * (f.volume as f32 / 100.0);
        if f.categoria == "vinheta" {
            g = g.min(self.ajustes.limite_vinheta as f32 / 100.0);
        }
        ItemAudio {
            caminho: Some(f.caminho.clone()),
            faixa_id: Some(f.id.clone()),
            titulo: f.titulo.clone().unwrap_or_else(|| f.nome.clone()),
            artista: f.artista.clone(),
            duracao_ms: f.duracao_ms,
            ganho: g.clamp(0.0, 1.0),
        }
    }

    fn beep(&self, db: &Db) -> ItemAudio {
        let lim = self.ajustes.limite_vinheta.min(self.ajustes.volumes_tipo.vinheta) as f32 / 100.0;
        if let Some(id) = &self.ajustes.beep_faixa_id {
            if let Some(f) = db.faixa(id) {
                if f.status == "ok" {
                    let mut it = self.item(&f, self.ajustes.volumes_tipo.vinheta);
                    it.ganho = it.ganho.min(lim);
                    return it;
                }
            }
        }
        ItemAudio::beep(lim)
    }

    fn beep_atraso(&self) -> ItemAudio {
        let lim = self.ajustes.limite_vinheta.min(self.ajustes.volumes_tipo.vinheta) as f32 / 100.0;
        ItemAudio::beep_atraso(lim)
    }

    /// Escolhe a música de um evento conforme a fonte configurada.
    fn escolher_musica(&mut self, db: &Db, ev: &Evento) -> Option<Faixa> {
        let candidatas: Vec<String> = if ev.selecao == ModoSelecao::FaixaFixa {
            ev.faixa_id.iter().cloned().collect()
        } else if let Some(p) = &ev.playlist_id {
            db.faixas_playlist(p)
        } else if let Some(p) = &ev.pasta {
            db.ids_tocaveis("musica", Some(p))
        } else {
            db.ids_tocaveis("musica", None)
        };
        let recentes = db.recentes(self.ajustes.evitar_repeticao_ultimas.max(1) * 2);
        let chave = ev.playlist_id.clone().or_else(|| ev.pasta.clone()).unwrap_or_else(|| ev.id.clone());
        let mut cursor = *self.cursores.get(&chave).unwrap_or(&0);
        let id = escolher_faixa(
            &candidatas,
            &recentes,
            self.ajustes.evitar_repeticao_ultimas,
            ev.selecao,
            ev.faixa_id.as_deref(),
            &mut cursor,
            &mut self.rng,
        )?;
        self.cursores.insert(chave, cursor);
        db.faixa(&id).filter(|f| f.status == "ok")
    }

    /// Inicia um evento da programação.
    pub fn iniciar_evento(&mut self, db: &Db, audio: &Audio, ev: &Evento, retomada: bool, interromper: bool, continuar_janela: bool) {
        let mut itens = Vec::new();
        let mut descr = Vec::new();
        // Tempo de avisos antes da música (conta no limite de duração do trecho).
        let mut pre_aviso_ms: u64 = 0;
        if !continuar_janela && !retomada {
            if ev.aviso_atraso {
                itens.push(self.beep_atraso());
                pre_aviso_ms += 780;
                descr.push("aviso de atraso".to_string());
            }
            if ev.tipo.eh_troca() && ev.aviso_troca {
                itens.push(self.beep(db));
                pre_aviso_ms += 2300;
                descr.push("aviso de troca".to_string());
            }
            if let Some(v) = ev.vinheta_id.as_ref().and_then(|id| db.faixa(id)).filter(|f| f.status == "ok") {
                descr.push(format!("vinheta \"{}\"", v.nome));
                itens.push(self.item(&v, self.ajustes.volumes_tipo.vinheta));
            }
            if let Some(r) = ev.recado_id.as_ref().and_then(|id| db.faixa(id)).filter(|f| f.status == "ok") {
                if self.ajustes.beep_antes_recado && ev.vinheta_id.is_none() {
                    itens.push(self.beep(db));
                }
                descr.push(format!("recado \"{}\"", r.nome));
                itens.push(self.item(&r, ev.volume.max(1)));
            } else if ev.recado_id.is_some() {
                self.registrar(db, "agendador", "Erro", Some(&ev.id), &format!("{}: arquivo de recado ausente ou inválido", ev.nome));
            }
        }
        let quer_musica = ev.tipo != TipoEvento::Recado || ev.playlist_id.is_some() || ev.pasta.is_some() || ev.faixa_id.is_some();
        if quer_musica {
            match self.escolher_musica(db, ev) {
                Some(f) => {
                    descr.push(format!("\"{}\"", f.titulo.clone().unwrap_or(f.nome.clone())));
                    db.marcar_reproducao(&f.id, "agendador");
                    itens.push(self.item(&f, ev.volume));
                }
                None => {
                    let m = format!("{}: nenhuma música disponível na fonte configurada (pasta/playlist vazia ou arquivos ausentes)", ev.nome);
                    self.registrar(db, "agendador", "Erro", Some(&ev.id), &m);
                    self.alerta(m);
                }
            }
        }
        if itens.is_empty() {
            // Nada a tocar: libera o agendador.
            let _ = self.agendador.faixa_terminou(agora_momento());
            self.origem = Origem::Nenhuma;
            return;
        }
        let id = self.novo_id();
        self.id_atual = id;
        let limite = if ev.tipo.eh_troca() { ev.limite_segundos.map(|s| s as u64 * 1000 + pre_aviso_ms) } else { None };
        let _ = audio.tx.send(Cmd::Volume { geral: self.volume_master() });
        let _ = audio.tx.send(Cmd::Tocar {
            itens,
            fade_in_ms: if continuar_janela { 800 } else { ev.fade_in_ms },
            fade_saida_ms: if interromper { 1500 } else { 0 },
            limite_ms: limite,
            fade_fim_ms: ev.fade_out_ms.max(500),
            id,
            posicao_ms: 0,
        });
        let faixas = match &self.origem {
            Origem::Evento { evento_id, faixas_tocadas, .. } if *evento_id == ev.id && continuar_janela => faixas_tocadas + 1,
            _ => 1,
        };
        self.origem = Origem::Evento { evento_id: ev.id.clone(), nome: ev.nome.clone(), tipo_evento: ev.tipo, fim: ev.fim.clone(), faixas_tocadas: faixas };
        let prefixo = if retomada { "Retomado" } else if continuar_janela { "Próxima faixa" } else { "Iniciado" };
        self.registrar(db, "agendador", "Info", Some(&ev.id), &format!("{}: {} — {}", prefixo, ev.nome, descr.join(" → ")));
    }

    pub fn aplicar(&mut self, db: &Db, audio: &Audio, acoes: Vec<Acao>) {
        for a in acoes {
            match a {
                Acao::Iniciar { evento, retomada, interromper_atual } => {
                    // Se o Rádio Livre ou um item manual estiver tocando, interrompe com fade.
                    let algo = !matches!(self.origem, Origem::Nenhuma);
                    self.iniciar_evento(db, audio, &evento, retomada, interromper_atual || algo, false);
                }
                Acao::EncerrarJanela { evento_id, modo, fade_ms } => {
                    if modo == FimJanela::FadeOut {
                        let _ = audio.tx.send(Cmd::Parar { fade_ms });
                        if matches!(&self.origem, Origem::Evento { evento_id: e, .. } if *e == evento_id) {
                            self.origem = Origem::Nenhuma;
                            self.id_atual = 0;
                        }
                    }
                }
                Acao::Registro { nivel, evento_id, mensagem } => {
                    let n = match nivel {
                        Nivel::Info => "Info",
                        Nivel::Aviso => "Aviso",
                        Nivel::Erro => "Erro",
                    };
                    self.registrar(db, "agendador", n, evento_id.as_deref(), &mensagem);
                }
            }
        }
    }

    // ------------------------- Rádio Livre -------------------------

    pub fn tocar_fila(&mut self, db: &Db, audio: &Audio, indice: usize) -> Result<(), String> {
        if self.silenciado {
            return Err("A rádio está silenciada. Toque em \"Retomar\" primeiro.".into());
        }
        let Some(it) = self.fila.get(indice).cloned() else { return Err("Fila vazia".into()) };
        let Some(f) = db.faixa(&it.faixa_id) else {
            self.registrar(db, "manual", "Aviso", None, "Item da fila não existe mais na biblioteca; pulado");
            return self.pular_invalido(db, audio, indice);
        };
        if f.status != "ok" {
            self.registrar(db, "manual", "Aviso", None, &format!("\"{}\" está {}; pulado", f.nome, f.status));
            return self.pular_invalido(db, audio, indice);
        }
        let vol = match f.categoria.as_str() {
            "vinheta" => self.ajustes.volumes_tipo.vinheta,
            "recado" => self.ajustes.volumes_tipo.recado,
            _ => self.ajustes.volumes_tipo.livre,
        };
        let item = self.item(&f, vol);
        let id = self.novo_id();
        self.id_atual = id;
        self.indice_fila = Some(indice);
        let interromper = !matches!(self.origem, Origem::Nenhuma);
        let _ = audio.tx.send(Cmd::Volume { geral: self.volume_master() });
        let _ = audio.tx.send(Cmd::Tocar {
            itens: vec![item],
            // Rádio Livre usa os fades padrão das Configurações.
            fade_in_ms: self.ajustes.fade_in_padrao_ms,
            fade_saida_ms: if interromper { self.ajustes.fade_out_padrao_ms.clamp(300, 4000) } else { 0 },
            limite_ms: None,
            fade_fim_ms: self.ajustes.fade_out_padrao_ms.max(300),
            id,
            posicao_ms: 0,
        });
        db.marcar_reproducao(&f.id, "livre");
        self.origem = Origem::Livre { uid: it.uid };
        self.ultima_acao = Some(format!("{} — Rádio Livre: {}", chrono::Local::now().format("%H:%M:%S"), f.titulo.clone().unwrap_or(f.nome.clone())));
        Ok(())
    }

    fn pular_invalido(&mut self, db: &Db, audio: &Audio, indice: usize) -> Result<(), String> {
        self.falhas_seguidas += 1;
        if self.falhas_seguidas > 5 || self.fila.len() <= 1 {
            self.falhas_seguidas = 0;
            self.origem = Origem::Nenhuma;
            return Err("Vários arquivos seguidos estão inacessíveis. Verifique a biblioteca.".into());
        }
        let prox = (indice + 1) % self.fila.len();
        self.tocar_fila(db, audio, prox)
    }

    pub fn proximo_da_fila(&self, atual: usize, manual: bool) -> Option<usize> {
        let n = self.fila.len();
        if n == 0 {
            return None;
        }
        if !manual && self.repetir == Repetir::Faixa {
            return Some(atual.min(n - 1));
        }
        if self.aleatorio && n > 1 {
            let mut r = self.rng.clone();
            let mut i = r.abaixo(n - 1);
            if i >= atual {
                i += 1;
            }
            return Some(i);
        }
        if atual + 1 < n {
            Some(atual + 1)
        } else if self.repetir == Repetir::Fila || manual {
            Some(0)
        } else {
            None
        }
    }

    // ------------------------- eventos do áudio -------------------------

    pub fn evento_audio(&mut self, db: &Db, audio: &Audio, e: EventoAudio) {
        match e {
            EventoAudio::Terminou { id } => {
                if id != self.id_atual || id == 0 {
                    return;
                }
                self.id_atual = 0;
                match self.origem.clone() {
                    Origem::Evento { .. } => match self.agendador.faixa_terminou(agora_momento()) {
                        Proximo::TocarMais(ev) => {
                            if self.modo == Modo::Escolar && !self.silenciado {
                                self.iniciar_evento(db, audio, &ev, false, false, true);
                            }
                        }
                        Proximo::IniciarPendente(ev) => {
                            if self.modo == Modo::Escolar && !self.silenciado {
                                self.iniciar_evento(db, audio, &ev, false, false, false);
                            }
                        }
                        Proximo::Parar => {
                            self.origem = Origem::Nenhuma;
                        }
                    },
                    Origem::Livre { uid } => {
                        self.falhas_seguidas = 0;
                        let atual = self.fila.iter().position(|x| x.uid == uid).or(self.indice_fila).unwrap_or(0);
                        self.origem = Origem::Nenhuma;
                        if self.modo == Modo::Livre && !self.silenciado {
                            if let Some(p) = self.proximo_da_fila(atual, false) {
                                self.rng.next_u64();
                                if let Err(m) = self.tocar_fila(db, audio, p) {
                                    self.alerta(m);
                                }
                            }
                        }
                    }
                    Origem::Nenhuma => {}
                }
            }
            EventoAudio::Erro { mensagem, faixa_id, .. } => {
                if let Some(f) = &faixa_id {
                    db.marcar_status(f, "invalido", Some(&mensagem));
                }
                self.registrar(db, "audio", "Erro", None, &format!("{} (pulado)", mensagem));
            }
            EventoAudio::DispositivoPerdido { nome, detalhe } => {
                let m = format!("Saída de áudio perdida: {} A rádio foi pausada e NÃO foi redirecionada para outro alto-falante. Reconecte o Bluetooth e confirme em Diagnóstico do áudio.", detalhe);
                self.registrar(db, "audio", "Erro", None, &format!("{} ({})", m, nome));
                self.alerta(m);
            }
            EventoAudio::DispositivoReaberto { nome } => {
                self.alertas.retain(|a| !a.starts_with("Saída de áudio perdida"));
                self.registrar(db, "audio", "Info", None, &format!("Saída de áudio confirmada pelo operador: {} (reprodução segue pausada até tocar Play)", nome));
            }
            EventoAudio::AnuncioTerminou => {}
            EventoAudio::Microfonia => {
                let m = "Microfone desligado automaticamente: nível saturado (possível microfonia). Afaste o microfone da caixa e reduza o ganho.".to_string();
                self.registrar(db, "audio", "Aviso", None, &m);
                self.alerta(m);
            }
        }
    }

    pub fn anunciar(&mut self, db: &Db, audio: &Audio, recado: Option<&Faixa>, vinheta: Option<&Faixa>, beep: bool) {
        let mut itens = vec![];
        if let Some(v) = vinheta {
            itens.push(self.item(v, self.ajustes.volumes_tipo.vinheta));
        } else if beep {
            itens.push(self.beep(db));
        }
        if let Some(r) = recado {
            itens.push(self.item(r, self.ajustes.volumes_tipo.recado));
        }
        let _ = audio.tx.send(Cmd::Volume { geral: self.volume_master() });
        let _ = audio.tx.send(Cmd::Anunciar {
            itens,
            pausar_musica: self.ajustes.duck_modo == ModoDuck::Pausar,
            nivel_duck: self.ajustes.duck_nivel as f32 / 100.0,
        });
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Painel {
    pub agora: String,
    pub data_iso: String,
    pub modo: Modo,
    pub silenciado: bool,
    pub audio: StatusAudio,
    pub origem: Origem,
    pub proximos: Vec<ItemAgenda>,
    pub perfil_hoje: PerfilDia,
    pub alertas: Vec<String>,
    pub avisos_config: Vec<String>,
    pub ultimo_tick: Option<String>,
    pub ultima_acao: Option<String>,
    pub sistema: StatusSistema,
    pub indice_fila: Option<usize>,
    pub repetir: Repetir,
    pub aleatorio: bool,
    pub tamanho_fila: usize,
    pub volume_geral: u8,
    pub volume_maximo: u8,
    pub indexando: bool,
}

pub fn painel(est: &Estado) -> Painel {
    let audio = est.audio.status.lock().map(|s| s.clone()).unwrap_or_default();
    let n = est.nucleo.lock().unwrap();
    let agora = agora_momento();
    // Avisos que dependem do banco: recalculados no máximo a cada 5 s.
    static CACHE: Mutex<Option<(Instant, Vec<String>)>> = Mutex::new(None);
    let mut avisos = {
        let mut c = CACHE.lock().unwrap_or_else(|e| e.into_inner());
        let velho = c.as_ref().map(|(t, _)| t.elapsed() > Duration::from_secs(5)).unwrap_or(true);
        if velho {
            if let Ok(db) = est.db.try_lock() {
                let mut v = Vec::new();
                let pastas = db.pastas().unwrap_or_default();
                if pastas.iter().all(|p| p.categoria != "musica") {
                    v.push("Nenhuma pasta de músicas adicionada. Vá em Biblioteca → Adicionar pasta.".to_string());
                }
                for p in pastas.iter().filter(|p| !p.acessivel) {
                    v.push(format!("Pasta inacessível: {}", p.caminho));
                }
                if !pastas.is_empty() && db.conta_tocaveis("musica") == 0 {
                    v.push("Nenhuma música válida encontrada nas pastas.".to_string());
                }
                *c = Some((Instant::now(), v));
            }
        }
        c.as_ref().map(|(_, v)| v.clone()).unwrap_or_default()
    };
    if !n.ajustes.assistente_concluido {
        avisos.push("Primeira configuração não concluída. Abra o Assistente em Configurações.".to_string());
    }
    if let (Some(p), Some(false)) = (n.sistema.bateria.percentual, n.sistema.bateria.carregando) {
        if p <= n.ajustes.alerta_bateria {
            avisos.push(format!("Bateria baixa ({}%). Ligue o notebook na tomada.", p));
        }
    }
    Painel {
        agora: fmt_hms(agora.seg),
        data_iso: agora.data.to_string(),
        modo: n.modo,
        silenciado: n.silenciado,
        audio,
        origem: n.origem.clone(),
        proximos: proximos(&n.agenda, agora, 4),
        perfil_hoje: n.agenda.perfil_do_dia(agora.data),
        alertas: n.alertas.clone(),
        avisos_config: avisos,
        ultimo_tick: n.ultimo_tick.clone(),
        ultima_acao: n.ultima_acao.clone(),
        sistema: n.sistema.clone(),
        indice_fila: n.indice_fila,
        repetir: n.repetir,
        aleatorio: n.aleatorio,
        tamanho_fila: n.fila.len(),
        volume_geral: n.ajustes.volume_geral,
        volume_maximo: n.ajustes.volume_maximo,
        indexando: est.indexando.load(std::sync::atomic::Ordering::Relaxed),
    }
}

/// Laço residente: eventos de áudio (contínuo), agendador (1 s), painel (250 ms), sistema (30 s).
pub fn laco(app: AppHandle, rx: Receiver<EventoAudio>) {
    let mut ultimo_tick = Instant::now() - Duration::from_secs(2);
    let mut ultimo_painel = Instant::now();
    let mut ultimo_sistema = Instant::now() - Duration::from_secs(60);
    let mut ultimo_tooltip = String::new();
    loop {
        let Some(est) = app.try_state::<Arc<Estado>>() else {
            std::thread::sleep(Duration::from_millis(200));
            continue;
        };
        let est = est.inner().clone();
        // Eventos de áudio
        while let Ok(ev) = rx.try_recv() {
            let mut n = est.nucleo.lock().unwrap();
            let db = est.db.lock().unwrap();
            n.evento_audio(&db, &est.audio, ev);
        }
        // Agendador
        if ultimo_tick.elapsed() >= Duration::from_millis(1000) {
            ultimo_tick = Instant::now();
            let agora = agora_momento();
            let mut n = est.nucleo.lock().unwrap();
            let bloqueado = n.silenciado || !est.audio.status.lock().map(|s| s.dispositivo_ok).unwrap_or(true);
            let agenda = n.agenda.clone();
            let modo = n.modo;
            let acoes = n.agendador.tick(&agenda, agora, modo, bloqueado);
            n.ultimo_tick = Some(agora.to_string());
            if !acoes.is_empty() {
                let db = est.db.lock().unwrap();
                n.aplicar(&db, &est.audio, acoes);
            }
        }
        // Sistema (bateria / Wi-Fi) em thread separada para não atrasar o agendador.
        if ultimo_sistema.elapsed() >= Duration::from_secs(30) {
            ultimo_sistema = Instant::now();
            let e2 = est.clone();
            std::thread::spawn(move || {
                let s = crate::sistema::ler();
                if let Ok(mut n) = e2.nucleo.lock() {
                    let lim = n.ajustes.alerta_bateria;
                    if let (Some(p), Some(false)) = (s.bateria.percentual, s.bateria.carregando) {
                        if p <= lim && !n.alerta_bateria_dado {
                            n.alerta_bateria_dado = true;
                            if let Ok(db) = e2.db.lock() {
                                db.registrar("sistema", "Aviso", None, &format!("Bateria baixa: {}%", p));
                            }
                        }
                    } else {
                        n.alerta_bateria_dado = false;
                    }
                    n.sistema = s;
                }
            });
        }
        // Painel para a interface + ícone da bandeja
        if ultimo_painel.elapsed() >= Duration::from_millis(250) {
            ultimo_painel = Instant::now();
            let p = painel(&est);
            let tip = format!(
                "Rádio Escolar MJS — {}{}",
                if p.silenciado {
                    "SILENCIADA".to_string()
                } else if p.modo == Modo::Livre {
                    "Rádio Livre".to_string()
                } else {
                    "Rádio Escolar".to_string()
                },
                p.proximos.first().map(|x| format!(" · próximo: {} {}", x.inicio, x.nome)).unwrap_or_default()
            );
            if tip != ultimo_tooltip {
                if let Some(t) = app.tray_by_id("principal") {
                    let _ = t.set_tooltip(Some(&tip));
                }
                ultimo_tooltip = tip;
            }
            let _ = app.emit("painel", p);
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}
