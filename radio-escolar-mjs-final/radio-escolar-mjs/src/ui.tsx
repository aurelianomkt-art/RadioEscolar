import React, { createContext, useContext } from "react";
import { LuArrowLeft } from "react-icons/lu";
import type { ConfigCompleta, Painel, Tela } from "./tipos";

export interface Ctx {
  painel: Painel | null;
  config: ConfigCompleta | null;
  recarregarConfig: () => Promise<void>;
  toast: (msg: string, tipo?: "ok" | "erro" | "info") => void;
  ir: (t: Tela) => void;
  confirmar: (titulo: string, texto: string, ok?: string) => Promise<boolean>;
  logo: string;
  trocarModo: (m: "Escolar" | "Livre") => Promise<void>;
}

export const AppCtx = createContext<Ctx>(null as any);
export const useApp = () => useContext(AppCtx);

export function Cabecalho({ titulo, voltar = true, children }: { titulo: string; voltar?: boolean; children?: React.ReactNode }) {
  const { ir } = useApp();
  return (
    <div className="cabecalho-tela">
      {voltar && (
        <button className="btn" onClick={() => ir("inicio")} aria-label="Voltar ao painel">
          <LuArrowLeft /> Voltar
        </button>
      )}
      <h1>{titulo}</h1>
      <div className="acoes">{children}</div>
    </div>
  );
}

export function Modal({ titulo, children, rodape, largo, fechar }: { titulo: string; children: React.ReactNode; rodape?: React.ReactNode; largo?: boolean; fechar?: () => void }) {
  return (
    <div
      className="fundo-modal"
      role="dialog"
      aria-modal="true"
      aria-label={titulo}
      onKeyDown={(e) => {
        if (e.key === "Escape" && fechar) fechar();
      }}
    >
      <div className={"modal" + (largo ? " largo" : "")}>
        <h2>{titulo}</h2>
        {children}
        {rodape && <div className="rodape-modal">{rodape}</div>}
      </div>
    </div>
  );
}

export function Campo({ rotulo, ajuda, children }: { rotulo: string; ajuda?: string; children: React.ReactNode }) {
  return (
    <div className="campo">
      <label>{rotulo}</label>
      {children}
      {ajuda && <span className="ajuda">{ajuda}</span>}
    </div>
  );
}

export function Deslizante({ valor, min = 0, max = 100, passo = 1, onChange, sufixo = "%", rotulo }: { valor: number; min?: number; max?: number; passo?: number; onChange: (v: number) => void; sufixo?: string; rotulo: string }) {
  return (
    <div className="flex" style={{ flexWrap: "nowrap" }}>
      <input type="range" aria-label={rotulo} min={min} max={max} step={passo} value={valor} onChange={(e) => onChange(Number(e.target.value))} style={{ flex: 1 }} />
      <span style={{ minWidth: "3.4rem", textAlign: "right", fontVariantNumeric: "tabular-nums", fontWeight: 600 }}>
        {valor}
        {sufixo}
      </span>
    </div>
  );
}

export function Checa({ v, onChange, children }: { v: boolean; onChange: (b: boolean) => void; children: React.ReactNode }) {
  return (
    <label className="checa">
      <input type="checkbox" checked={!!v} onChange={(e) => onChange(e.target.checked)} />
      <span>{children}</span>
    </label>
  );
}
