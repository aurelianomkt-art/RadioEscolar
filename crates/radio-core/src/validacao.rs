//! Validação da programação antes de salvar, com sugestões.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::modelo::{Agenda, Evento, FimJanela, ModoSelecao, TipoEvento};
use crate::tempo::{fmt_hm, nome_dia, Data};
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Gravidade {
    /// Impede salvar.
    Erro,
    /// Permite salvar, mas o funcionário deve conferir.
    Alerta,
    /// Apenas informação (ex.: recreio prevalece sobre sinal no mesmo minuto).
    Info,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(rename_all = "camelCase"))]
pub struct Aviso {
    pub gravidade: Gravidade,
    pub evento_id: Option<String>,
    pub mensagem: String,
    pub sugestao: Option<String>,
}

fn av(g: Gravidade, e: Option<&Evento>, m: String, s: Option<String>) -> Aviso {
    Aviso { gravidade: g, evento_id: e.map(|x| x.id.clone()), mensagem: m, sugestao: s }
}

pub fn validar(agenda: &Agenda) -> Vec<Aviso> {
    use Gravidade::*;
    let mut out = Vec::new();
    let mut ids = HashSet::new();
    let perfis: HashSet<&str> = agenda.perfis.iter().map(|p| p.id.as_str()).collect();

    for e in &agenda.eventos {
        if e.id.trim().is_empty() {
            out.push(av(Erro, Some(e), format!("Evento \"{}\" sem identificador", e.nome), None));
        }
        if !ids.insert(e.id.clone()) {
            out.push(av(Erro, Some(e), format!("Identificador repetido: {}", e.id), Some("Duplique o evento pelo botão Duplicar, que gera um novo identificador".into())));
        }
        if e.nome.trim().is_empty() {
            out.push(av(Alerta, Some(e), "Evento sem nome".into(), Some("Dê um nome fácil, como \"Troca 2ª aula\"".into())));
        }
        let ini = e.inicio_seg();
        if ini.is_none() {
            out.push(av(Erro, Some(e), format!("{}: horário inicial inválido ({})", e.nome, e.inicio), Some("Use o formato HH:MM, por exemplo 07:50".into())));
        }
        if e.tipo.eh_janela() {
            match (ini, e.fim_seg()) {
                (_, None) => out.push(av(Erro, Some(e), format!("{}: {} precisa de horário final", e.nome, e.tipo.nome()), Some("Informe até que horas as músicas devem tocar".into()))),
                (Some(i), Some(f)) if f <= i => out.push(av(Erro, Some(e), format!("{}: horário final ({}) não é depois do inicial ({})", e.nome, fmt_hm(f), fmt_hm(i)), None)),
                (Some(i), Some(f)) if f - i > 4 * 3600 => out.push(av(Alerta, Some(e), format!("{}: janela muito longa ({} min)", e.nome, (f - i) / 60), Some("Confira se o horário final está correto".into()))),
                _ => {}
            }
        } else if e.fim.as_deref().map(|s| !s.trim().is_empty()).unwrap_or(false) {
            out.push(av(Info, Some(e), format!("{}: horário final é ignorado em {}", e.nome, e.tipo.nome()), Some("Para limitar a duração, use \"Limite de duração\"".into())));
        }
        if e.volume > 100 {
            out.push(av(Erro, Some(e), format!("{}: volume acima de 100", e.nome), None));
        }
        if e.ativo && e.dias.is_empty() {
            out.push(av(Alerta, Some(e), format!("{}: nenhum dia da semana marcado", e.nome), Some("Marque os dias ou desative o evento".into())));
        }
        if e.ativo && e.perfis.is_empty() {
            out.push(av(Alerta, Some(e), format!("{}: não pertence a nenhum perfil de dia e nunca tocará", e.nome), Some("Marque pelo menos o perfil \"Dia normal\"".into())));
        }
        for p in &e.perfis {
            if !perfis.is_empty() && !perfis.contains(p.as_str()) {
                out.push(av(Alerta, Some(e), format!("{}: perfil desconhecido \"{}\"", e.nome, p), None));
            }
        }
        if e.selecao == ModoSelecao::FaixaFixa && e.faixa_id.is_none() {
            out.push(av(Erro, Some(e), format!("{}: modo \"faixa fixa\" sem faixa escolhida", e.nome), Some("Escolha a faixa ou mude para Aleatório".into())));
        }
        if e.tipo == TipoEvento::Recado && e.recado_id.is_none() {
            out.push(av(Erro, Some(e), format!("{}: recado agendado sem arquivo de recado", e.nome), None));
        }
        if e.tipo.eh_troca() && e.limite_segundos == Some(0) {
            out.push(av(Erro, Some(e), format!("{}: limite de duração zero", e.nome), Some("Deixe em branco para tocar a música completa".into())));
        }
        if e.tipo.eh_janela() && e.fim_janela == FimJanela::TerminarFaixa && e.extra_max_segundos > 600 {
            out.push(av(Alerta, Some(e), format!("{}: teto de {} s após o fim é alto", e.nome, e.extra_max_segundos), None));
        }
    }

    // Conflitos por dia da semana (para cada perfil).
    let todos_perfis: Vec<String> = if agenda.perfis.is_empty() {
        let mut v: Vec<String> = agenda.eventos.iter().flat_map(|e| e.perfis.clone()).collect();
        v.sort();
        v.dedup();
        v
    } else {
        agenda.perfis.iter().map(|p| p.id.clone()).collect()
    };
    let mut vistos = HashSet::new();
    for perfil in &todos_perfis {
        for dia in 0u8..7 {
            let evs: Vec<&Evento> = agenda
                .eventos
                .iter()
                .filter(|e| e.ativo && e.dias.contains(&dia) && e.perfis.contains(perfil) && e.inicio_seg().is_some())
                .collect();
            for (i, a) in evs.iter().enumerate() {
                for b in evs.iter().skip(i + 1) {
                    let chave = if a.id < b.id { (a.id.clone(), b.id.clone()) } else { (b.id.clone(), a.id.clone()) };
                    if vistos.contains(&chave) {
                        continue;
                    }
                    let (ai, bi) = (a.inicio_seg().unwrap(), b.inicio_seg().unwrap());
                    if ai / 60 == bi / 60 {
                        vistos.insert(chave);
                        let (v, p) = if crate::agendador::prevalece(a, b) { (a, b) } else { (b, a) };
                        let g = if a.prioridade == b.prioridade && a.tipo == b.tipo { Alerta } else { Info };
                        out.push(av(
                            g,
                            Some(p),
                            format!("{} e {} começam juntos às {} ({}): tocará só \"{}\"", a.nome, b.nome, fmt_hm(ai), nome_dia(dia), v.nome),
                            Some(if v.tipo == TipoEvento::Recreio && p.tipo.eh_troca() {
                                "Comportamento padrão: a playlist do recreio substitui a sinalização. Se quiser, adicione uma vinheta de abertura ao recreio".into()
                            } else {
                                "Ajuste a prioridade ou o horário de um deles".into()
                            }),
                        ));
                        continue;
                    }
                    // Sobreposição de janelas, ou evento dentro de janela.
                    let dentro = |j: &Evento, x: u32| j.tipo.eh_janela() && j.fim_seg().map(|f| x > j.inicio_seg().unwrap() && x < f).unwrap_or(false);
                    for (j, o, oi) in [(a, b, bi), (b, a, ai)] {
                        if dentro(j, oi) {
                            vistos.insert(chave.clone());
                            let msg = if crate::agendador::prevalece(j, o) {
                                format!("{} ({}) cai dentro de {} ({}): não tocará, pois {} tem prioridade maior", o.nome, fmt_hm(oi), j.nome, nome_dia(dia), j.nome)
                            } else {
                                format!("{} ({}) cai dentro de {} ({}): vai interromper a janela com fade", o.nome, fmt_hm(oi), j.nome, nome_dia(dia))
                            };
                            out.push(av(Alerta, Some(o), msg, Some(format!("Mova {} para antes de {} ou depois de {}", o.nome, fmt_hm(j.inicio_seg().unwrap()), fmt_hm(j.fim_seg().unwrap())))));
                        }
                    }
                }
            }
        }
    }

    for x in &agenda.excecoes {
        if Data::parse(&x.data).is_none() {
            out.push(av(Erro, None, format!("Data inválida no calendário: {}", x.data), Some("Use AAAA-MM-DD".into())));
        }
        if let Some(p) = &x.perfil {
            if !perfis.is_empty() && !perfis.contains(p.as_str()) {
                out.push(av(Erro, None, format!("Calendário {}: perfil inexistente \"{}\"", x.data, p), None));
            }
        }
    }
    out
}

pub fn tem_erro(avisos: &[Aviso]) -> bool {
    avisos.iter().any(|a| a.gravidade == Gravidade::Erro)
}
