import React, { useCallback, useEffect, useState } from "react";
import { LuChevronDown, LuChevronUp, LuGripVertical, LuPlay, LuTrash2 } from "react-icons/lu";
import { chamar, fmtTempo, ouvir } from "../api";
import type { ItemFilaView } from "../tipos";
import { CATEGORIAS } from "../tipos";
import { useApp } from "../ui";

/** Fila do Rádio Livre com arrastar-e-soltar (e botões ↑/↓ para quem usa teclado). */
export default function Fila({ compacta }: { compacta?: boolean }) {
  const { painel, toast } = useApp();
  const [fila, setFila] = useState<ItemFilaView[]>([]);
  const [arrastando, setArrastando] = useState<number | null>(null);
  const [alvo, setAlvo] = useState<number | null>(null);

  const carregar = useCallback(() => chamar<ItemFilaView[]>("fila_obter").then(setFila).catch(() => {}), []);
  useEffect(() => {
    carregar();
    let off: any;
    ouvir("fila", () => carregar()).then((f) => (off = f));
    return () => off && off();
  }, [carregar]);

  const reordenar = async (nova: ItemFilaView[]) => {
    setFila(nova);
    await chamar("fila_reordenar", { uids: nova.map((x) => x.uid) });
  };
  const mover = (i: number, d: number) => {
    const j = i + d;
    if (j < 0 || j >= fila.length) return;
    const n = [...fila];
    [n[i], n[j]] = [n[j], n[i]];
    reordenar(n);
  };
  const atualUid = painel?.origem.tipo === "livre" ? painel.origem.uid : null;

  if (fila.length === 0) return <div className="vazio">A fila está vazia. Adicione músicas, vinhetas ou recados pela lista ao lado ou pela Biblioteca.</div>;

  return (
    <div className="lista" style={compacta ? { maxHeight: "52vh", overflow: "auto" } : undefined}>
      {fila.map((it, i) => {
        const f = it.faixa;
        const invalido = !f || f.status !== "ok";
        return (
          <div
            key={it.uid}
            className={"linha-item" + (it.uid === atualUid ? " atual livre" : "") + (arrastando === it.uid ? " arrastando" : "") + (alvo === i && arrastando != null ? " alvo" : "")}
            draggable
            onDragStart={(e) => {
              setArrastando(it.uid);
              e.dataTransfer.effectAllowed = "move";
            }}
            onDragOver={(e) => {
              e.preventDefault();
              setAlvo(i);
            }}
            onDragEnd={() => {
              setArrastando(null);
              setAlvo(null);
            }}
            onDrop={(e) => {
              e.preventDefault();
              if (arrastando == null) return;
              const de = fila.findIndex((x) => x.uid === arrastando);
              const n = [...fila];
              const [m] = n.splice(de, 1);
              n.splice(i, 0, m);
              setArrastando(null);
              setAlvo(null);
              reordenar(n);
            }}
            style={{ gridTemplateColumns: "auto 1fr auto" }}
          >
            <span className="alca" title="Arraste para reordenar" aria-hidden>
              <LuGripVertical />
            </span>
            <span style={{ minWidth: 0 }}>
              <b style={{ display: "block", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                {i + 1}. {f ? f.titulo || f.nome : "(arquivo removido da biblioteca)"}
              </b>
              <span className="pequeno muted">
                {f ? `${CATEGORIAS[f.categoria] ?? f.categoria} · ${fmtTempo(f.duracaoMs)}${f.artista ? ` · ${f.artista}` : ""}` : ""}
                {invalido && <span className="chip erro" style={{ marginLeft: 6 }}>{f ? f.status : "ausente"} — será pulado</span>}
              </span>
            </span>
            <span className="flex" style={{ flexWrap: "nowrap" }}>
              {!compacta && (
                <>
                  <button className="btn icone" onClick={() => mover(i, -1)} aria-label="Mover para cima">
                    <LuChevronUp />
                  </button>
                  <button className="btn icone" onClick={() => mover(i, 1)} aria-label="Mover para baixo">
                    <LuChevronDown />
                  </button>
                </>
              )}
              <button className="btn icone" onClick={() => chamar("fila_tocar", { uid: it.uid }).catch((e) => toast(e.message, "erro"))} aria-label="Tocar agora" disabled={invalido}>
                <LuPlay />
              </button>
              {!compacta && (
                <button className="btn icone perigo" onClick={() => chamar("fila_remover", { uid: it.uid })} aria-label="Remover da fila">
                  <LuTrash2 />
                </button>
              )}
            </span>
          </div>
        );
      })}
    </div>
  );
}
