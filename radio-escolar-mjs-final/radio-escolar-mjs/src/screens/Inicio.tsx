import React from "react";
import { LuCalendarDays, LuListMusic, LuMegaphone, LuTriangleAlert, LuWandSparkles } from "react-icons/lu";
import Player from "../components/Player";
import AgendaDia from "../components/AgendaDia";
import FilaResumo from "../components/FilaResumo";
import { useApp } from "../ui";

export default function Inicio() {
  const { painel, ir } = useApp();
  if (!painel) return <div className="muted">Conectando ao aplicativo…</div>;
  return (
    <div className="grade d2">
      <div className="grade" style={{ alignContent: "start" }}>
        <Player />
        <div className="cartao">
          <h2>Ações rápidas</h2>
          <div className="flex">
            <button className="btn grande acao" onClick={() => ir("recados")}>
              <LuMegaphone /> Recado imediato
            </button>
            <button className="btn grande" onClick={() => ir("programacao")}>
              <LuCalendarDays /> Programação
            </button>
            <button className="btn grande" onClick={() => ir("livre")}>
              <LuListMusic /> Fila do Rádio Livre
            </button>
          </div>
          {painel.ultimaAcao && <p className="pequeno muted" style={{ marginBottom: 0 }}>Última ação: {painel.ultimaAcao}</p>}
        </div>
        {painel.avisosConfig.length > 0 && (
          <div className="cartao">
            <h2>
              <LuTriangleAlert color="var(--laranja)" /> Pendências de configuração
            </h2>
            <div className="avisos">
              {painel.avisosConfig.map((a) => (
                <div key={a} className="aviso Alerta">
                  {a}
                </div>
              ))}
            </div>
            <div className="flex" style={{ marginTop: 10 }}>
              <button className="btn" onClick={() => ir("assistente")}>
                <LuWandSparkles /> Abrir assistente
              </button>
              <button className="btn" onClick={() => ir("biblioteca")}>
                Biblioteca
              </button>
            </div>
          </div>
        )}
      </div>
      <div className="cartao">
        {painel.modo === "Escolar" ? (
          <>
            <h2>
              <LuCalendarDays /> Programação de hoje
            </h2>
            <AgendaDia compacta />
          </>
        ) : (
          <>
            <h2>
              <LuListMusic /> Fila do Rádio Livre
            </h2>
            <FilaResumo />
          </>
        )}
      </div>
    </div>
  );
}
