import React, { useEffect, useState } from "react";
import { LuListPlus, LuPlay, LuPlus, LuRepeat, LuRepeat1, LuSearch, LuShuffle, LuTrash2 } from "react-icons/lu";
import Player from "../components/Player";
import Fila from "../components/FilaResumo";
import { chamar, fmtTempo } from "../api";
import type { Faixa, Playlist, Repetir } from "../tipos";
import { Cabecalho, useApp } from "../ui";

export default function RadioLivre() {
  const { painel, toast, trocarModo, confirmar } = useApp();
  const [aba, setAba] = useState("musica");
  const [texto, setTexto] = useState("");
  const [faixas, setFaixas] = useState<Faixa[]>([]);
  const [playlists, setPlaylists] = useState<Playlist[]>([]);

  useEffect(() => {
    if (aba === "playlists") chamar<Playlist[]>("listar_playlists").then(setPlaylists);
    else chamar<Faixa[]>("listar_faixas", { texto, categoria: aba, status: "ok", limite: 400 }).then(setFaixas);
  }, [aba, texto]);

  const livre = painel?.modo === "Livre";
  const repetir = painel?.repetir ?? "Nao";
  const prox: Record<Repetir, Repetir> = { Nao: "Fila", Fila: "Faixa", Faixa: "Nao" };
  const add = (ids: string[], tocar = false) =>
    chamar("fila_adicionar", { ids, tocar })
      .then(() => toast(tocar ? "Tocando agora" : `${ids.length} item(ns) adicionado(s) à fila`, "ok"))
      .catch((e) => toast(e.message, "erro"));

  return (
    <div>
      <Cabecalho titulo="Rádio Livre">
        {!livre && (
          <button className="btn acao grande" onClick={() => trocarModo("Livre")}>
            Ativar Rádio Livre
          </button>
        )}
      </Cabecalho>
      {!livre && <div className="aviso Alerta" style={{ marginBottom: 12 }}>O Rádio Livre não está ativo. Você pode montar a fila agora; para tocar, ative o modo (a programação automática ficará suspensa).</div>}
      <div className="grade d2">
        <div className="grade" style={{ alignContent: "start" }}>
          <Player />
          <div className="cartao">
            <div className="flex" style={{ marginBottom: 10 }}>
              <h2 style={{ margin: 0 }}>Fila</h2>
              <span className="espaco" />
              <button className={"btn" + (repetir !== "Nao" ? " amarelo" : "")} onClick={() => chamar("definir_repetir", { repetir: prox[repetir] })} title="Repetição">
                {repetir === "Faixa" ? <LuRepeat1 /> : <LuRepeat />} {repetir === "Nao" ? "Sem repetir" : repetir === "Fila" ? "Repetir fila" : "Repetir faixa"}
              </button>
              <button className={"btn" + (painel?.aleatorio ? " amarelo" : "")} onClick={() => chamar("definir_aleatorio", { aleatorio: !painel?.aleatorio })}>
                <LuShuffle /> Aleatório {painel?.aleatorio ? "ligado" : "desligado"}
              </button>
              <button
                className="btn perigo"
                onClick={async () => {
                  if (await confirmar("Limpar a fila?", "Todos os itens serão retirados da fila (os arquivos não são apagados).", "Limpar")) chamar("fila_limpar");
                }}
              >
                <LuTrash2 /> Limpar
              </button>
            </div>
            <Fila />
          </div>
        </div>
        <div className="cartao">
          <h2>Adicionar à fila</h2>
          <div className="abas" role="tablist">
            {[
              ["musica", "Músicas"],
              ["vinheta", "Vinhetas"],
              ["recado", "Recados"],
              ["playlists", "Playlists"],
            ].map(([k, n]) => (
              <button key={k} className={aba === k ? "on" : ""} onClick={() => setAba(k)} role="tab" aria-selected={aba === k}>
                {n}
              </button>
            ))}
          </div>
          {aba !== "playlists" ? (
            <>
              <div className="flex" style={{ marginBottom: 8 }}>
                <LuSearch />
                <input type="search" placeholder="Buscar por nome, artista…" value={texto} onChange={(e) => setTexto(e.target.value)} style={{ flex: 1 }} />
                <button className="btn" onClick={() => add(faixas.map((f) => f.id))} disabled={!faixas.length}>
                  <LuListPlus /> Todas ({faixas.length})
                </button>
              </div>
              <div className="rolagem">
                <table className="tabela">
                  <tbody>
                    {faixas.map((f) => (
                      <tr key={f.id}>
                        <td className="cortar">
                          <b>{f.titulo || f.nome}</b>
                          <div className="pequeno muted">{f.artista || f.formato.toUpperCase()}</div>
                        </td>
                        <td className="pequeno muted">{fmtTempo(f.duracaoMs)}</td>
                        <td style={{ whiteSpace: "nowrap", textAlign: "right" }}>
                          <button className="btn icone" onClick={() => add([f.id])} aria-label="Adicionar à fila">
                            <LuPlus />
                          </button>{" "}
                          <button className="btn icone acao" onClick={() => add([f.id], true)} disabled={!livre} aria-label="Tocar agora" title={livre ? "Tocar agora" : "Ative o Rádio Livre"}>
                            <LuPlay />
                          </button>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
                {faixas.length === 0 && <div className="vazio">Nada encontrado. Adicione pastas na Biblioteca.</div>}
              </div>
            </>
          ) : (
            <div className="lista">
              {playlists.map((p) => (
                <div key={p.id} className="linha-item">
                  <LuListPlus />
                  <span>
                    <b>{p.nome}</b> <span className="muted pequeno">· {p.faixas.length} faixa(s)</span>
                  </span>
                  <span className="flex">
                    <button className="btn" onClick={() => chamar("fila_adicionar_playlist", { id: p.id, substituir: false }).then((n) => toast(`${n} faixa(s) adicionadas`, "ok"))}>
                      Adicionar
                    </button>
                    <button className="btn" onClick={() => chamar("fila_adicionar_playlist", { id: p.id, substituir: true }).then((n) => toast(`Fila substituída (${n})`, "ok"))}>
                      Substituir fila
                    </button>
                  </span>
                </div>
              ))}
              {playlists.length === 0 && <div className="vazio">Nenhuma playlist. Crie na Biblioteca.</div>}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
