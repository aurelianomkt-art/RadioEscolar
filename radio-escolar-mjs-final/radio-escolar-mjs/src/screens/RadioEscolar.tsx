import React, { useState } from "react";
import { LuCalendarDays, LuClock, LuPencil } from "react-icons/lu";
import Player from "../components/Player";
import AgendaDia from "../components/AgendaDia";
import BotoesRecado from "../components/BotoesRecado";
import { chamar, hojeIso } from "../api";
import { NOMES_TIPO } from "../tipos";
import { Cabecalho, useApp } from "../ui";

export default function RadioEscolar() {
  const { painel, ir, trocarModo, config, recarregarConfig, toast } = useApp();
  const [data, setData] = useState(hojeIso());
  const a = config?.agenda;

  // Interruptor por toque: liga/desliga o evento só na data mostrada.
  const alternarNoDia = async (eventoId: string) => {
    if (!config) return;
    const agenda = structuredClone(config.agenda);
    const existe = agenda.desligados.some((d) => d.data === data && d.eventoId === eventoId);
    agenda.desligados = existe
      ? agenda.desligados.filter((d) => !(d.data === data && d.eventoId === eventoId))
      : [...agenda.desligados, { data, eventoId }];
    try {
      await chamar("salvar_agenda", { agenda });
      await recarregarConfig();
      toast(existe ? "Evento ligado neste dia" : "Evento desligado só neste dia", "ok");
    } catch (e: any) {
      toast(e.message, "erro");
    }
  };
  const tocando = !!painel?.audio.item;
  const titulo = painel?.audio.item?.titulo ?? (painel?.silenciado ? "Rádio silenciada" : "Aguardando o próximo horário");
  const proximos = (painel?.proximos ?? []).filter((x) => !x.suprimidoPor).slice(0, 5);

  return (
    <div>
      <Cabecalho titulo="Rádio Escolar">
        {painel?.modo !== "Escolar" && (
          <button className="btn primario grande" onClick={() => trocarModo("Escolar")}>
            Voltar ao Rádio Escolar
          </button>
        )}
        <button className="btn" onClick={() => ir("programacao")}>
          <LuPencil /> Editar programação
        </button>
      </Cabecalho>

      <div className={"letreiro" + (tocando ? " no-ar" : "")} aria-live="polite">
        <div className="letreiro-rotulo">{painel?.origem.tipo === "evento" ? NOMES_TIPO[painel.origem.tipoEvento] : "No ar"}</div>
        <div className="letreiro-titulo">{titulo}</div>
        {painel?.origem.tipo === "evento" && <div className="letreiro-sub">{painel.origem.nome}</div>}
      </div>

      <div className="grade d2">
        <div className="grade" style={{ alignContent: "start" }}>
          <Player />
          <div className="cartao">
            <h2><LuClock /> Próximos horários</h2>
            {proximos.length === 0 ? (
              <div className="vazio">Nenhum horário pendente hoje.</div>
            ) : (
              <div className="lista">
                {proximos.map((p) => (
                  <div key={p.eventoId + p.data + p.inicio} className={"linha-item tipo-" + p.tipo}>
                    <span className="hora-item">{p.inicio}</span>
                    <span style={{ minWidth: 0 }}>
                      <b>{p.nome}</b>
                    </span>
                    <span className={"chip " + p.tipo}>{NOMES_TIPO[p.tipo]}</span>
                  </div>
                ))}
              </div>
            )}
            <ul className="pequeno" style={{ lineHeight: 1.6, margin: "12px 0 0", paddingLeft: "1.1rem" }}>
              <li><b>Trocas de professor:</b> uma música completa por horário, sem sobreposição (limite de duração opcional por evento).</li>
              <li><b>Recepção e recreio:</b> músicas em sequência até o horário final, com fade-out no limite (ou terminar a faixa, se configurado).</li>
              <li><b>Mesmo minuto:</b> o recreio prevalece sobre a sinalização (09:30 e 15:30). Nunca tocam duas músicas juntas.</li>
              <li><b>Computador desligado, suspenso ou relógio alterado:</b> horários perdidos <u>não</u> são tocados depois.</li>
              <li><b>Reinício durante recepção/recreio:</b> {a?.retomarJanelaAposReinicio ? "retoma a playlist da janela atual" : "não retoma automaticamente"} (configurável).</li>
              <li><b>Silenciar agora</b> bloqueia tudo até alguém tocar em Retomar.</li>
            </ul>
          </div>
        </div>
        <div className="grade" style={{ alignContent: "start" }}>
          <BotoesRecado />
          <div className="cartao">
            <div className="flex" style={{ marginBottom: 10 }}>
              <h2 style={{ margin: 0 }}>
                <LuCalendarDays /> Agenda do dia
              </h2>
              <span className="espaco" />
              <input type="date" value={data} onChange={(e) => setData(e.target.value || hojeIso())} aria-label="Escolher dia" />
            </div>
            <AgendaDia data={data} onAlternar={alternarNoDia} />
          </div>
        </div>
      </div>
    </div>
  );
}
