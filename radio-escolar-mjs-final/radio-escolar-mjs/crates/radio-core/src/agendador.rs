//! Agendador determinístico.
//!
//! O processo nativo chama `tick()` a cada ~1 s com o horário local do Windows.
//! O agendador devolve AÇÕES; quem toca áudio é o motor de áudio. Isso permite
//! testar todas as regras sem som e sem relógio real.
//!
//! Regras (na ordem do documento de requisitos):
//! 1. "Silenciar agora" tem prioridade máxima: nada inicia até retomada explícita.
//! 3. Rádio Livre suspende a programação (eventos são registrados como suspensos, não perdidos).
//! 4. No mesmo minuto, vence o evento de maior prioridade (recreio > sinalização).
//! 5. Nunca duas faixas ao mesmo tempo: novo evento interrompe com fade, aguarda ou é ignorado.
//! 6. Eventos perdidos com o PC desligado/suspenso NÃO são tocados ao voltar.
//! 7. Ao voltar dentro de uma janela (recepção/recreio), pode retomar a playlist da janela.

use std::collections::HashSet;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::modelo::{Agenda, Evento, FimJanela, PoliticaConflito, TipoEvento};
use crate::tempo::{fmt_hm, Data, Momento};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Modo {
    Escolar,
    Livre,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "acao", rename_all = "camelCase", rename_all_fields = "camelCase"))]
