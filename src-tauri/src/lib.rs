//! Rádio Escolar Mestre João Silvério — processo nativo (Tauri 2).

mod audio;
mod biblioteca;
mod comandos;
mod config;
mod db;
mod demo;
mod energia;
mod motor;
mod sistema;

use config::Ajustes;
use motor::{Estado, ItemFila, Nucleo, Origem, Repetir};
use radio_core::{agenda_padrao, Agenda, Agendador, Modo, Rng};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, WindowEvent};

/// Versão portátil: se existir `portatil.txt` ao lado do executável, os dados ficam em `dados\` ao lado dele.
pub fn modo_portatil() -> bool {
    std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join("portatil.txt").exists())).unwrap_or(false)
}

fn pasta_dados(app: &AppHandle) -> Result<PathBuf, String> {
    if modo_portatil() {
        if let Some(d) = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join("dados"))) {
            return Ok(d);
        }
    }
    app.path().app_data_dir().map_err(|e| e.to_string())
}

pub fn mostrar_janela(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

fn criar_bandeja(app: &AppHandle) -> tauri::Result<()> {
    let abrir = MenuItem::with_id(app, "abrir", "Abrir painel", true, None::<&str>)?;
    let silenciar = MenuItem::with_id(app, "silenciar", "SILENCIAR AGORA", true, None::<&str>)?;
    let retomar = MenuItem::with_id(app, "retomar", "Retomar", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let sair = MenuItem::with_id(app, "sair", "Sair do aplicativo", true, None::<&str>)?;
    let itens: [&dyn tauri::menu::IsMenuItem<tauri::Wry>; 5] = [&abrir, &silenciar, &retomar, &sep, &sair];
    let menu = Menu::with_items(app, &itens)?;
    let mut b = TrayIconBuilder::with_id("principal")
        .tooltip("Rádio Escolar Mestre João Silvério")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, ev| match ev.id().as_ref() {
            "abrir" => mostrar_janela(app),
            "silenciar" => {
                let est = app.state::<Arc<Estado>>();
                comandos::silenciar_agora(est);
            }
            "retomar" => {
                let est = app.state::<Arc<Estado>>();
                comandos::retomar(est);
            }
            "sair" => {
                if let Some(est) = app.try_state::<Arc<Estado>>() {
                    let _ = est.audio.tx.send(audio::Cmd::Silenciar);
                    if let Ok(db) = est.db.lock() {
                        db.registrar("sistema", "Info", None, "Aplicativo encerrado pelo menu da bandeja");
                    }
                }
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, ev| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = ev {
                mostrar_janela(tray.app_handle());
            }
        });
    if let Some(icone) = app.default_window_icon() {
        b = b.icon(icone.clone());
    }
    b.build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            // Impede duas instâncias (duas rádios tocando juntas): só mostra a janela existente.
            mostrar_janela(app);
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec!["--minimizado"])))
        .setup(|app| {
            let dados = pasta_dados(app.handle())?;
            std::fs::create_dir_all(&dados)?;
            let db_path = dados.join("radio.db");
            let db = db::Db::abrir(&db_path)?;

            let agenda: Agenda = db
                .ler_config("agenda")?
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_else(agenda_padrao);
            let ajustes: Ajustes = db.ler_config("ajustes")?.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
            let fila: Vec<ItemFila> = db.ler_config("fila")?.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
            let modo = match db.ler_config("modo")?.as_deref() {
                Some("Livre") => Modo::Livre,
                _ => Modo::Escolar,
            };
            if db.ler_config("agenda")?.is_none() {
                let _ = db.gravar_config("agenda", &serde_json::to_string(&agenda).unwrap_or_default());
            }
            db.podar_historico();
            db.registrar(
                "sistema",
                "Info",
                None,
                &format!(
                    "Aplicativo iniciado (v{}, modo {}). Eventos anteriores a agora não serão tocados.",
                    env!("CARGO_PKG_VERSION"),
                    if modo == Modo::Livre { "Rádio Livre" } else { "Rádio Escolar" }
                ),
            );

            let (tx_ev, rx_ev) = std::sync::mpsc::channel();
            let master = (ajustes.volume_geral.min(ajustes.volume_maximo) as f32) / 100.0;
            let audio = audio::iniciar(ajustes.dispositivo_saida.clone(), master, tx_ev);
            let semente = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(7);
            let uid_seq = fila.iter().map(|i| i.uid).max().unwrap_or(0);
            let iniciar_min = ajustes.iniciar_minimizado;
            let nucleo = Nucleo {
                agenda,
                ajustes,
                agendador: Agendador::new(),
                modo,
                silenciado: false,
                origem: Origem::Nenhuma,
                id_seq: 0,
                id_atual: 0,
                rng: Rng::new(semente),
                cursores: Default::default(),
                fila,
                indice_fila: None,
                repetir: Repetir::Nao,
                aleatorio: false,
                falhas_seguidas: 0,
                alertas: vec![],
                ultimo_tick: None,
                ultima_acao: None,
                sistema: Default::default(),
                alerta_bateria_dado: false,
                uid_seq,
            };
            let est = Arc::new(Estado {
                nucleo: Mutex::new(nucleo),
                db: Mutex::new(db),
                db_path,
                dados_dir: dados,
                audio,
                desbloqueado_ate: Mutex::new(None),
                indexando: std::sync::atomic::AtomicBool::new(false),
            });
            app.manage(est.clone());
            criar_bandeja(app.handle())?;

            // Laço residente do agendador (independe da janela).
            let h = app.handle().clone();
            std::thread::Builder::new().name("agendador".into()).spawn(move || motor::laco(h, rx_ev))?;

            // Reindexação ao iniciar + automática.
            comandos::indexar_em_segundo_plano(app.handle().clone(), est.clone());
            let h2 = app.handle().clone();
            let e2 = est.clone();
            std::thread::spawn(move || {
                let mut minutos = 0u32;
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(60));
                    minutos += 1;
                    let intervalo = e2.nucleo.lock().map(|n| n.ajustes.reindexar_auto_min).unwrap_or(0);
                    if intervalo > 0 && minutos >= intervalo {
                        minutos = 0;
                        comandos::indexar_em_segundo_plano(h2.clone(), e2.clone());
                    }
                }
            });

            if std::env::args().any(|a| a == "--minimizado") && iniciar_min {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.hide();
                }
            }
            Ok(())
        })
        .on_window_event(|janela, ev| {
            if let WindowEvent::CloseRequested { api, .. } = ev {
                let para_bandeja = janela
                    .app_handle()
                    .try_state::<Arc<Estado>>()
                    .map(|e| e.nucleo.lock().map(|n| n.ajustes.minimizar_para_bandeja).unwrap_or(true))
                    .unwrap_or(true);
                if para_bandeja {
                    // A rádio continua funcionando na bandeja.
                    let _ = janela.hide();
                    api.prevent_close();
                } else if let Some(est) = janela.app_handle().try_state::<Arc<Estado>>() {
                    let _ = est.audio.tx.send(audio::Cmd::Silenciar);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            comandos::painel_agora,
            comandos::obter_config,
            comandos::definir_modo,
            comandos::silenciar_agora,
            comandos::retomar,
            comandos::tocar_pausar,
            comandos::proxima,
            comandos::anterior,
            comandos::definir_volume,
            comandos::buscar,
            comandos::fila_obter,
            comandos::fila_adicionar,
            comandos::fila_adicionar_playlist,
            comandos::fila_reordenar,
            comandos::fila_remover,
            comandos::fila_limpar,
            comandos::fila_tocar,
            comandos::definir_repetir,
            comandos::definir_aleatorio,
            comandos::listar_pastas,
            comandos::adicionar_pasta,
            comandos::remover_pasta,
            comandos::reindexar,
            comandos::listar_faixas,
            comandos::definir_categoria_faixa,
            comandos::definir_volume_faixa,
            comandos::resumo_biblioteca,
            comandos::listar_playlists,
            comandos::salvar_playlist,
            comandos::excluir_playlist,
            comandos::tocar_recado,
            comandos::validar_agenda,
            comandos::salvar_agenda,
            comandos::agenda_dia,
            comandos::agenda_inicial,
            comandos::salvar_ajustes,
            comandos::definir_pin,
            comandos::desbloquear,
            comandos::bloquear,
            comandos::esta_desbloqueado,
            comandos::definir_autostart,
            comandos::concluir_assistente,
            comandos::obter_logo,
            comandos::definir_logo,
            comandos::dispositivos,
            comandos::capa,
            comandos::selecionar_dispositivo,
            comandos::confirmar_dispositivo,
            comandos::tocar_teste,
            comandos::diagnostico,
            comandos::limpar_alertas,
            comandos::mic_abrir,
            comandos::mic_fechar,
            comandos::mic_falar,
            comandos::mic_ajuste,
            comandos::listar_historico,
            comandos::exportar_historico,
            comandos::exportar_config,
            comandos::importar_config,
            energia::energia_tampa,
            energia::energia_desligar_em,
            energia::energia_abrir_configuracoes,
            comandos::criar_demo,
        ])
        .run(tauri::generate_context!())
        .expect("erro ao iniciar a Rádio Escolar");
}
