//! Energia do notebook no Windows: tampa fechada e desligamento agendado.
//!
//! Usa apenas ferramentas que o próprio Windows traz (`powercfg`, `schtasks`, `cmd`).
//! Nada aqui exige senha de administrador: as alterações são do plano de energia e
//! de uma tarefa do usuário atual.

use std::process::Command;

type R<T> = Result<T, String>;

const TAREFA_DESLIGAR: &str = "RadioEscolarDesligar";

#[cfg(windows)]
fn executar(programa: &str, args: &[&str]) -> R<String> {
    use std::os::windows::process::CommandExt;
    const SEM_JANELA: u32 = 0x0800_0000; // CREATE_NO_WINDOW
    let saida = Command::new(programa)
        .args(args)
        .creation_flags(SEM_JANELA)
        .output()
        .map_err(|e| format!("Não foi possível executar {}: {}", programa, e))?;
    if saida.status.success() {
        Ok(String::from_utf8_lossy(&saida.stdout).to_string())
    } else {
        let erro = String::from_utf8_lossy(&saida.stderr).trim().to_string();
        Err(format!("{} não concluiu: {}", programa, if erro.is_empty() { "erro desconhecido".into() } else { erro }))
    }
}

#[cfg(not(windows))]
fn executar(programa: &str, _args: &[&str]) -> R<String> {
    Err(format!("{} só funciona no Windows", programa))
}

/// Válido: "HH:MM" entre 00:00 e 23:59.
fn hora_valida(h: &str) -> bool {
    let partes: Vec<&str> = h.split(':').collect();
    if partes.len() != 2 || partes[0].len() != 2 || partes[1].len() != 2 {
        return false;
    }
    match (partes[0].parse::<u32>(), partes[1].parse::<u32>()) {
        (Ok(hh), Ok(mm)) => hh < 24 && mm < 60,
        _ => false,
    }
}

/// Mantém o notebook ligado com a tampa fechada (`manter = true`) ou restaura
/// "Suspender ao fechar a tampa" (padrão do Windows para notebooks).
#[tauri::command]
pub fn energia_tampa(manter: bool) -> R<()> {
    let valor = if manter { "0" } else { "1" };
    executar("powercfg", &["/setacvalueindex", "SCHEME_CURRENT", "SUB_BUTTONS", "LIDACTION", valor])?;
    executar("powercfg", &["/setdcvalueindex", "SCHEME_CURRENT", "SUB_BUTTONS", "LIDACTION", valor])?;
    executar("powercfg", &["/setactive", "SCHEME_CURRENT"])?;
    Ok(())
}

/// Desliga o notebook todo dia no horário indicado; `None` cancela o desligamento.
#[tauri::command]
pub fn energia_desligar_em(hora: Option<String>) -> R<()> {
    match hora {
        None => {
            // Se a tarefa não existir, não há o que cancelar.
            let _ = executar("schtasks", &["/Delete", "/TN", TAREFA_DESLIGAR, "/F"]);
            Ok(())
        }
        Some(h) => {
            if !hora_valida(&h) {
                return Err("Horário inválido. Use HH:MM.".into());
            }
            executar(
                "schtasks",
                &["/Create", "/SC", "DAILY", "/ST", h.as_str(), "/TN", TAREFA_DESLIGAR, "/TR", "shutdown /s /f /t 60", "/F"],
            )?;
            Ok(())
        }
    }
}

/// Abre a página de energia do Windows, onde o usuário ajusta o que faltar.
#[tauri::command]
pub fn energia_abrir_configuracoes() -> R<()> {
    executar("cmd", &["/C", "start", "", "ms-settings:powersleep"]).map(|_| ())
}
