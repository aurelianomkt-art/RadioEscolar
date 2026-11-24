import React, { useEffect, useState } from "react";
import { LuGripVertical, LuMegaphone, LuPencil, LuSiren } from "react-icons/lu";
import { chamar } from "../api";
import type { Faixa } from "../tipos";
import { useApp } from "../ui";

/** Botão de recado: nome escrito pelo usuário, recado e vinheta opcional. Guardado neste notebook. */
export interface BotaoRecado {
  nome: string;
  recadoId: string | null;
  vinhetaId: string | null;
}

// Posições 0 a 4 = teclas 1 a 5 (arrastáveis). Posição 5 = tecla 0, emergência (fixa).
const CHAVE = "rme.botoesRecado";
const PADRAO: BotaoRecado[] = [
  { nome: "Recado 1", recadoId: null, vinhetaId: null },
  { nome: "Recado 2", recadoId: null, vinhetaId: null },
  { nome: "Recado 3", recadoId: null, vinhetaId: null },
  { nome: "Recado 4", recadoId: null, vinhetaId: null },
  { nome: "Recado 5", recadoId: null, vinhetaId: null },
  { nome: "Emergência", recadoId: null, vinhetaId: null },
];

export function lerBotoes(): BotaoRecado[] {
  try {
    const bruto = localStorage.getItem(CHAVE);
    if (!bruto) return PADRAO;
    const lista = JSON.parse(bruto) as Partial<BotaoRecado>[];
    return PADRAO.map((p, i) => ({
      nome: typeof lista[i]?.nome === "string" ? (lista[i].nome as string) : p.nome,
      recadoId: lista[i]?.recadoId ?? null,
      vinhetaId: lista[i]?.vinhetaId ?? null,
    }));
  } catch {
    return PADRAO;
  }
}

