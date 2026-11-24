//! Indexação das pastas locais. Nunca move, renomeia ou apaga arquivos originais.

use crate::db::{Db, Faixa, Pasta};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::time::UNIX_EPOCH;

pub const EXTENSOES: [&str; 5] = ["mp3", "wav", "ogg", "flac", "oga"];

pub fn id_do_caminho(caminho: &str) -> String {
    let mut h = Sha256::new();
    h.update(caminho.to_lowercase().replace('/', "\\").as_bytes());
    let r = h.finalize();
    r.iter().take(10).map(|b| format!("{:02x}", b)).collect()
}

#[derive(Debug, Default, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumoIndexacao {
    pub pastas: usize,
    pub pastas_inacessiveis: Vec<String>,
    pub novas_ou_alteradas: usize,
    pub inalteradas: usize,
    pub ausentes: usize,
    pub invalidas: usize,
}

struct Meta {
    titulo: Option<String>,
    artista: Option<String>,
    duracao_ms: Option<u64>,
}

fn ler_meta(p: &Path) -> Result<Meta, String> {
    use lofty::prelude::*;
    match lofty::read_from_path(p) {
        Ok(t) => {
            let dur = t.properties().duration().as_millis() as u64;
            let tag = t.primary_tag().or_else(|| t.first_tag());
            Ok(Meta {
                titulo: tag.and_then(|x| x.title().map(|s| s.to_string())).filter(|s| !s.trim().is_empty()),
                artista: tag.and_then(|x| x.artist().map(|s| s.to_string())).filter(|s| !s.trim().is_empty()),
                duracao_ms: if dur > 0 { Some(dur) } else { None },
            })
        }
        Err(err) => {
            // Metadados ilegíveis não significam áudio inválido: tenta decodificar.
            let f = std::fs::File::open(p).map_err(|x| format!("Não foi possível abrir: {}", x))?;
            let d = rodio::Decoder::new(std::io::BufReader::new(f)).map_err(|x| format!("Formato não reconhecido ({}; {})", err, x))?;
            use rodio::Source;
            Ok(Meta { titulo: None, artista: None, duracao_ms: d.total_duration().map(|x| x.as_millis() as u64) })
        }
    }
}

/// Indexa uma pasta. Faixas que sumiram ficam marcadas como "ausente" (não são apagadas do histórico).
pub fn indexar_pasta(db: &Db, pasta: &Pasta, resumo: &mut ResumoIndexacao) {
    resumo.pastas += 1;
    let raiz = Path::new(&pasta.caminho);
    if !raiz.is_dir() {
        db.marcar_pasta(pasta.id, false);
        resumo.pastas_inacessiveis.push(pasta.caminho.clone());
        for id in db.ids_da_pasta(pasta.id) {
            db.marcar_status(&id, "ausente", Some("Pasta inacessível ou removida"));
        }
        db.registrar("biblioteca", "Aviso", None, &format!("Pasta inacessível: {}", pasta.caminho));
        return;
    }
    let mut vistos = std::collections::HashSet::new();
    let profundidade = if pasta.subpastas { usize::MAX } else { 1 };
    for ent in walkdir::WalkDir::new(raiz).max_depth(profundidade).follow_links(false).into_iter().filter_map(|x| x.ok()) {
        if !ent.file_type().is_file() {
            continue;
        }
        let p = ent.path();
        let ext = p.extension().and_then(|x| x.to_str()).map(|x| x.to_lowercase()).unwrap_or_default();
        if !EXTENSOES.contains(&ext.as_str()) {
            continue;
        }
        let caminho = p.to_string_lossy().to_string();
        let id = id_do_caminho(&caminho);
        vistos.insert(id.clone());
        let md = ent.metadata().ok();
        let tamanho = md.as_ref().map(|m| m.len() as i64).unwrap_or(-1);
        let modificado = md
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(-1);
        if let Some((t, m, st)) = db.assinatura(&id) {
            if t == tamanho && m == modificado && st != "ausente" {
                resumo.inalteradas += 1;
                continue;
            }
        }
        let nome = p.file_stem().map(|x| x.to_string_lossy().to_string()).unwrap_or_else(|| caminho.clone());
        let (status, erro, meta) = match ler_meta(p) {
            Ok(m) => ("ok", None, m),
            Err(x) => {
                resumo.invalidas += 1;
                ("invalido", Some(x), Meta { titulo: None, artista: None, duracao_ms: None })
            }
        };
        let f = Faixa {
            id,
            caminho,
            nome,
            titulo: meta.titulo,
            artista: meta.artista,
            duracao_ms: meta.duracao_ms,
            formato: if ext == "oga" { "ogg".into() } else { ext },
            categoria: pasta.categoria.clone(),
            pasta_id: pasta.id,
            status: status.into(),
            erro,
            volume: 100,
            tocada_em: None,
            vezes: 0,
        };
        if let Err(x) = db.gravar_faixa(&f, tamanho, modificado) {
            db.registrar("biblioteca", "Erro", None, &format!("Falha ao gravar {}: {}", f.caminho, x));
        } else {
            resumo.novas_ou_alteradas += 1;
        }
    }
    for id in db.ids_da_pasta(pasta.id) {
        if !vistos.contains(&id) {
            if let Some(f) = db.faixa(&id) {
                if f.status != "ausente" {
                    db.marcar_status(&id, "ausente", Some("Arquivo removido, renomeado ou movido"));
                    resumo.ausentes += 1;
                }
            }
        }
    }
    db.marcar_pasta(pasta.id, true);
}

pub fn indexar_tudo(db: &Db) -> ResumoIndexacao {
    let mut r = ResumoIndexacao::default();
    if let Ok(ps) = db.pastas() {
        for p in ps {
            indexar_pasta(db, &p, &mut r);
        }
    }
    r
}
