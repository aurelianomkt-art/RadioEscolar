//! Leituras do Windows (bateria e Wi-Fi). Quando não há leitura confiável, devolve "indisponível".

use serde::Serialize;

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bateria {
    pub disponivel: bool,
    pub percentual: Option<u8>,
    pub carregando: Option<bool>,
    pub sem_bateria: bool,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WiFi {
    pub disponivel: bool,
    pub conectado: Option<bool>,
    pub ssid: Option<String>,
    pub sinal: Option<u8>,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusSistema {
    pub bateria: Bateria,
    pub wifi: WiFi,
    pub atualizado_em: String,
}

pub fn ler() -> StatusSistema {
    StatusSistema { bateria: bateria(), wifi: wifi(), atualizado_em: chrono::Local::now().format("%H:%M:%S").to_string() }
}

#[cfg(windows)]
fn bateria() -> Bateria {
    use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
    let mut s: SYSTEM_POWER_STATUS = unsafe { std::mem::zeroed() };
    let ok = unsafe { GetSystemPowerStatus(&mut s) };
    if ok == 0 {
        return Bateria::default();
    }
    // BatteryFlag 128 = sem bateria; 255 = desconhecido.
    if s.BatteryFlag == 128 {
        return Bateria { disponivel: true, percentual: None, carregando: Some(true), sem_bateria: true };
    }
    if s.BatteryFlag == 255 {
        return Bateria::default();
    }
    Bateria {
        disponivel: true,
        percentual: if s.BatteryLifePercent <= 100 { Some(s.BatteryLifePercent) } else { None },
        carregando: match s.ACLineStatus {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        },
        sem_bateria: false,
    }
}

#[cfg(not(windows))]
fn bateria() -> Bateria {
    Bateria::default()
}

/// Lê `netsh wlan show interfaces` (funciona em Windows em português ou inglês).
#[cfg(windows)]
fn wifi() -> WiFi {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let out = std::process::Command::new("netsh").args(["wlan", "show", "interfaces"]).creation_flags(CREATE_NO_WINDOW).output();
    let Ok(out) = out else { return WiFi::default() };
    // netsh usa a página de código do console; decodifica com perdas (acentos não importam aqui).
    let txt = String::from_utf8_lossy(&out.stdout).to_string();
    interpretar_netsh(&txt)
}

#[cfg(not(windows))]
fn wifi() -> WiFi {
    WiFi::default()
}

#[allow(dead_code)]
pub fn interpretar_netsh(txt: &str) -> WiFi {
    let mut w = WiFi::default();
    let mut achou_interface = false;
    for linha in txt.lines() {
        let Some((k, v)) = linha.split_once(':') else { continue };
        let k = k.trim().to_lowercase();
        let v = v.trim();
        if k == "ssid" {
            w.ssid = Some(v.to_string()).filter(|s| !s.is_empty());
            achou_interface = true;
        } else if k.starts_with("estado") || k == "state" {
            achou_interface = true;
            let vl = v.to_lowercase();
            w.conectado = Some(vl.starts_with("conectado") || vl == "connected");
        } else if k.starts_with("sinal") || k == "signal" {
            w.sinal = v.trim_end_matches('%').trim().parse().ok();
        }
    }
    w.disponivel = achou_interface;
    if w.conectado == Some(false) {
        w.ssid = None;
        w.sinal = None;
    }
    w
}