export default function BotoesRecado() {
  const { toast, painel, config, confirmar } = useApp();
  const [botoes, setBotoes] = useState<BotaoRecado[]>(lerBotoes);
  const [recados, setRecados] = useState<Faixa[]>([]);
  const [vinhetas, setVinhetas] = useState<Faixa[]>([]);
  const [editando, setEditando] = useState<number | null>(null);
  const [arrastando, setArrastando] = useState<number | null>(null);
  const [alvo, setAlvo] = useState<number | null>(null);

  useEffect(() => {
    chamar<Faixa[]>("listar_faixas", { texto: "", categoria: "recado", status: "ok", limite: 500 })
      .then(setRecados)
      .catch(() => {});
    chamar<Faixa[]>("listar_faixas", { texto: "", categoria: "vinheta", status: "ok", limite: 500 })
      .then(setVinhetas)
      .catch(() => {});
  }, []);

  const salvar = (novos: BotaoRecado[]) => {
    setBotoes(novos);
    try {
      localStorage.setItem(CHAVE, JSON.stringify(novos));
    } catch {
      toast("Não foi possível guardar os botões neste notebook.", "erro");
    }
  };

  const disparar = async (i: number) => {
    const b = botoes[i];
    if (!b.recadoId) {
      toast(`Escolha o recado do botão "${b.nome}" (lápis).`, "info");
      return;
    }
    const musica = painel?.audio.tocando;
    if (musica) {
      const ok = await confirmar(
        "Tocar recado agora?",
        `A música será ${config?.ajustes.duckModo === "Abaixar" ? "abaixada" : "pausada"} suavemente e volta sozinha depois do recado.`,
        "Tocar recado"
      );
      if (!ok) return;
    }
    chamar("tocar_recado", { recadoId: b.recadoId, vinhetaId: b.vinhetaId, beep: config?.ajustes.beepAntesRecado ?? true })
      .then(() => toast("Recado no ar", "ok"))
      .catch((e) => toast(e.message, "erro"));
  };

  // Teclas 1 a 5 e 0 (fora de campos de texto). Reassina a cada render para usar os botões atuais.
  useEffect(() => {
    const h = (e: KeyboardEvent) => {
      const alvoTecla = e.target as HTMLElement;
      if (["INPUT", "SELECT", "TEXTAREA"].includes(alvoTecla?.tagName)) return;
      if (e.key >= "1" && e.key <= "5") disparar(Number(e.key) - 1);
      else if (e.key === "0") disparar(5);
    };
    window.addEventListener("keydown", h);
    return () => window.removeEventListener("keydown", h);
  });

  const atualizar = (i: number, parcial: Partial<BotaoRecado>) => salvar(botoes.map((b, j) => (j === i ? { ...b, ...parcial } : b)));

  // Arrastar só entre as cinco teclas numéricas; a emergência fica sempre na tecla 0.
  const soltar = (para: number) => {
    if (arrastando == null || arrastando === para || arrastando > 4 || para > 4) return;
    const nova = [...botoes];
    const [item] = nova.splice(arrastando, 1);
    nova.splice(para, 0, item);
    salvar(nova);
  };

  return (
    <div className="cartao">
      <h2>Recados</h2>
      <div className="pequeno muted" style={{ marginBottom: 8 }}>
        Teclas 1 a 5 tocam os recados (arraste para mudar a ordem); a tecla 0 é o recado de emergência.
      </div>
      <div className="botoes-recado">
        {botoes.map((b, i) => {
          const emergencia = i === 5;
          const recado = recados.find((r) => r.id === b.recadoId);
          const vinheta = vinhetas.find((v) => v.id === b.vinhetaId);
          return (
            <div
              key={i}
              className={"botao-recado" + (emergencia ? " emergencia" : "") + (arrastando === i ? " arrastando" : "") + (alvo === i && arrastando != null ? " alvo" : "")}
              draggable={!emergencia}
              onDragStart={(e) => {
                if (emergencia) return;
                setArrastando(i);
                e.dataTransfer.effectAllowed = "move";
              }}
              onDragOver={(e) => {
                if (emergencia) return;
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
              {!emergencia && (
                <span className="botao-recado-alca" aria-hidden>
                  <LuGripVertical />
                </span>
              )}
              <button className="btn botao-recado-disparo" onClick={() => disparar(i)} title={`Tecla ${emergencia ? 0 : i + 1}`}>
                {emergencia ? <LuSiren /> : <LuMegaphone />}
                <span className="botao-recado-nome">{b.nome}</span>
                <small className="muted">
                  {recado ? recado.titulo || recado.nome : "sem recado"}
                  {vinheta ? ` · vinheta: ${vinheta.nome}` : ""}
                </small>
                <kbd>{emergencia ? 0 : i + 1}</kbd>
              </button>
              <button className="btn icone" onClick={() => setEditando(editando === i ? null : i)} aria-label={`Editar ${b.nome}`}>
                <LuPencil />
              </button>
              {editando === i && (
                <div className="botao-recado-editor">
                  <input type="text" value={b.nome} maxLength={24} onChange={(e) => atualizar(i, { nome: e.target.value })} aria-label="Nome do botão" />
                  <select value={b.recadoId ?? ""} onChange={(e) => atualizar(i, { recadoId: e.target.value || null })} aria-label="Recado deste botão">
                    <option value="">— escolher recado —</option>
                    {recados.map((r) => (
                      <option key={r.id} value={r.id}>
                        {r.titulo || r.nome}
                      </option>
                    ))}
                  </select>
                  <select value={b.vinhetaId ?? ""} onChange={(e) => atualizar(i, { vinhetaId: e.target.value || null })} aria-label="Vinheta antes deste botão">
                    <option value="">{emergencia ? "Sem vinheta (sirene própria, se houver)" : "Sem vinheta antes"}</option>
                    {vinhetas.map((v) => (
                      <option key={v.id} value={v.id}>
                        {v.titulo || v.nome}
                      </option>
                    ))}
                  </select>
                  <button className="btn" onClick={() => setEditando(null)}>
                    Pronto
                  </button>
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
