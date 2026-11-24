import React, { useEffect, useState } from "react";
import { LuToggleLeft, LuToggleRight } from "react-icons/lu";
import { chamar, hojeIso } from "../api";
import type { Agenda, ItemAgenda, PerfilDia } from "../tipos";
import { NOMES_TIPO } from "../tipos";
import { useApp } from "../ui";

export function nomePerfil(p: PerfilDia | undefined, perfis: { id: string; nome: string }[] = []) {
  if (!p) return "";
  if (p.tipo === "SemAula") return "Sem aula (nada programado)";
  if (p.tipo === "Suspenso") return `Dia suspenso${p.valor ? ` — ${p.valor}` : ""}`;
  return perfis.find((x) => x.id === p.valor)?.nome ?? p.valor;
}

export default function AgendaDia({ data, agenda, compacta, onAlternar }: { data?: string; agenda?: Agenda; compacta?: boolean; onAlternar?: (eventoId: string) => void }) {
  const { painel, config } = useApp();
  const [dia, setDia] = useState<{ perfil: PerfilDia; itens: ItemAgenda[] } | null>(null);
  const d = data ?? hojeIso();

  useEffect(() => {
    let vivo = true;
    const carregar = () => chamar("agenda_dia", { data: d, agenda: agenda ?? null }).then((r: any) => vivo && setDia(r)).catch(() => {});
    carregar();
    const t = setInterval(carregar, 20000);
    return () => {
      vivo = false;
      clearInterval(t);
    };
  }, [d, agenda, config?.agenda]);

  if (!dia) return <div className="muted">Carregando agenda…</div>;
  const atual = painel?.origem.tipo === "evento" ? painel.origem.eventoId : null;
  return (
    <div>
      <div className="pequeno muted" style={{ marginBottom: 8 }}>
        Perfil do dia: <b>{nomePerfil(dia.perfil, config?.agenda.perfis)}</b>
      </div>
      {dia.itens.length === 0 ? (
        <div className="vazio">Nenhum evento neste dia.</div>
      ) : (
        <div className="lista" style={compacta ? { maxHeight: "48vh", overflow: "auto" } : undefined}>
          {dia.itens.map((it) => {
            const passado = it.emSegundos < 0 && it.eventoId !== atual;
            const desligado = it.suprimidoPor === "desligado neste dia";
            return (
              <div key={it.eventoId} className={"linha-item tipo-" + it.tipo + (it.eventoId === atual ? " atual" : "") + (passado ? " passado" : "") + (it.suprimidoPor ? " suprimido" : "")}>
                <span className="hora-item">
                  {it.inicio}
                  {it.fim && <small className="muted" style={{ display: "block", fontWeight: 500 }}>até {it.fim}</small>}
                </span>
                <span style={{ minWidth: 0 }}>
                  <b>{it.nome}</b>
                  {desligado && <span className="pequeno muted"> — desligado neste dia</span>}
                  {it.suprimidoPor && !desligado && <span className="pequeno muted"> — não toca: {it.suprimidoPor} prevalece</span>}
                  {it.eventoId === atual && <span className="pequeno" style={{ color: "var(--azul)", fontWeight: 700 }}> — TOCANDO</span>}
                </span>
                <span className={"chip " + it.tipo}>{NOMES_TIPO[it.tipo]}</span>
                {onAlternar && (
                  <button
                    className="btn icone"
                    onClick={() => onAlternar(it.eventoId)}
                    aria-pressed={!desligado}
                    aria-label={`${desligado ? "Ligar" : "Desligar"} ${it.nome} neste dia`}
                    title={desligado ? "Ligar neste dia" : "Desligar só neste dia"}
                  >
                    {desligado ? <LuToggleLeft /> : <LuToggleRight />}
                  </button>
                )}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
