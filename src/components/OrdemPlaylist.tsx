import React, { useEffect, useState } from "react";
import { LuGripVertical, LuX } from "react-icons/lu";
import { chamar } from "../api";
import type { Faixa } from "../tipos";

/** Músicas da playlist em ordem, com arrastar-e-soltar para mudar a ordem de execução. */
export default function OrdemPlaylist({ ids, onChange }: { ids: string[]; onChange: (ids: string[]) => void }) {
  const [nomes, setNomes] = useState<Record<string, string>>({});
  const [arrastando, setArrastando] = useState<number | null>(null);
  const [alvo, setAlvo] = useState<number | null>(null);

  useEffect(() => {
    chamar<Faixa[]>("listar_faixas", { texto: "", categoria: "musica", status: "", limite: 5000 })
      .then((lista) => {
        const mapa: Record<string, string> = {};
        lista.forEach((f) => (mapa[f.id] = f.titulo || f.nome));
        setNomes(mapa);
      })
      .catch(() => {});
  }, []);

  const soltar = (para: number) => {
    if (arrastando == null || arrastando === para) return;
    const nova = [...ids];
    const [item] = nova.splice(arrastando, 1);
    nova.splice(para, 0, item);
    onChange(nova);
  };

  if (ids.length === 0) return <div className="vazio">Playlist vazia. Use "Nova playlist com…" na aba Arquivos.</div>;

  return (
    <div className="lista ordem-playlist">
      {ids.map((id, i) => (
        <div
          key={id + "-" + i}
          className={"linha-item ordem" + (arrastando === i ? " arrastando" : "") + (alvo === i && arrastando != null ? " alvo" : "")}
          draggable
          onDragStart={(e) => {
            setArrastando(i);
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
            soltar(i);
            setArrastando(null);
            setAlvo(null);
          }}
        >
          <LuGripVertical aria-hidden />
          <span className="cortar">
            {i + 1}. {nomes[id] ?? "(arquivo fora da biblioteca)"}
          </span>
          <button className="btn icone" onClick={() => onChange(ids.filter((_, j) => j !== i))} aria-label="Retirar da playlist">
            <LuX />
          </button>
        </div>
      ))}
    </div>
  );
}
