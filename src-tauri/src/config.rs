//! Ajustes do aplicativo (persistidos como JSON na tabela `config`).

use radio_core::Agenda;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModoDuck {
    /// Pausa a música com fade, toca o recado e retoma do mesmo ponto. (PADRÃO)
    Pausar,
    /// Abaixa a música para `duck_nivel` durante o recado.
    Abaixar,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct VolumesTipo {
    pub recepcao: u8,
    pub sinal: u8,
    pub recreio: u8,
    pub recado: u8,
    pub vinheta: u8,
    pub livre: u8,
}

impl Default for VolumesTipo {
    fn default() -> Self {
        VolumesTipo { recepcao: 60, sinal: 55, recreio: 60, recado: 70, vinheta: 50, livre: 60 }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Ajustes {
    /// Volume geral 0–100 (aplicado sobre o volume de cada evento).
    pub volume_geral: u8,
    /// Teto absoluto de volume geral (proteção auditiva / ambiente inclusivo).
    pub volume_maximo: u8,
    /// Teto específico para vinhetas e beep (0–100).
    pub limite_vinheta: u8,
    pub volumes_tipo: VolumesTipo,
    pub fade_in_padrao_ms: u32,
    pub fade_out_padrao_ms: u32,
    /// Quantas faixas recentes evitar no modo aleatório.
    pub evitar_repeticao_ultimas: usize,
    /// Nome do dispositivo de saída. `None` = dispositivo padrão do Windows.
    pub dispositivo_saida: Option<String>,
    /// Beep/vinheta antes de recados.
    pub beep_antes_recado: bool,
    /// Faixa usada como beep (None = beep suave interno, sintetizado).
    pub beep_faixa_id: Option<String>,
    pub duck_modo: ModoDuck,
    /// Nível da música durante recado quando `duck_modo = Abaixar` (0–100).
    pub duck_nivel: u8,
    /// Alerta de bateria baixa (%).
    pub alerta_bateria: u8,
    pub indexar_subpastas: bool,
    /// Reindexação automática (minutos; 0 = desligada).
    pub reindexar_auto_min: u32,
    pub iniciar_com_windows: bool,
    pub iniciar_minimizado: bool,
    pub minimizar_para_bandeja: bool,
    /// Escala da interface (0.8–1.6).
    pub escala: f32,
    pub alto_contraste: bool,
    /// Logo personalizada (caminho copiado para a pasta de dados).
    pub logo_arquivo: Option<String>,
    pub nome_escola: String,
    pub assistente_concluido: bool,
    /// Microfone: ganho 0–100 e limiar do portão de ruído 0–100.
    pub mic_ganho: u8,
    pub mic_portao: u8,
    pub mic_dispositivo: Option<String>,
    /// Hash do PIN (SHA-256 com sal). Vazio = sem PIN.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub pin_hash: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub pin_sal: String,
    /// Minutos que o desbloqueio por PIN vale.
    pub pin_validade_min: u32,
}

impl Default for Ajustes {
    fn default() -> Self {
        Ajustes {
            volume_geral: 70,
            volume_maximo: 85,
            limite_vinheta: 60,
            volumes_tipo: VolumesTipo::default(),
            fade_in_padrao_ms: 2000,
            fade_out_padrao_ms: 4000,
            evitar_repeticao_ultimas: 15,
            dispositivo_saida: None,
            beep_antes_recado: true,
            beep_faixa_id: None,
            duck_modo: ModoDuck::Pausar,
            duck_nivel: 20,
            alerta_bateria: 25,
            indexar_subpastas: true,
            reindexar_auto_min: 30,
            iniciar_com_windows: false,
            iniciar_minimizado: false,
            minimizar_para_bandeja: true,
            escala: 1.0,
            alto_contraste: false,
            logo_arquivo: None,
            nome_escola: "Rádio Escolar Mestre João Silvério".into(),
            assistente_concluido: false,
            mic_ganho: 50,
            mic_portao: 8,
            mic_dispositivo: None,
            pin_hash: String::new(),
            pin_sal: String::new(),
            pin_validade_min: 10,
        }
    }
}

impl Ajustes {
    /// Cópia para enviar à interface (sem hash/sal do PIN).
    pub fn publico(&self) -> AjustesPublicos {
        AjustesPublicos { ajustes: Ajustes { pin_hash: String::new(), pin_sal: String::new(), ..self.clone() }, tem_pin: !self.pin_hash.is_empty() }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AjustesPublicos {
    #[serde(flatten)]
    pub ajustes: Ajustes,
    pub tem_pin: bool,
}

/// Arquivo de backup/exportação.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Backup {
    pub formato: String,
    pub versao_app: String,
    pub exportado_em: String,
    pub agenda: Agenda,
    pub ajustes: Ajustes,
    pub pastas: Vec<crate::db::Pasta>,
    pub playlists: Vec<crate::db::Playlist>,
}
