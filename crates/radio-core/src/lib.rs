//! Núcleo da Rádio Escolar Mestre João Silvério.
//!
//! Sem dependências obrigatórias: regras de agenda, prioridade, calendário,
//! seleção de faixas e validação. Todo o comportamento crítico está aqui
//! e é coberto por testes automatizados (`cargo test -p radio-core`).

pub mod agendador;
pub mod modelo;
pub mod padrao;
pub mod selecao;
pub mod tempo;
pub mod validacao;

pub use agendador::{agenda_do_dia, janela_vigente, proximos, Acao, Agendador, ItemAgenda, Modo, Nivel, Proximo};
pub use modelo::*;
pub use padrao::{agenda_padrao, perfis_padrao};
pub use selecao::{escolher_faixa, Rng};
pub use tempo::{fmt_hm, fmt_hms, parse_hora, Data, Momento};
pub use validacao::{tem_erro, validar, Aviso, Gravidade};

#[cfg(test)]
mod testes;
