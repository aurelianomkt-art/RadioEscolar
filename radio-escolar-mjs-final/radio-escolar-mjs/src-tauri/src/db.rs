//! Banco local SQLite (arquivo `radio.db` na pasta de dados do aplicativo).

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub type Res<T> = Result<T, String>;

fn e<E: std::fmt::Display>(x: E) -> String {
    x.to_string()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pasta {
    pub id: i64,
    pub caminho: String,
    /// "musica" | "vinheta" | "recado"
    pub categoria: String,
    pub subpastas: bool,
    pub ultima_indexacao: Option<String>,
    pub acessivel: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Faixa {
    pub id: String,
    pub caminho: String,
    pub nome: String,
    pub titulo: Option<String>,
    pub artista: Option<String>,
    pub duracao_ms: Option<u64>,
    pub formato: String,
    pub categoria: String,
    pub pasta_id: i64,
    /// "ok" | "ausente" | "invalido"
    pub status: String,
    pub erro: Option<String>,
    /// Volume individual 0–100 (vinhetas e recados).
    pub volume: u8,
    pub tocada_em: Option<String>,
    pub vezes: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
    pub id: String,
    pub nome: String,
    pub faixas: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Registro {
    pub id: i64,
    pub quando: String,
    /// "agendador" | "manual" | "audio" | "sistema" | "biblioteca" | "config"
    pub origem: String,
    /// "Info" | "Aviso" | "Erro"
    pub nivel: String,
    pub evento_id: Option<String>,
    pub mensagem: String,
}

pub struct Db {
    pub c: Connection,
}

impl Db {
    pub fn abrir(arquivo: &Path) -> Res<Db> {
        let c = Connection::open(arquivo).map_err(e)?;
        c.busy_timeout(std::time::Duration::from_secs(8)).map_err(e)?;
        c.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             PRAGMA foreign_keys=ON;
             CREATE TABLE IF NOT EXISTS config (chave TEXT PRIMARY KEY, valor TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS pastas (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                caminho TEXT NOT NULL UNIQUE,
                categoria TEXT NOT NULL DEFAULT 'musica',
                subpastas INTEGER NOT NULL DEFAULT 1,
                ultima_indexacao TEXT,
                acessivel INTEGER NOT NULL DEFAULT 1);
             CREATE TABLE IF NOT EXISTS faixas (
                id TEXT PRIMARY KEY,
                caminho TEXT NOT NULL UNIQUE,
                nome TEXT NOT NULL,
                titulo TEXT, artista TEXT, duracao_ms INTEGER,
                formato TEXT NOT NULL,
                categoria TEXT NOT NULL,
                pasta_id INTEGER NOT NULL REFERENCES pastas(id) ON DELETE CASCADE,
                status TEXT NOT NULL DEFAULT 'ok',
                erro TEXT,
                volume INTEGER NOT NULL DEFAULT 100,
                tamanho INTEGER, modificado INTEGER);
             CREATE INDEX IF NOT EXISTS ix_faixas_cat ON faixas(categoria, status);
             CREATE TABLE IF NOT EXISTS playlists (id TEXT PRIMARY KEY, nome TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS playlist_itens (
                playlist_id TEXT NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
                faixa_id TEXT NOT NULL,
                ordem INTEGER NOT NULL);
             CREATE TABLE IF NOT EXISTS reproducoes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                quando TEXT NOT NULL,
                faixa_id TEXT NOT NULL,
                origem TEXT NOT NULL);
             CREATE INDEX IF NOT EXISTS ix_repr ON reproducoes(faixa_id, quando);
             CREATE TABLE IF NOT EXISTS historico (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                quando TEXT NOT NULL,
                origem TEXT NOT NULL,
                nivel TEXT NOT NULL,
                evento_id TEXT,
                mensagem TEXT NOT NULL);",
        )
        .map_err(e)?;
        Ok(Db { c })
    }

    // ---------- config ----------
    pub fn ler_config(&self, chave: &str) -> Res<Option<String>> {
        self.c
            .query_row("SELECT valor FROM config WHERE chave=?1", [chave], |r| r.get(0))
            .optional()
            .map_err(e)
    }
    pub fn gravar_config(&self, chave: &str, valor: &str) -> Res<()> {
        self.c
            .execute("INSERT INTO config(chave,valor) VALUES(?1,?2) ON CONFLICT(chave) DO UPDATE SET valor=excluded.valor", params![chave, valor])
            .map(|_| ())
            .map_err(e)
    }

    // ---------- histórico ----------
    pub fn registrar(&self, origem: &str, nivel: &str, evento_id: Option<&str>, msg: &str) {
        let agora = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let _ = self.c.execute(
            "INSERT INTO historico(quando,origem,nivel,evento_id,mensagem) VALUES(?1,?2,?3,?4,?5)",
            params![agora, origem, nivel, evento_id, msg],
        );
    }
    pub fn historico(&self, texto: &str, nivel: &str, data: &str, limite: i64) -> Res<Vec<Registro>> {
        let mut st = self
            .c
            .prepare(
                "SELECT id,quando,origem,nivel,evento_id,mensagem FROM historico
                 WHERE (?1='' OR mensagem LIKE '%'||?1||'%')
                   AND (?2='' OR nivel=?2)
                   AND (?3='' OR quando LIKE ?3||'%')
                 ORDER BY id DESC LIMIT ?4",
            )
            .map_err(e)?;
        let v = st
            .query_map(params![texto, nivel, data, limite], |r| {
                Ok(Registro { id: r.get(0)?, quando: r.get(1)?, origem: r.get(2)?, nivel: r.get(3)?, evento_id: r.get(4)?, mensagem: r.get(5)? })
            })
            .map_err(e)?
            .filter_map(|x| x.ok())
            .collect();
        Ok(v)
    }
    /// Mantém o histórico em tamanho razoável (últimos 180 dias).
    pub fn podar_historico(&self) {
        let _ = self.c.execute("DELETE FROM historico WHERE quando < date('now','localtime','-180 days')", []);
        let _ = self.c.execute("DELETE FROM reproducoes WHERE quando < date('now','localtime','-180 days')", []);
    }

    // ---------- reproduções ----------
    pub fn marcar_reproducao(&self, faixa_id: &str, origem: &str) {
        let agora = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let _ = self.c.execute("INSERT INTO reproducoes(quando,faixa_id,origem) VALUES(?1,?2,?3)", params![agora, faixa_id, origem]);
    }
    /// Ids tocados recentemente (mais novo primeiro).
    pub fn recentes(&self, n: usize) -> Vec<String> {
        let mut st = match self.c.prepare("SELECT faixa_id FROM reproducoes ORDER BY id DESC LIMIT ?1") {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        let r = st.query_map([n as i64], |r| r.get::<_, String>(0));
        let v: Vec<String> = match r {
            Ok(it) => it.filter_map(|x| x.ok()).collect(),
            Err(_) => vec![],
        };
        v
    }

    // ---------- pastas ----------
    pub fn pastas(&self) -> Res<Vec<Pasta>> {
        let mut st = self.c.prepare("SELECT id,caminho,categoria,subpastas,ultima_indexacao,acessivel FROM pastas ORDER BY id").map_err(e)?;
        let v = st
            .query_map([], |r| {
                Ok(Pasta {
                    id: r.get(0)?,
                    caminho: r.get(1)?,
                    categoria: r.get(2)?,
                    subpastas: r.get::<_, i64>(3)? != 0,
                    ultima_indexacao: r.get(4)?,
                    acessivel: r.get::<_, i64>(5)? != 0,
                })
            })
            .map_err(e)?
            .filter_map(|x| x.ok())
            .collect();
        Ok(v)
    }
    pub fn adicionar_pasta(&self, caminho: &str, categoria: &str, subpastas: bool) -> Res<i64> {
        self.c
            .execute(
                "INSERT INTO pastas(caminho,categoria,subpastas) VALUES(?1,?2,?3)
                 ON CONFLICT(caminho) DO UPDATE SET categoria=excluded.categoria, subpastas=excluded.subpastas",
                params![caminho, categoria, subpastas as i64],
            )
            .map_err(e)?;
        self.c.query_row("SELECT id FROM pastas WHERE caminho=?1", [caminho], |r| r.get(0)).map_err(e)
    }
    pub fn remover_pasta(&self, id: i64) -> Res<()> {
        self.c.execute("DELETE FROM faixas WHERE pasta_id=?1", [id]).map_err(e)?;
        self.c.execute("DELETE FROM pastas WHERE id=?1", [id]).map(|_| ()).map_err(e)
    }
    pub fn marcar_pasta(&self, id: i64, acessivel: bool) {
        let agora = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let _ = self.c.execute("UPDATE pastas SET acessivel=?1, ultima_indexacao=?2 WHERE id=?3", params![acessivel as i64, agora, id]);
    }

    // ---------- faixas ----------
    const COLS: &'static str = "f.id,f.caminho,f.nome,f.titulo,f.artista,f.duracao_ms,f.formato,f.categoria,f.pasta_id,f.status,f.erro,f.volume,
        (SELECT MAX(quando) FROM reproducoes r WHERE r.faixa_id=f.id),
        (SELECT COUNT(*) FROM reproducoes r WHERE r.faixa_id=f.id)";

    fn mapear(r: &rusqlite::Row) -> rusqlite::Result<Faixa> {
        Ok(Faixa {
            id: r.get(0)?,
            caminho: r.get(1)?,
            nome: r.get(2)?,
            titulo: r.get(3)?,
            artista: r.get(4)?,
            duracao_ms: r.get::<_, Option<i64>>(5)?.map(|x| x as u64),
            formato: r.get(6)?,
            categoria: r.get(7)?,
            pasta_id: r.get(8)?,
            status: r.get(9)?,
            erro: r.get(10)?,
            volume: r.get::<_, i64>(11)?.clamp(0, 100) as u8,
            tocada_em: r.get(12)?,
            vezes: r.get(13)?,
        })
    }

    pub fn faixas(&self, texto: &str, categoria: &str, status: &str, limite: i64) -> Res<Vec<Faixa>> {
        let sql = format!(
            "SELECT {} FROM faixas f
             WHERE (?1='' OR f.nome LIKE '%'||?1||'%' OR IFNULL(f.titulo,'') LIKE '%'||?1||'%' OR IFNULL(f.artista,'') LIKE '%'||?1||'%' OR f.caminho LIKE '%'||?1||'%')
               AND (?2='' OR f.categoria=?2)
               AND (?3='' OR f.status=?3)
             ORDER BY f.categoria, f.caminho LIMIT ?4",
            Self::COLS
        );
        let mut st = self.c.prepare(&sql).map_err(e)?;
        let v = st.query_map(params![texto, categoria, status, limite], Self::mapear).map_err(e)?.filter_map(|x| x.ok()).collect();
        Ok(v)
    }
    pub fn faixa(&self, id: &str) -> Option<Faixa> {
        let sql = format!("SELECT {} FROM faixas f WHERE f.id=?1", Self::COLS);
        self.c.query_row(&sql, [id], Self::mapear).optional().ok().flatten()
    }
    /// Ids de faixas tocáveis de uma categoria, opcionalmente dentro de uma pasta (prefixo do caminho).
    pub fn ids_tocaveis(&self, categoria: &str, pasta: Option<&str>) -> Vec<String> {
        let pref = pasta.unwrap_or("");
        let mut st = match self.c.prepare(
            "SELECT id FROM faixas WHERE status='ok' AND categoria=?1 AND (?2='' OR caminho LIKE ?2||'%') ORDER BY caminho",
        ) {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        let r = st.query_map(params![categoria, pref], |r| r.get::<_, String>(0));
        let v: Vec<String> = match r {
            Ok(it) => it.filter_map(|x| x.ok()).collect(),
            Err(_) => vec![],
        };
        v
    }
    pub fn conta_tocaveis(&self, categoria: &str) -> i64 {
        self.c
            .query_row("SELECT COUNT(*) FROM faixas WHERE status='ok' AND categoria=?1", [categoria], |r| r.get(0))
            .unwrap_or(0)
    }
    pub fn gravar_faixa(&self, f: &Faixa, tamanho: i64, modificado: i64) -> Res<()> {
        self.c
            .execute(
                "INSERT INTO faixas(id,caminho,nome,titulo,artista,duracao_ms,formato,categoria,pasta_id,status,erro,tamanho,modificado)
                 VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)
                 ON CONFLICT(id) DO UPDATE SET nome=excluded.nome,titulo=excluded.titulo,artista=excluded.artista,
                   duracao_ms=excluded.duracao_ms,formato=excluded.formato,categoria=excluded.categoria,pasta_id=excluded.pasta_id,
                   status=excluded.status,erro=excluded.erro,tamanho=excluded.tamanho,modificado=excluded.modificado",
                params![
                    f.id,
                    f.caminho,
                    f.nome,
                    f.titulo,
                    f.artista,
                    f.duracao_ms.map(|x| x as i64),
                    f.formato,
                    f.categoria,
                    f.pasta_id,
                    f.status,
                    f.erro,
                    tamanho,
                    modificado
                ],
            )
            .map(|_| ())
            .map_err(e)
    }
    /// (tamanho, modificado) já indexados de um caminho, para pular arquivos inalterados.
    pub fn assinatura(&self, id: &str) -> Option<(i64, i64, String)> {
        self.c
            .query_row("SELECT IFNULL(tamanho,-1), IFNULL(modificado,-1), status FROM faixas WHERE id=?1", [id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .optional()
            .ok()
            .flatten()
    }
    pub fn ids_da_pasta(&self, pasta_id: i64) -> Vec<String> {
        let mut st = match self.c.prepare("SELECT id FROM faixas WHERE pasta_id=?1") {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        let r = st.query_map([pasta_id], |r| r.get::<_, String>(0));
        let v: Vec<String> = match r {
            Ok(it) => it.filter_map(|x| x.ok()).collect(),
            Err(_) => vec![],
        };
        v
    }
    pub fn marcar_status(&self, id: &str, status: &str, erro: Option<&str>) {
        let _ = self.c.execute("UPDATE faixas SET status=?1, erro=?2 WHERE id=?3", params![status, erro, id]);
    }
    pub fn definir_categoria(&self, id: &str, categoria: &str) -> Res<()> {
        self.c.execute("UPDATE faixas SET categoria=?1 WHERE id=?2", params![categoria, id]).map(|_| ()).map_err(e)
    }
    pub fn definir_volume_faixa(&self, id: &str, volume: u8) -> Res<()> {
        self.c.execute("UPDATE faixas SET volume=?1 WHERE id=?2", params![volume.min(100) as i64, id]).map(|_| ()).map_err(e)
    }
    pub fn contagem(&self) -> Res<Vec<(String, String, i64)>> {
        let mut st = self.c.prepare("SELECT categoria,status,COUNT(*) FROM faixas GROUP BY categoria,status").map_err(e)?;
        let v = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).map_err(e)?.filter_map(|x| x.ok()).collect();
        Ok(v)
    }

    // ---------- playlists ----------
    pub fn playlists(&self) -> Res<Vec<Playlist>> {
        let mut st = self.c.prepare("SELECT id,nome FROM playlists ORDER BY nome").map_err(e)?;
        let base: Vec<(String, String)> = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?))).map_err(e)?.filter_map(|x| x.ok()).collect();
        let mut out = Vec::new();
        for (id, nome) in base {
            let mut s2 = self.c.prepare("SELECT faixa_id FROM playlist_itens WHERE playlist_id=?1 ORDER BY ordem").map_err(e)?;
            let faixas = s2.query_map([&id], |r| r.get::<_, String>(0)).map_err(e)?.filter_map(|x| x.ok()).collect();
            out.push(Playlist { id, nome, faixas });
        }
        Ok(out)
    }
    pub fn salvar_playlist(&mut self, p: &Playlist) -> Res<()> {
        let tx = self.c.transaction().map_err(e)?;
        tx.execute("INSERT INTO playlists(id,nome) VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET nome=excluded.nome", params![p.id, p.nome]).map_err(e)?;
        tx.execute("DELETE FROM playlist_itens WHERE playlist_id=?1", [&p.id]).map_err(e)?;
        for (i, f) in p.faixas.iter().enumerate() {
            tx.execute("INSERT INTO playlist_itens(playlist_id,faixa_id,ordem) VALUES(?1,?2,?3)", params![p.id, f, i as i64]).map_err(e)?;
        }
        tx.commit().map_err(e)
    }
    pub fn excluir_playlist(&self, id: &str) -> Res<()> {
        self.c.execute("DELETE FROM playlists WHERE id=?1", [id]).map(|_| ()).map_err(e)
    }
    pub fn faixas_playlist(&self, id: &str) -> Vec<String> {
        let mut st = match self.c.prepare(
            "SELECT i.faixa_id FROM playlist_itens i JOIN faixas f ON f.id=i.faixa_id WHERE i.playlist_id=?1 AND f.status='ok' ORDER BY i.ordem",
        ) {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        let r = st.query_map([id], |r| r.get::<_, String>(0));
        let v: Vec<String> = match r {
            Ok(it) => it.filter_map(|x| x.ok()).collect(),
            Err(_) => vec![],
        };
        v
    }
}
