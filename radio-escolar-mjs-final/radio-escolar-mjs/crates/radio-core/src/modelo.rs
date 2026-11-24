//! Modelo de configuração da programação (serializado em JSON no SQLite).

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::tempo::{parse_hora, Data};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum TipoEvento {
    /// Janela de músicas sequenciais na entrada dos alunos.
    Recepcao,
    /// Troca de professor: por padrão UMA música completa.
    Sinal,
    /// Janela de músicas sequenciais no intervalo.
    Recreio,
    /// Recado agendado (vinheta → recado).
    Recado,
    /// Saída do dia: troca com identidade própria (mesmo comportamento da troca).
    Saida,
}

impl TipoEvento {
    /// Trocas de professor (uma música por horário): sinalização e saída.
    pub fn eh_troca(&self) -> bool {
        matches!(self, TipoEvento::Sinal | TipoEvento::Saida)
    }
    pub fn eh_janela(&self) -> bool {
        matches!(self, TipoEvento::Recepcao | TipoEvento::Recreio)
    }
    /// Prioridade padrão usada quando o evento não define outra.
    pub fn prioridade_padrao(&self) -> i32 {
        match self {
            TipoEvento::Recreio => 80,
            TipoEvento::Recepcao => 60,
            TipoEvento::Recado => 40,
            TipoEvento::Sinal | TipoEvento::Saida => 20,
        }
    }
    pub fn nome(&self) -> &'static str {
        match self {
            TipoEvento::Recepcao => "Recepção",
            TipoEvento::Sinal => "Sinalização",
            TipoEvento::Saida => "Saída",
            TipoEvento::Recreio => "Recreio",
            TipoEvento::Recado => "Recado",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ModoSelecao {
    /// Aleatório evitando as faixas tocadas recentemente.
    Aleatorio,
    /// Em ordem, continuando de onde parou.
    Sequencial,
    /// Sempre a faixa escolhida em `faixa_id`.
    FaixaFixa,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum FimJanela {
    /// No horário limite, abaixa suavemente e para. (PADRÃO)
    FadeOut,
    /// Deixa terminar a faixa em andamento e não inicia outra
    /// (com teto de segurança `extra_max_segundos`).
    TerminarFaixa,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum PoliticaConflito {
    /// Abaixa (fade-out) o que estiver tocando e inicia este evento. (PADRÃO)
    Interromper,
    /// Espera o que estiver tocando terminar e então inicia, se ainda válido.
    Aguardar,
    /// Não toca este evento se já houver algo tocando.
    Ignorar,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(rename_all = "camelCase", default))]
pub struct Evento {
    pub id: String,
    pub nome: String,
    pub tipo: TipoEvento,
    pub ativo: bool,
    /// Dias da semana (0 = segunda … 6 = domingo).
    pub dias: Vec<u8>,
    /// Perfis de dia em que o evento vale (ex.: "normal", "prova").
    pub perfis: Vec<String>,
    /// "HH:MM" ou "HH:MM:SS".
    pub inicio: String,
    /// Obrigatório para janelas (recepção/recreio).
    pub fim: Option<String>,
    /// Origem das músicas: playlist, pasta ou faixa fixa. Se nenhuma, usa a categoria Músicas inteira.
    pub playlist_id: Option<String>,
    pub pasta: Option<String>,
    pub faixa_id: Option<String>,
    pub selecao: ModoSelecao,
    /// 0–100 (multiplicado pelo volume geral e limitado pelo teto).
    pub volume: u8,
    pub fade_in_ms: u32,
    pub fade_out_ms: u32,
    /// Para sinalização: limita a duração (trecho). `None` = música completa.
    pub limite_segundos: Option<u32>,
    pub fim_janela: FimJanela,
    /// Teto após o horário final quando `fim_janela = TerminarFaixa`.
    pub extra_max_segundos: u32,
    /// Vinheta tocada antes (abertura do recreio, antes de recado…).
    pub vinheta_id: Option<String>,
    /// Recado tocado (eventos do tipo Recado ou sequência vinheta → recado → música).
    pub recado_id: Option<String>,
    pub prioridade: i32,
    pub conflito: PoliticaConflito,
    pub observacao: String,
    /// Aviso de troca (ding-dong) antes da música. Pré-marcado nas trocas e saídas.
    #[cfg_attr(feature = "serde", serde(default = "verdadeiro"))]
    pub aviso_troca: bool,
    /// Aviso de atraso (campainha diferente) antes do evento.
    pub aviso_atraso: bool,
}

#[cfg(feature = "serde")]
fn verdadeiro() -> bool {
    true
}

impl Default for Evento {
    fn default() -> Self {
        Evento {
            id: String::new(),
            nome: String::new(),
            tipo: TipoEvento::Sinal,
            ativo: true,
            dias: vec![0, 1, 2, 3, 4],
            perfis: vec!["normal".into()],
            inicio: "07:00".into(),
            fim: None,
            playlist_id: None,
            pasta: None,
            faixa_id: None,
            selecao: ModoSelecao::Aleatorio,
            volume: 60,
            fade_in_ms: 1500,
            fade_out_ms: 4000,
            limite_segundos: None,
            fim_janela: FimJanela::FadeOut,
            extra_max_segundos: 240,
            vinheta_id: None,
            recado_id: None,
            prioridade: 20,
            conflito: PoliticaConflito::Interromper,
            observacao: String::new(),
            aviso_troca: true,
            aviso_atraso: false,
        }
    }
}

impl Evento {
    pub fn inicio_seg(&self) -> Option<u32> {
        parse_hora(&self.inicio)
    }
    pub fn fim_seg(&self) -> Option<u32> {
        self.fim.as_deref().and_then(parse_hora)
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(rename_all = "camelCase"))]
pub struct Perfil {
    pub id: String,
    pub nome: String,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(rename_all = "camelCase", default))]
pub struct Excecao {
    /// "AAAA-MM-DD"
    pub data: String,
    /// Perfil a usar nesse dia. `None` = dia suspenso (nada toca).
    pub perfil: Option<String>,
    pub descricao: String,
}

impl Default for Excecao {
    fn default() -> Self {
        Excecao { data: String::new(), perfil: None, descricao: String::new() }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(rename_all = "camelCase", default))]
pub struct Agenda {
    pub eventos: Vec<Evento>,
    pub perfis: Vec<Perfil>,
    /// Perfil padrão de cada dia da semana (0 = segunda). `None` = sem aula.
    pub perfil_por_dia_semana: Vec<Option<String>>,
    pub excecoes: Vec<Excecao>,
    /// Atraso máximo aceitável para disparar um evento pontual (segundos).
    pub tolerancia_segundos: u32,
    /// Intervalo sem "batidas" do relógio que caracteriza suspensão/hibernação/salto de relógio.
    pub salto_segundos: u32,
    /// Após reinício/suspensão/volta do Rádio Livre dentro de uma janela,
    /// retomar a playlist da janela atual (nunca sinais anteriores).
    pub retomar_janela_apos_reinicio: bool,
    /// Tempo máximo que um evento com política "Aguardar" pode esperar.
    pub espera_max_segundos: u32,
    /// Eventos desligados pelo interruptor, válidos só na data indicada.
    pub desligados: Vec<Desligado>,
}

/// Interruptor por toque: o evento fica desligado só naquela data.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(rename_all = "camelCase", default))]
pub struct Desligado {
    /// "AAAA-MM-DD"
    pub data: String,
    pub evento_id: String,
}

impl Default for Agenda {
    fn default() -> Self {
        Agenda {
            eventos: vec![],
            perfis: vec![],
            perfil_por_dia_semana: vec![None; 7],
            excecoes: vec![],
            tolerancia_segundos: 60,
            salto_segundos: 90,
            retomar_janela_apos_reinicio: true,
            espera_max_segundos: 300,
            desligados: vec![],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "tipo", content = "valor"))]
pub enum PerfilDia {
    Perfil(String),
    Suspenso(String),
    SemAula,
}

impl Agenda {
    pub fn evento(&self, id: &str) -> Option<&Evento> {
        self.eventos.iter().find(|e| e.id == id)
    }

    pub fn perfil_do_dia(&self, data: Data) -> PerfilDia {
        let ds = data.to_string();
        if let Some(ex) = self.excecoes.iter().rev().find(|e| e.data == ds) {
            return match &ex.perfil {
                Some(p) => PerfilDia::Perfil(p.clone()),
                None => PerfilDia::Suspenso(ex.descricao.clone()),
            };
        }
        match self.perfil_por_dia_semana.get(data.dia_semana() as usize).cloned().flatten() {
            Some(p) => PerfilDia::Perfil(p),
            None => PerfilDia::SemAula,
        }
    }

    /// Eventos válidos numa data, ordenados por horário e prioridade (maior primeiro).
    /// Eventos que valem na data, sem considerar o interruptor (usado para exibir a lista do dia).
    pub fn eventos_do_dia_todos(&self, data: Data) -> Vec<&Evento> {
        let perfil = match self.perfil_do_dia(data) {
            PerfilDia::Perfil(p) => p,
            _ => return vec![],
        };
        let ds = data.dia_semana();
        let mut v: Vec<&Evento> = self
            .eventos
            .iter()
            .filter(|e| e.ativo && e.inicio_seg().is_some())
            .filter(|e| e.perfis.iter().any(|p| *p == perfil))
            .filter(|e| {
                // Em dias de exceção com perfil explícito, o dia da semana não restringe
                // (ex.: sábado letivo). Nos demais, respeita a lista de dias.
                e.dias.contains(&ds) || self.excecoes.iter().any(|x| x.data == data.to_string() && x.perfil.is_some())
            })
            .collect();
        v.sort_by(|a, b| {
            a.inicio_seg()
                .cmp(&b.inicio_seg())
                .then(b.prioridade.cmp(&a.prioridade))
                .then(a.id.cmp(&b.id))
        });
        v
    }

    /// Eventos que tocam na data: os do dia, menos os desligados pelo interruptor.
    pub fn eventos_do_dia(&self, data: Data) -> Vec<&Evento> {
        let iso = data.to_string();
        self.eventos_do_dia_todos(data)
            .into_iter()
            .filter(|e| !self.desligado_em(&iso, &e.id))
            .collect()
    }

    /// Verdadeiro se o interruptor desligou este evento nesta data.
    pub fn desligado_em(&self, iso: &str, evento_id: &str) -> bool {
        self.desligados.iter().any(|d| d.data == iso && d.evento_id == evento_id)
    }
}
