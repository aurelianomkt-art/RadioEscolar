import React, { useEffect, useRef, useState } from "react";
import { LuMegaphone, LuMusic, LuPause, LuPlay, LuSkipBack, LuSkipForward, LuVolume2, LuRadio } from "react-icons/lu";
import { chamar, fmtTempo } from "../api";
import { NOMES_TIPO } from "../tipos";
import { useApp } from "../ui";

export default function Player() {
  const { painel, toast } = useApp();
  const [niveis, setNiveis] = useState<number[]>(Array(28).fill(0));
  const [vol, setVol] = useState<number | null>(null);
  const timerVol = useRef<any>(null);
  const [capa, setCapa] = useState<string | null>(null);
  const faixaId = painel?.audio.item?.faixaId ?? null;
  useEffect(() => {
    setCapa(null);
    if (faixaId) chamar<string | null>("capa", { faixaId }).then(setCapa).catch(() => {});
  }, [faixaId]);

  const a = painel?.audio;
  useEffect(() => {
    if (!a) return;
    setNiveis((n) => [...n.slice(1), Math.min(1, a.tocando ? a.nivel : 0)]);
  }, [painel?.agora, a?.posicaoMs]);

  if (!painel || !a) return <div className="cartao">Carregando…</div>;
  const o = painel.origem;
  const livre = o.tipo === "livre" || painel.modo === "Livre";
  const temItem = !!a.item;
  const dur = a.duracaoMs ?? a.item?.duracaoMs ?? null;
  const pct = temItem && dur ? Math.min(100, (a.posicaoMs / dur) * 100) : 0;
  const volume = vol ?? painel.volumeGeral;

  const exec = (cmd: string, args: any = {}) => chamar(cmd, args).catch((e) => toast(e.message, "erro"));
  const mudarVolume = (v: number) => {
    setVol(v);
    clearTimeout(timerVol.current);
    timerVol.current = setTimeout(() => chamar("definir_volume", { volume: v }).then(() => setVol(null)), 150);
  };

  let origemTxt = "Nada tocando";
  if (o.tipo === "evento") origemTxt = `${NOMES_TIPO[o.tipoEvento]} · ${o.nome}${o.fim ? ` (até ${o.fim})` : ""}`;
  else if (o.tipo === "livre") origemTxt = "Rádio Livre · fila";
  if (a.emAnuncio) origemTxt = "Recado / anúncio no ar";

  return (
    <div className="cartao">
      <div className="player">
        <div className={"capa" + (temItem ? (livre ? " livre" : "") : " parado")} aria-hidden>
          {capa && temItem && !a.emAnuncio ? (
            <img src={capa} alt="" style={{ width: "100%", height: "100%", objectFit: "cover", borderRadius: "inherit" }} />
          ) : a.emAnuncio ? (
            <LuMegaphone />
          ) : temItem ? (
            <LuMusic />
          ) : (
            <LuRadio />
          )}
        </div>
        <div style={{ minWidth: 0 }}>
          <div className={"origem" + (livre ? " livre" : "")}>{origemTxt}</div>
          <div className="titulo-musica">{a.item ? a.item.titulo : painel.silenciado ? "Rádio silenciada" : painel.modo === "Livre" ? "Fila pronta: toque Play" : "Aguardando o próximo horário"}</div>
          <div className="artista">
            {a.item?.artista || (a.item ? "Artista não informado" : painel.modo === "Livre" ? "Escolha músicas na fila do Rádio Livre" : "A programação toca sozinha nos horários definidos")}
            {a.totalItens > 1 && ` · item ${a.indiceItem + 1} de ${a.totalItens}`}
            {a.pausado && " · PAUSADO"}
          </div>
        </div>
      </div>

      <div className="progresso">
        <div
          className={"barra" + (livre ? " livre" : "")}
          role="progressbar"
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={Math.round(pct)}
          onClick={(e) => {
            if (!dur) return;
            const r = (e.currentTarget as HTMLDivElement).getBoundingClientRect();
            exec("buscar", { ms: Math.round(((e.clientX - r.left) / r.width) * dur) });
          }}
        >
          <div style={{ width: `${pct}%` }} />
        </div>
        <div className="tempos">
          <span>{temItem ? fmtTempo(a.posicaoMs) : "--:--"}</span>
          <span>{temItem && dur ? `-${fmtTempo(dur - a.posicaoMs)}` : "--:--"}</span>
        </div>
      </div>

      <div className={"niveis" + (a.tocando ? " tocando" : "")} aria-hidden title="Visualizador de níveis">
        {niveis.map((n, i) => (
          <span key={i} style={{ height: `${Math.max(4, n * 100)}%` }} />
        ))}
      </div>

      <div className="controles">
        <button className="btn redondo" onClick={() => exec("anterior")} aria-label="Anterior / reiniciar faixa">
          <LuSkipBack />
        </button>
        <button className={"btn redondo grande " + (livre ? "acao" : "primario")} onClick={() => exec("tocar_pausar")} aria-label={a.tocando ? "Pausar" : "Tocar"}>
          {a.tocando ? <LuPause /> : <LuPlay />}
        </button>
        <button className="btn redondo" onClick={() => exec("proxima")} aria-label="Próxima">
          <LuSkipForward />
        </button>
      </div>

      <div className="volume">
        <LuVolume2 size={22} aria-hidden />
        <input type="range" min={0} max={painel.volumeMaximo} value={volume} onChange={(e) => mudarVolume(Number(e.target.value))} aria-label="Volume geral" />
        <span style={{ minWidth: "5.5rem", textAlign: "right", fontWeight: 600 }}>
          {volume}% <small className="muted">máx {painel.volumeMaximo}</small>
        </span>
      </div>
    </div>
  );
}
