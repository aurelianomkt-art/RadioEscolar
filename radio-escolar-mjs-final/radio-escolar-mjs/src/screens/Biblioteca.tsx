import React, { useEffect, useState } from "react";
import { LuChevronDown, LuChevronUp, LuFolderPlus, LuListPlus, LuRefreshCw, LuSearch, LuTrash2, LuPlus, LuSave } from "react-icons/lu";
import { chamar, escolherPasta, fmtTempo, ouvir } from "../api";
import type { Faixa, Pasta, Playlist } from "../tipos";
import OrdemPlaylist from "../components/OrdemPlaylist";
import { CATEGORIAS } from "../tipos";
import { Cabecalho, Campo, Checa, Modal, useApp } from "../ui";

export default function Biblioteca() {
  const { toast, confirmar, painel, config } = useApp();
  const [aba, setAba] = useState("faixas");
  const [pastas, setPastas] = useState<Pasta[]>([]);
  const [faixas, setFaixas] = useState<Faixa[]>([]);
  const [texto, setTexto] = useState("");
  const [cat, setCat] = useState("");
  const [status, setStatus] = useState("");
  const [sel, setSel] = useState<Set<string>>(new Set());
  const [novaPasta, setNovaPasta] = useState<{ caminho: string; categoria: string; subpastas: boolean } | null>(null);
  const [playlists, setPlaylists] = useState<Playlist[]>([]);
  const [editPl, setEditPl] = useState<Playlist | null>(null);

  const carregar = () => {
    chamar<Pasta[]>("listar_pastas").then(setPastas);
    chamar<Faixa[]>("listar_faixas", { texto, categoria: cat, status, limite: 3000 }).then(setFaixas);
    chamar<Playlist[]>("listar_playlists").then(setPlaylists);
  };
  useEffect(carregar, [texto, cat, status]);
  useEffect(() => {
    let off: any;
    ouvir("indexacao", () => carregar()).then((f) => (off = f));
    return () => off && off();
  }, [texto, cat, status]);

  const nomeFaixa = (id: string) => {
    const f = faixas.find((x) => x.id === id);
    return f ? f.titulo || f.nome : id;
  };

  return (
    <div>
      <Cabecalho titulo="Biblioteca local">
        <button
          className="btn acao"
          onClick={async () => {
            const c = await escolherPasta("Escolha a pasta com os áudios");
            if (c) setNovaPasta({ caminho: c, categoria: "musica", subpastas: config?.ajustes.indexarSubpastas ?? true });
          }}
        >
          <LuFolderPlus /> Adicionar pasta
        </button>
        <button className="btn" disabled={painel?.indexando} onClick={() => chamar("reindexar").then(() => toast("Reindexando em segundo plano…"))}>
          <LuRefreshCw /> {painel?.indexando ? "Indexando…" : "Reindexar agora"}
        </button>
      </Cabecalho>

      <div className="abas">
        {[
          ["faixas", `Arquivos (${faixas.length})`],
          ["pastas", `Pastas (${pastas.length})`],
          ["playlists", `Playlists (${playlists.length})`],
        ].map(([k, n]) => (
          <button key={k} className={aba === k ? "on" : ""} onClick={() => setAba(k)}>
            {n}
          </button>
        ))}
      </div>

      {aba === "pastas" && (
        <div className="cartao">
          <p className="muted pequeno">Os arquivos originais nunca são movidos, renomeados ou apagados. Cada pasta tem uma categoria: Músicas, Vinhetas ou Recados.</p>
          <div className="lista">
            {pastas.length === 0 && <div className="vazio">Nenhuma pasta. Clique em "Adicionar pasta".</div>}
            {pastas.map((p) => (
              <div key={p.id} className="linha-item">
                <span className={"chip " + (p.acessivel ? "ok" : "erro")}>{p.acessivel ? "OK" : "Inacessível"}</span>
                <span style={{ minWidth: 0 }}>
                  <b style={{ wordBreak: "break-all" }}>{p.caminho}</b>
                  <div className="pequeno muted">
                    {CATEGORIAS[p.categoria]} · {p.subpastas ? "inclui subpastas" : "só esta pasta"} · última indexação: {p.ultimaIndexacao ?? "nunca"}
                  </div>
                </span>
                <button
                  className="btn perigo"
                  onClick={async () => {
                    if (await confirmar("Remover pasta da biblioteca?", "Os arquivos continuam no computador; só deixam de aparecer na rádio.", "Remover"))
                      chamar("remover_pasta", { id: p.id }).then(carregar).catch((e) => toast(e.message, "erro"));
                  }}
                >
                  <LuTrash2 /> Remover
                </button>
              </div>
            ))}
          </div>
        </div>
      )}

      {aba === "faixas" && (
        <div className="cartao">
          <div className="flex" style={{ marginBottom: 10 }}>
            <LuSearch />
            <input type="search" placeholder="Buscar nome, título, artista ou pasta" value={texto} onChange={(e) => setTexto(e.target.value)} style={{ flex: 1, minWidth: "14rem" }} />
            <select value={cat} onChange={(e) => setCat(e.target.value)} aria-label="Categoria">
              <option value="">Todas as categorias</option>
              <option value="musica">Músicas</option>
              <option value="vinheta">Vinhetas</option>
              <option value="recado">Recados</option>
            </select>
            <select value={status} onChange={(e) => setStatus(e.target.value)} aria-label="Situação">
              <option value="">Todas as situações</option>
              <option value="ok">OK</option>
              <option value="ausente">Ausentes</option>
              <option value="invalido">Inválidos</option>
            </select>
            <button className="btn" disabled={!sel.size} onClick={() => setEditPl({ id: "", nome: "", faixas: [...sel] })}>
              <LuListPlus /> Nova playlist com {sel.size}
            </button>
            <button className="btn" disabled={!sel.size} onClick={() => chamar("fila_adicionar", { ids: [...sel], tocar: false }).then(() => toast(`${sel.size} item(ns) na fila do Rádio Livre`, "ok"))}>
              <LuPlus /> Fila do Rádio Livre
            </button>
          </div>
          <div className="rolagem" style={{ maxHeight: "58vh" }}>
            <table className="tabela">
              <thead>
                <tr>
                  <th>
                    <input
                      type="checkbox"
                      aria-label="Selecionar todos"
                      checked={sel.size > 0 && sel.size === faixas.filter((f) => f.status === "ok").length}
                      onChange={(e) => setSel(e.target.checked ? new Set(faixas.filter((f) => f.status === "ok").map((f) => f.id)) : new Set())}
                    />
                  </th>
                  <th>Nome / metadados</th>
                  <th>Duração</th>
                  <th>Formato</th>
                  <th>Categoria</th>
                  <th>Localização</th>
                  <th>Situação</th>
                </tr>
              </thead>
              <tbody>
                {faixas.map((f) => (
                  <tr key={f.id} className={sel.has(f.id) ? "sel" : ""}>
                    <td>
                      <input
                        type="checkbox"
                        disabled={f.status !== "ok"}
                        checked={sel.has(f.id)}
                        onChange={(e) => {
                          const s = new Set(sel);
                          e.target.checked ? s.add(f.id) : s.delete(f.id);
                          setSel(s);
                        }}
                        aria-label={`Selecionar ${f.nome}`}
                      />
                    </td>
                    <td className="cortar">
                      <b>{f.titulo || f.nome}</b>
                      <div className="pequeno muted">
                        {f.artista ?? "—"} · tocada {f.vezes}x{f.tocadaEm ? ` · última ${f.tocadaEm}` : ""}
                      </div>
                    </td>
                    <td>{fmtTempo(f.duracaoMs)}</td>
                    <td>{f.formato.toUpperCase()}</td>
                    <td>
                      <select value={f.categoria} onChange={(e) => chamar("definir_categoria_faixa", { id: f.id, categoria: e.target.value }).then(carregar)} aria-label="Categoria do arquivo">
                        <option value="musica">Música</option>
                        <option value="vinheta">Vinheta</option>
                        <option value="recado">Recado</option>
                      </select>
                    </td>
                    <td className="cortar pequeno muted" title={f.caminho}>
                      {f.caminho}
                    </td>
                    <td>
                      <span className={"chip " + (f.status === "ok" ? "ok" : "erro")} title={f.erro ?? ""}>
                        {f.status === "ok" ? "OK" : f.status === "ausente" ? "Ausente" : "Inválido"}
                      </span>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
            {faixas.length === 0 && <div className="vazio">Nenhum arquivo encontrado com esses filtros.</div>}
          </div>
          <p className="pequeno muted">Formatos: MP3, WAV, OGG e FLAC. Arquivos ausentes ou inválidos são pulados automaticamente e registrados no Histórico.</p>
        </div>
      )}

      {aba === "playlists" && (
        <div className="cartao">
          <div className="flex" style={{ marginBottom: 10 }}>
            <button className="btn primario" onClick={() => setEditPl({ id: "", nome: "", faixas: [] })}>
              <LuPlus /> Nova playlist
            </button>
            <span className="muted pequeno">Dica: na aba Arquivos, selecione músicas e use "Nova playlist com…".</span>
          </div>
          <div className="lista">
            {playlists.length === 0 && <div className="vazio">Nenhuma playlist.</div>}
            {playlists.map((p) => (
              <div key={p.id} className="linha-item">
                <LuListPlus />
                <span>
                  <b>{p.nome}</b> <span className="muted pequeno">· {p.faixas.length} faixa(s)</span>
                </span>
                <span className="flex">
                  <button className="btn" onClick={() => setEditPl(structuredClone(p))}>
                    Editar
                  </button>
                  <button
                    className="btn perigo"
                    onClick={async () => {
                      if (await confirmar("Excluir playlist?", `"${p.nome}" será excluída (as músicas não são apagadas).`, "Excluir")) chamar("excluir_playlist", { id: p.id }).then(carregar);
                    }}
                  >
                    <LuTrash2 />
                  </button>
                </span>
              </div>
            ))}
          </div>
        </div>
      )}

      {novaPasta && (
        <Modal
          titulo="Adicionar pasta"
          fechar={() => setNovaPasta(null)}
          rodape={
            <>
              <button className="btn" onClick={() => setNovaPasta(null)}>
                Cancelar
              </button>
              <button
                className="btn primario"
                onClick={() =>
                  chamar("adicionar_pasta", novaPasta)
                    .then(() => {
                      toast("Pasta adicionada. Indexando em segundo plano…", "ok");
                      setNovaPasta(null);
                      carregar();
                    })
                    .catch((e) => toast(e.message, "erro"))
                }
              >
                Adicionar
              </button>
            </>
          }
        >
          <p style={{ wordBreak: "break-all" }}>
            <b>{novaPasta.caminho}</b>
          </p>
          <div className="campos">
            <Campo rotulo="Os arquivos desta pasta são">
              <select value={novaPasta.categoria} onChange={(e) => setNovaPasta({ ...novaPasta, categoria: e.target.value })}>
                <option value="musica">Músicas</option>
                <option value="vinheta">Vinhetas</option>
                <option value="recado">Recados</option>
              </select>
            </Campo>
            <div className="campo">
              <span className="rotulo">Subpastas</span>
              <Checa v={novaPasta.subpastas} onChange={(b) => setNovaPasta({ ...novaPasta, subpastas: b })}>
                Incluir subpastas
              </Checa>
            </div>
          </div>
        </Modal>
      )}

      {editPl && (
        <Modal
          titulo={editPl.id ? "Editar playlist" : "Nova playlist"}
          fechar={() => setEditPl(null)}
          rodape={
            <>
              <button className="btn" onClick={() => setEditPl(null)}>
                Cancelar
              </button>
              <button
                className="btn primario"
                onClick={() =>
                  chamar("salvar_playlist", { playlist: editPl })
                    .then(() => {
                      toast("Playlist salva", "ok");
                      setEditPl(null);
                      setSel(new Set());
                      carregar();
                    })
                    .catch((e) => toast(e.message, "erro"))
                }
              >
                <LuSave /> Salvar
              </button>
            </>
          }
        >
          <Campo rotulo="Nome da playlist">
            <input type="text" autoFocus value={editPl.nome} onChange={(e) => setEditPl({ ...editPl, nome: e.target.value })} placeholder="Ex.: Recreio calmo" />
          </Campo>
          <Campo rotulo="Ordem das músicas" ajuda="Arraste para mudar a ordem. Vale quando a seleção for Sequencial.">
            <OrdemPlaylist ids={editPl.faixas} onChange={(faixas) => setEditPl({ ...editPl, faixas })} />
          </Campo>
          <h3>Faixas ({editPl.faixas.length})</h3>
          <div className="lista" style={{ maxHeight: "40vh", overflow: "auto" }}>
            {editPl.faixas.map((id, i) => (
              <div key={id + i} className="linha-item">
                <span className="hora-item">{i + 1}</span>
                <span className="cortar">{nomeFaixa(id)}</span>
                <span className="flex" style={{ flexWrap: "nowrap" }}>
                  <button className="btn icone" disabled={i === 0} onClick={() => { const f = [...editPl.faixas]; [f[i - 1], f[i]] = [f[i], f[i - 1]]; setEditPl({ ...editPl, faixas: f }); }} aria-label="Subir">
                    <LuChevronUp />
                  </button>
                  <button className="btn icone" disabled={i === editPl.faixas.length - 1} onClick={() => { const f = [...editPl.faixas]; [f[i + 1], f[i]] = [f[i], f[i + 1]]; setEditPl({ ...editPl, faixas: f }); }} aria-label="Descer">
                    <LuChevronDown />
                  </button>
                  <button className="btn icone perigo" onClick={() => setEditPl({ ...editPl, faixas: editPl.faixas.filter((_, j) => j !== i) })} aria-label="Remover">
                    <LuTrash2 />
                  </button>
                </span>
              </div>
            ))}
            {editPl.faixas.length === 0 && <div className="vazio">Selecione músicas na aba Arquivos para incluí-las.</div>}
          </div>
          {sel.size > 0 && (
            <button className="btn" style={{ marginTop: 8 }} onClick={() => setEditPl({ ...editPl, faixas: [...editPl.faixas, ...[...sel].filter((x) => !editPl.faixas.includes(x))] })}>
              <LuPlus /> Incluir {sel.size} selecionada(s)
            </button>
          )}
        </Modal>
      )}
    </div>
  );
}
