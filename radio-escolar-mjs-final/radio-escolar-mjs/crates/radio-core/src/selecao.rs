//! Escolha de faixas: aleatória sem repetição recente, sequencial ou fixa.

use crate::modelo::ModoSelecao;

/// Gerador pseudoaleatório simples (xorshift64*), suficiente para sortear músicas.
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(semente: u64) -> Rng {
        Rng(if semente == 0 { 0x9E37_79B9_7F4A_7C15 } else { semente })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn abaixo(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
}

/// Escolhe a próxima faixa.
///
/// * `candidatas`: ids disponíveis (já sem arquivos ausentes/inválidos).
/// * `recentes`: ids tocados recentemente, do mais novo para o mais antigo.
/// * `evitar_ultimas`: quantas faixas recentes evitar (reduzido automaticamente
///   se a lista for pequena, para sempre haver opção).
/// * `cursor`: posição para o modo sequencial (atualizado).
pub fn escolher_faixa(
    candidatas: &[String],
    recentes: &[String],
    evitar_ultimas: usize,
    modo: ModoSelecao,
    faixa_fixa: Option<&str>,
    cursor: &mut usize,
    rng: &mut Rng,
) -> Option<String> {
    match modo {
        ModoSelecao::FaixaFixa => {
            let f = faixa_fixa?;
            candidatas.iter().find(|c| c.as_str() == f).cloned()
        }
        ModoSelecao::Sequencial => {
            if candidatas.is_empty() {
                return None;
            }
            let i = *cursor % candidatas.len();
            *cursor = i + 1;
            Some(candidatas[i].clone())
        }
        ModoSelecao::Aleatorio => {
            if candidatas.is_empty() {
                return None;
            }
            let evitar = evitar_ultimas.min(candidatas.len().saturating_sub(1));
            let bloqueadas: Vec<&String> = recentes.iter().take(evitar).collect();
            let livres: Vec<&String> = candidatas.iter().filter(|c| !bloqueadas.contains(c)).collect();
            if livres.is_empty() {
                return Some(candidatas[rng.abaixo(candidatas.len())].clone());
            }
            Some(livres[rng.abaixo(livres.len())].clone())
        }
    }
}