pub enum Acao {
    /// Iniciar um evento. Se `interromper_atual`, o motor faz fade-out do que toca antes.
    Iniciar {
        evento: Evento,
        /// true quando é retomada de janela após reinício/suspensão/volta do Rádio Livre.
        retomada: bool,
        interromper_atual: bool,
    },
    /// Encerrar a janela ativa (fade-out ou deixar a faixa terminar).
    EncerrarJanela { evento_id: String, modo: FimJanela, fade_ms: u32 },
    /// Registro de histórico (não toca nada).
    Registro { nivel: Nivel, evento_id: Option<String>, mensagem: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Nivel {
    Info,
    Aviso,
    Erro,
}

/// O que fazer quando uma faixa termina.
#[derive(Clone, Debug, PartialEq)]
pub enum Proximo {
    /// Continuar a janela com outra faixa.
    TocarMais(Evento),
    /// Iniciar evento que aguardava.
    IniciarPendente(Evento),
    /// Parar (silêncio).
    Parar,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Ativo {
    pub evento: Evento,
    pub data: Data,
    /// Fim da janela em segundos absolutos (None para sinal/recado).
    pub fim_abs: Option<i64>,
    pub encerrando: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Agendador {
    ultimo: Option<Momento>,
    disparados: HashSet<(Data, String)>,
    pub ativo: Option<Ativo>,
    pendente: Option<(Evento, Data, i64)>,
}

impl Agendador {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ultimo_tick(&self) -> Option<Momento> {
        self.ultimo
    }

    /// Chamado a cada ~1 s.
    pub fn tick(&mut self, agenda: &Agenda, agora: Momento, modo: Modo, silenciado: bool) -> Vec<Acao> {
        let mut acoes = Vec::new();
        self.limpar_disparados(agora.data);

        let ultimo = match self.ultimo {
            None => {
                acoes.extend(self.reavaliar(agenda, agora, modo, silenciado, "Início do aplicativo"));
                return acoes;
            }
            Some(u) => u,
        };

        let delta = agora.abs() - ultimo.abs();
        if delta < 0 {
            acoes.extend(self.reavaliar(
                agenda,
                agora,
                modo,
                silenciado,
                &format!("Relógio do Windows voltou {} s; agenda reavaliada sem repetir eventos", -delta),
            ));
            return acoes;
        }
        if delta as u32 > agenda.salto_segundos {
            acoes.extend(self.reavaliar(
                agenda,
                agora,
                modo,
                silenciado,
                &format!(
                    "Intervalo de {} s sem relógio (suspensão, hibernação ou ajuste de hora). Eventos perdidos NÃO serão tocados",
                    delta
                ),
            ));
            return acoes;
        }

        // 1) Fim de janela ativa.
        acoes.extend(self.verificar_fim_janela(agora));

        // 2) Pendente expirado.
        if let Some((ev, _, exp)) = &self.pendente {
            if agora.abs() > *exp {
                acoes.push(Acao::Registro {
                    nivel: Nivel::Aviso,
                    evento_id: Some(ev.id.clone()),
                    mensagem: format!("{} não tocou: aguardou além do limite", ev.nome),
                });
                self.pendente = None;
            }
        }

        // 3) Eventos que venceram entre o último tick e agora.
        let devidos = self.devidos(agenda, ultimo, agora);
        if devidos.is_empty() {
            self.ultimo = Some(agora);
            return acoes;
        }
        for (d, e) in &devidos {
            self.disparados.insert((*d, e.id.clone()));
        }

        // Atraso além da tolerância (tick muito lento): não toca.
        let devidos: Vec<(Data, Evento)> = devidos
            .into_iter()
            .filter(|(d, e)| {
                let ini = Momento::new(*d, e.inicio_seg().unwrap_or(0)).abs();
                let atraso = agora.abs() - ini;
                if atraso > agenda.tolerancia_segundos as i64 {
                    acoes.push(Acao::Registro {
                        nivel: Nivel::Aviso,
                        evento_id: Some(e.id.clone()),
                        mensagem: format!("{} ignorado: atraso de {} s acima da tolerância", e.nome, atraso),
                    });
                    false
                } else {
                    true
                }
            })
            .collect();
        if devidos.is_empty() {
            self.ultimo = Some(agora);
            return acoes;
        }

        if modo == Modo::Livre {
            for (_, e) in &devidos {
                acoes.push(Acao::Registro {
                    nivel: Nivel::Info,
                    evento_id: Some(e.id.clone()),
                    mensagem: format!("{} ({}) suspenso: Rádio Livre ativo", e.nome, fmt_hm(e.inicio_seg().unwrap_or(0))),
                });
            }
            self.ultimo = Some(agora);
            return acoes;
        }
        if silenciado {
            for (_, e) in &devidos {
                acoes.push(Acao::Registro {
                    nivel: Nivel::Aviso,
                    evento_id: Some(e.id.clone()),
                    mensagem: format!("{} não tocou: rádio silenciada ou sem saída de áudio (aguardando o operador)", e.nome),
                });
            }
            self.ultimo = Some(agora);
            return acoes;
        }

        // Vence o de maior prioridade (empate: janela > pontual, depois ordem de id).
        let mut ordenados = devidos;
        ordenados.sort_by(|(_, a), (_, b)| {
            b.prioridade
                .cmp(&a.prioridade)
                .then(b.tipo.eh_janela().cmp(&a.tipo.eh_janela()))
                .then(a.id.cmp(&b.id))
        });
        let (data_v, vencedor) = ordenados.remove(0);
        for (_, e) in &ordenados {
            acoes.push(Acao::Registro {
                nivel: Nivel::Info,
                evento_id: Some(e.id.clone()),
                mensagem: format!(
                    "{} suprimido: coincide com {} (prioridade maior). Não tocam duas faixas ao mesmo tempo",
                    e.nome, vencedor.nome
                ),
            });
        }

        acoes.extend(self.aplicar_conflito(agenda, data_v, vencedor, agora));
        self.ultimo = Some(agora);
        acoes
    }

    fn aplicar_conflito(&mut self, agenda: &Agenda, data: Data, ev: Evento, agora: Momento) -> Vec<Acao> {
        let mut acoes = Vec::new();
        if let Some(at) = &self.ativo {
            if at.evento.prioridade > ev.prioridade {
                acoes.push(Acao::Registro {
                    nivel: Nivel::Info,
                    evento_id: Some(ev.id.clone()),
                    mensagem: format!("{} não tocou: {} (prioridade maior) está em andamento", ev.nome, at.evento.nome),
                });
                return acoes;
            }
            match ev.conflito {
                PoliticaConflito::Ignorar => {
                    acoes.push(Acao::Registro {
                        nivel: Nivel::Info,
                        evento_id: Some(ev.id.clone()),
                        mensagem: format!("{} ignorado: {} ainda tocando", ev.nome, at.evento.nome),
                    });
                    return acoes;
                }
                PoliticaConflito::Aguardar => {
                    let exp = agora.abs() + agenda.espera_max_segundos as i64;
                    acoes.push(Acao::Registro {
                        nivel: Nivel::Info,
                        evento_id: Some(ev.id.clone()),
                        mensagem: format!("{} aguardando {} terminar", ev.nome, at.evento.nome),
                    });
                    self.pendente = Some((ev, data, exp));
                    return acoes;
                }
                PoliticaConflito::Interromper => {
                    acoes.push(Acao::Registro {
                        nivel: Nivel::Info,
                        evento_id: Some(at.evento.id.clone()),
                        mensagem: format!("{} encerrado com fade para iniciar {}", at.evento.nome, ev.nome),
                    });
                    self.iniciar(data, ev.clone());
                    acoes.push(Acao::Iniciar { evento: ev, retomada: false, interromper_atual: true });
                    return acoes;
                }
            }
        }
        self.iniciar(data, ev.clone());
        acoes.push(Acao::Iniciar { evento: ev, retomada: false, interromper_atual: false });
        acoes
    }

    fn iniciar(&mut self, data: Data, ev: Evento) {
        let fim_abs = if ev.tipo.eh_janela() {
            ev.fim_seg().map(|f| Momento::new(data, f).abs())
        } else {
            None
        };
        self.ativo = Some(Ativo { evento: ev, data, fim_abs, encerrando: false });
    }

    fn verificar_fim_janela(&mut self, agora: Momento) -> Vec<Acao> {
        let mut acoes = Vec::new();
        if let Some(at) = &mut self.ativo {
            if let Some(fim) = at.fim_abs {
                let passou = agora.abs() - fim;
                if passou >= 0 && !at.encerrando {
                    at.encerrando = true;
                    acoes.push(Acao::EncerrarJanela {
                        evento_id: at.evento.id.clone(),
                        modo: at.evento.fim_janela,
                        fade_ms: at.evento.fade_out_ms,
                    });
                    acoes.push(Acao::Registro {
                        nivel: Nivel::Info,
                        evento_id: Some(at.evento.id.clone()),
                        mensagem: match at.evento.fim_janela {
                            FimJanela::FadeOut => format!("{} encerrado no horário (fade-out)", at.evento.nome),
                            FimJanela::TerminarFaixa => {
                                format!("{}: horário final atingido, terminando a faixa atual", at.evento.nome)
                            }
                        },
                    });
                    if at.evento.fim_janela == FimJanela::FadeOut {
                        self.ativo = None;
                    }
                } else if at.encerrando
                    && at.evento.fim_janela == FimJanela::TerminarFaixa
                    && passou >= at.evento.extra_max_segundos as i64
                {
                    acoes.push(Acao::EncerrarJanela {
                        evento_id: at.evento.id.clone(),
                        modo: FimJanela::FadeOut,
                        fade_ms: at.evento.fade_out_ms,
                    });
                    acoes.push(Acao::Registro {
                        nivel: Nivel::Aviso,
                        evento_id: Some(at.evento.id.clone()),
                        mensagem: format!("{}: faixa longa demais, encerrada com fade (teto de segurança)", at.evento.nome),
                    });
                    self.ativo = None;
                }
            }
        }
        acoes
    }

    /// Eventos com início em (ultimo, agora], cobrindo virada de dia.
    fn devidos(&self, agenda: &Agenda, ultimo: Momento, agora: Momento) -> Vec<(Data, Evento)> {
        let mut out = Vec::new();
        let mut d = ultimo.data;
        while d <= agora.data {
            for e in agenda.eventos_do_dia(d) {
                let ini = Momento::new(d, e.inicio_seg().unwrap_or(0));
                if ini > ultimo && ini <= agora && !self.disparados.contains(&(d, e.id.clone())) {
                    out.push((d, e.clone()));
                }
            }
            d = d.somar_dias(1);
        }
        out
    }

    /// Reavaliação após início, suspensão, salto de relógio, volta do Rádio Livre ou retomada.
    /// Nunca toca eventos pontuais vencidos; pode retomar a janela vigente.
    pub fn reavaliar(&mut self, agenda: &Agenda, agora: Momento, modo: Modo, silenciado: bool, motivo: &str) -> Vec<Acao> {
        let mut acoes = vec![Acao::Registro { nivel: Nivel::Info, evento_id: None, mensagem: motivo.to_string() }];
        self.ultimo = Some(agora);
        self.pendente = None;

        // Marca tudo que já começou hoje (e ontem, por segurança) como disparado.
        for d in [agora.data.somar_dias(-1), agora.data] {
            for e in agenda.eventos_do_dia(d) {
                let ini = Momento::new(d, e.inicio_seg().unwrap_or(0));
                if ini <= agora {
                    self.disparados.insert((d, e.id.clone()));
                }
            }
        }

        // Janela ativa que já passou do fim → encerra.
        if let Some(at) = &self.ativo {
            if let Some(f) = at.fim_abs {
                if agora.abs() >= f {
                    acoes.push(Acao::EncerrarJanela { evento_id: at.evento.id.clone(), modo: FimJanela::FadeOut, fade_ms: at.evento.fade_out_ms });
                    self.ativo = None;
                }
            } else {
                // Sinal/recado interrompido por suspensão: não retoma.
                self.ativo = None;
            }
        }

        if modo == Modo::Livre || silenciado {
            self.ativo = None;
            return acoes;
        }
        if self.ativo.is_some() {
            return acoes;
        }
        if let Some((d, e)) = janela_vigente(agenda, agora) {
            if agenda.retomar_janela_apos_reinicio {
                acoes.push(Acao::Registro {
                    nivel: Nivel::Info,
                    evento_id: Some(e.id.clone()),
                    mensagem: format!("Retomando {} (janela vigente até {})", e.nome, e.fim.clone().unwrap_or_default()),
                });
                self.iniciar(d, e.clone());
                acoes.push(Acao::Iniciar { evento: e, retomada: true, interromper_atual: false });
            } else {
                acoes.push(Acao::Registro {
                    nivel: Nivel::Info,
                    evento_id: Some(e.id.clone()),
                    mensagem: format!("{} está em andamento, mas a retomada automática está desativada", e.nome),
                });
            }
        }
        acoes
    }

    /// O motor avisa que a faixa terminou naturalmente.
    pub fn faixa_terminou(&mut self, agora: Momento) -> Proximo {
        if let Some(at) = &self.ativo {
            let continua = at.evento.tipo.eh_janela()
                && !at.encerrando
                && at.fim_abs.map(|f| agora.abs() < f).unwrap_or(false);
            if continua {
                return Proximo::TocarMais(at.evento.clone());
            }
        }
        self.ativo = None;
        if let Some((ev, d, exp)) = self.pendente.take() {
            if agora.abs() <= exp {
                self.iniciar(d, ev.clone());
                return Proximo::IniciarPendente(ev);
            }
        }
        Proximo::Parar
    }

    /// "Silenciar agora": esquece o que estava ativo/pendente. Retomada só por `reavaliar`.
    pub fn silenciar(&mut self) {
        self.ativo = None;
        self.pendente = None;
    }

    /// Entrar no Rádio Livre: a programação fica suspensa, sem perder a agenda.
    pub fn entrar_livre(&mut self) {
        self.ativo = None;
        self.pendente = None;
    }

    fn limpar_disparados(&mut self, hoje: Data) {
        let limite = hoje.somar_dias(-2);
        self.disparados.retain(|(d, _)| *d >= limite);
    }
}

/// Janela (recepção/recreio) em andamento neste instante, se houver.
pub fn janela_vigente(agenda: &Agenda, agora: Momento) -> Option<(Data, Evento)> {
    let mut melhor: Option<(Data, Evento)> = None;
    for e in agenda.eventos_do_dia(agora.data) {
        if !e.tipo.eh_janela() {
            continue;
        }
        let (Some(i), Some(f)) = (e.inicio_seg(), e.fim_seg()) else { continue };
        if i <= agora.seg && agora.seg < f {
            let melhor_prio = melhor.as_ref().map(|(_, m)| m.prioridade).unwrap_or(i32::MIN);
            if e.prioridade > melhor_prio {
                melhor = Some((agora.data, e.clone()));
            }
        }
    }
    melhor
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(rename_all = "camelCase"))]
pub struct ItemAgenda {
    pub evento_id: String,
    pub nome: String,
    pub tipo: TipoEvento,
    pub data: String,
    pub inicio: String,
    pub fim: Option<String>,
    /// Segundos até o início (negativo = já começou).
    pub em_segundos: i64,
    /// Se for suprimido por conflito no mesmo minuto, nome do evento vencedor.
    pub suprimido_por: Option<String>,
}

/// Próximos `n` eventos a partir de agora (até 14 dias à frente).
pub fn proximos(agenda: &Agenda, agora: Momento, n: usize) -> Vec<ItemAgenda> {
    let mut out = Vec::new();
    for k in 0..14 {
        let d = agora.data.somar_dias(k);
        for it in agenda_do_dia(agenda, d, agora) {
            if it.em_segundos > 0 {
                out.push(it);
                if out.len() >= n {
                    return out;
                }
            }
        }
    }
    out
}

/// Agenda completa de um dia, com indicação de conflitos.
pub fn agenda_do_dia(agenda: &Agenda, data: Data, agora: Momento) -> Vec<ItemAgenda> {
    let evs = agenda.eventos_do_dia_todos(data);
    let iso = data.to_string();
    evs.iter()
        .map(|e| {
            let ini = e.inicio_seg().unwrap_or(0);
            let vencedor = evs
                .iter()
                .filter(|o| o.id != e.id && o.inicio_seg().map(|x| x / 60) == Some(ini / 60))
                .filter(|o| {
                    o.prioridade > e.prioridade
                        || (o.prioridade == e.prioridade && o.tipo.eh_janela() && !e.tipo.eh_janela())
                        || (o.prioridade == e.prioridade && o.tipo.eh_janela() == e.tipo.eh_janela() && o.id < e.id)
                })
                .max_by_key(|o| o.prioridade)
                .map(|o| o.nome.clone());
            ItemAgenda {
                evento_id: e.id.clone(),
                nome: e.nome.clone(),
                tipo: e.tipo,
                data: data.to_string(),
                inicio: fmt_hm(ini),
                fim: e.fim_seg().map(fmt_hm),
                em_segundos: Momento::new(data, ini).abs() - agora.abs(),
                suprimido_por: if agenda.desligado_em(&iso, &e.id) { Some("desligado neste dia".into()) } else { vencedor },
            }
        })
        .collect()
}

/// Atalho usado nos testes e na tela: um recreio deve prevalecer sobre sinal no mesmo minuto.
pub fn prevalece(a: &Evento, b: &Evento) -> bool {
    a.prioridade > b.prioridade || (a.prioridade == b.prioridade && a.tipo.eh_janela() && !b.tipo.eh_janela())
}

