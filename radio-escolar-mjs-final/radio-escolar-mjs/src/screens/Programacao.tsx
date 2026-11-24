import React, { useEffect, useMemo, useRef, useState } from "react";
import { LuCopy, LuDownload, LuPencil, LuPlus, LuSave, LuTrash2, LuUndo2, LuUpload, LuCheck } from "react-icons/lu";
import AgendaDia from "../components/AgendaDia";
import { chamar, escolherArquivo, hojeIso, salvarArquivo } from "../api";
import type { Agenda, Aviso, Evento, Faixa, Pasta, Playlist, TipoEvento } from "../tipos";
import { DIAS, NOMES_TIPO } from "../tipos";
import { Cabecalho, Campo, Checa, Deslizante, Modal, useApp } from "../ui";

const hm = (s: string) => {
  const [h, m] = (s || "0:0").split(":").map(Number);
  return h * 60 + (m || 0);
};

function novoEvento(tipo: TipoEvento = "Sinal", aj?: { fadeInPadraoMs: number; fadeOutPadraoMs: number; volumesTipo: any }): Evento {
  const janela = tipo === "Recepcao" || tipo === "Recreio";
  const vt = aj?.volumesTipo;
  return {
    id: `ev-${Date.now().toString(36)}`, nome: "", tipo, ativo: true, dias: [0, 1, 2, 3, 4], perfis: ["normal"],
    inicio: "07:00", fim: janela ? "07:20" : null, playlistId: null, pasta: null, faixaId: null, selecao: "Aleatorio",
    volume: vt ? { Recepcao: vt.recepcao, Recreio: vt.recreio, Sinal: vt.sinal, Saida: vt.sinal, Recado: vt.recado }[tipo] : janela ? 60 : 55,
    fadeInMs: aj?.fadeInPadraoMs ?? (janela ? 3000 : 1500), fadeOutMs: aj?.fadeOutPadraoMs ?? (janela ? 6000 : 3000), limiteSegundos: null,
    fimJanela: "FadeOut", extraMaxSegundos: 240, vinhetaId: null, recadoId: null,
    prioridade: tipo === "Recreio" ? 80 : tipo === "Recepcao" ? 60 : tipo === "Recado" ? 40 : 20, conflito: "Interromper", observacao: "",
    avisoTroca: tipo === "Sinal" || tipo === "Saida", avisoAtraso: false,
  };
}

export default function Programacao() {
  const { config, recarregarConfig, toast, confirmar } = useApp();
  const [ag, setAg] = useState<Agenda | null>(null);
  const [aba, setAba] = useState("eventos");
  const [avisos, setAvisos] = useState<Aviso[]>([]);
  const [editando, setEditando] = useState<Evento | null>(null);
  const [diaPrevia, setDiaPrevia] = useState(hojeIso());
  const [novaEx, setNovaEx] = useState({ data: hojeIso(), perfil: "feriado" as string | null, descricao: "" });
  const original = useRef<string>("");

  useEffect(() => {
    if (config && !ag) {
      setAg(structuredClone(config.agenda));
      original.current = JSON.stringify(config.agenda);
    }
  }, [config]);

  useEffect(() => {
    if (!ag) return;
    const t = setTimeout(() => chamar<Aviso[]>("validar_agenda", { agenda: ag }).then(setAvisos).catch(() => {}), 300);
    return () => clearTimeout(t);
  }, [ag]);

  const alterado = ag && JSON.stringify(ag) !== original.current;
  const erros = avisos.filter((a) => a.gravidade === "Erro").length;

  const grupos = useMemo(() => {
    if (!ag) return [];
    const ord = [...ag.eventos].sort((a, b) => hm(a.inicio) - hm(b.inicio) || b.prioridade - a.prioridade);
    return [
      ["Manhã", ord.filter((e) => hm(e.inicio) < 12 * 60 + 30)],
      ["Tarde", ord.filter((e) => hm(e.inicio) >= 12 * 60 + 30 && hm(e.inicio) < 18 * 60)],
      ["Noite", ord.filter((e) => hm(e.inicio) >= 18 * 60)],
    ] as [string, Evento[]][];
  }, [ag]);

  if (!ag) return <div className="muted">Carregando…</div>;

  const salvar = async () => {
    try {
      await chamar("salvar_agenda", { agenda: ag });
      original.current = JSON.stringify(ag);
      await recarregarConfig();
      toast("Programação salva.", "ok");
    } catch (e: any) {
      toast(e.message, "erro");
    }
  };
  const gravarEvento = (e: Evento) => {
    const existe = ag.eventos.some((x) => x.id === e.id);
    setAg({ ...ag, eventos: existe ? ag.eventos.map((x) => (x.id === e.id ? e : x)) : [...ag.eventos, e] });
    setEditando(null);
  };
  const avisosDe = (id: string) => avisos.filter((a) => a.eventoId === id);

  return (
    <div>
      <Cabecalho titulo="Programação e Calendário">
        {alterado && <span className="chip" style={{ alignSelf: "center", background: "var(--amarelo-claro)" }}>Alterações não salvas</span>}
        <button
          className="btn"
          disabled={!alterado}
          onClick={async () => {
            if (await confirmar("Descartar alterações?", "Volta para a última programação salva.", "Descartar")) setAg(JSON.parse(original.current));
          }}
        >
          <LuUndo2 /> Descartar
        </button>
        <button className="btn primario" disabled={!alterado || erros > 0} onClick={salvar} title={erros ? "Corrija os erros antes" : ""}>
          <LuSave /> Salvar programação
        </button>
      </Cabecalho>

      <div className="abas" role="tablist">
        {[
          ["eventos", "Eventos"],
          ["semana", "Perfis da semana"],
          ["calendario", "Calendário e exceções"],
          ["previa", "Prévia do dia"],
          ["regras", "Regras gerais"],
          ["backup", "Backup"],
        ].map(([k, n]) => (
          <button key={k} className={aba === k ? "on" : ""} onClick={() => setAba(k)} role="tab" aria-selected={aba === k}>
            {n}
          </button>
        ))}
      </div>

      <div className="grade d2">
        <div className="cartao">
          {aba === "eventos" && (
            <>
              <div className="flex" style={{ marginBottom: 10 }}>
                {(["Sinal", "Saida", "Recreio", "Recepcao", "Recado"] as TipoEvento[]).map((t) => (
                  <button key={t} className="btn" onClick={() => setEditando(novoEvento(t, config?.ajustes))}>
                    <LuPlus /> {NOMES_TIPO[t]}
                  </button>
                ))}
              </div>
              {grupos.map(([nome, evs]) =>
                evs.length ? (
                  <div key={nome}>
                    <h3>{nome}</h3>
                    <div className="lista">
                      {evs.map((e) => {
                        const av = avisosDe(e.id);
                        const erro = av.some((a) => a.gravidade === "Erro");
                        return (
                          <div key={e.id} className={"linha-item tipo-" + e.tipo + (!e.ativo ? " passado" : "")} style={{ borderColor: erro ? "var(--vermelho)" : undefined }}>
                            <span className="hora-item">
                              {e.inicio}
                              {e.fim && <small className="muted" style={{ display: "block", fontWeight: 500 }}>até {e.fim}</small>}
                            </span>
                            <span style={{ minWidth: 0 }}>
                              <b>{e.nome || "(sem nome)"}</b> {!e.ativo && <span className="chip">desativado</span>}
                              <div className="pequeno muted">
                                {e.dias.map((d) => DIAS[d]).join(", ")} · vol {e.volume}% · prioridade {e.prioridade}
                                {e.vinhetaId ? " · com vinheta" : ""}
                                {e.limiteSegundos ? ` · limite ${e.limiteSegundos}s` : ""}
                              </div>
                              {av.map((a, i) => (
                                <div key={i} className={"pequeno"} style={{ color: a.gravidade === "Erro" ? "var(--vermelho)" : a.gravidade === "Alerta" ? "#8a5a00" : "var(--azul)" }}>
                                  • {a.mensagem}
                                </div>
                              ))}
                            </span>
                            <span className="flex" style={{ flexWrap: "nowrap" }}>
                              <span className={"chip " + e.tipo}>{NOMES_TIPO[e.tipo]}</span>
                              <button className="btn icone" onClick={() => setEditando(structuredClone(e))} aria-label="Editar">
                                <LuPencil />
                              </button>
                              <button className="btn icone" onClick={() => setEditando({ ...structuredClone(e), id: `ev-${Date.now().toString(36)}`, nome: e.nome + " (cópia)" })} aria-label="Duplicar">
                                <LuCopy />
                              </button>
                              <button
                                className="btn icone perigo"
                                onClick={async () => {
                                  if (await confirmar("Excluir evento?", `"${e.nome}" será removido da programação (ao salvar).`, "Excluir")) setAg({ ...ag, eventos: ag.eventos.filter((x) => x.id !== e.id) });
                                }}
                                aria-label="Excluir"
                              >
                                <LuTrash2 />
                              </button>
                            </span>
                          </div>
                        );
                      })}
                    </div>
                  </div>
                ) : null
              )}
              <div className="flex" style={{ marginTop: 14 }}>
                <button
                  className="btn perigo"
                  onClick={async () => {
                    if (await confirmar("Restaurar agenda inicial?", "Substitui todos os eventos pela agenda padrão da escola (só vale depois de Salvar).", "Restaurar")) {
                      const p = await chamar<Agenda>("agenda_inicial");
                      setAg({ ...ag, eventos: p.eventos, perfis: p.perfis });
                    }
                  }}
                >
                  Restaurar agenda inicial
                </button>
              </div>
            </>
          )}

          {aba === "semana" && (
            <>
              <p className="muted pequeno">Escolha o perfil padrão de cada dia da semana. Cada evento diz em quais perfis vale (na edição do evento).</p>
              <div className="campos">
                {DIAS.map((d, i) => (
                  <Campo key={d} rotulo={d}>
                    <select
                      value={ag.perfilPorDiaSemana[i] ?? ""}
                      onChange={(e) => {
                        const v = [...ag.perfilPorDiaSemana];
                        v[i] = e.target.value || null;
                        setAg({ ...ag, perfilPorDiaSemana: v });
                      }}
                    >
                      <option value="">Sem aula</option>
                      {ag.perfis.map((p) => (
                        <option key={p.id} value={p.id}>
                          {p.nome}
                        </option>
                      ))}
                    </select>
                  </Campo>
                ))}
              </div>
              <h3>Perfis de dia</h3>
              <p className="muted pequeno">Perfis: dia normal, prova, evento, reunião, feriado, sábado letivo, dia reduzido. "Feriado" não tem eventos por padrão. Você pode renomear.</p>
              <div className="lista">
                {ag.perfis.map((p, i) => (
                  <div key={p.id} className="linha-item" style={{ gridTemplateColumns: "8rem 1fr auto" }}>
                    <code className="pequeno muted">{p.id}</code>
                    <input
                      type="text"
                      value={p.nome}
                      onChange={(e) => {
                        const ps = [...ag.perfis];
                        ps[i] = { ...p, nome: e.target.value };
                        setAg({ ...ag, perfis: ps });
                      }}
                      aria-label={`Nome do perfil ${p.id}`}
                    />
                    <span className="pequeno muted">{ag.eventos.filter((e) => e.perfis.includes(p.id)).length} evento(s)</span>
                  </div>
                ))}
              </div>
            </>
          )}

          {aba === "calendario" && (
            <>
              <p className="muted pequeno">Exceções valem só na data escolhida: troque o perfil do dia (prova, sábado letivo, feriado…) ou suspenda o dia inteiro.</p>
              <div className="flex" style={{ alignItems: "flex-end" }}>
                <Campo rotulo="Data">
                  <input type="date" value={novaEx.data} onChange={(e) => setNovaEx({ ...novaEx, data: e.target.value })} />
                </Campo>
                <Campo rotulo="O que acontece">
                  <select value={novaEx.perfil ?? "__suspender"} onChange={(e) => setNovaEx({ ...novaEx, perfil: e.target.value === "__suspender" ? null : e.target.value })}>
                    <option value="__suspender">Suspender o dia (nada toca)</option>
                    {ag.perfis.map((p) => (
                      <option key={p.id} value={p.id}>
                        Usar perfil: {p.nome}
                      </option>
                    ))}
                  </select>
                </Campo>
                <Campo rotulo="Descrição">
                  <input type="text" value={novaEx.descricao} placeholder="Ex.: Feriado municipal" onChange={(e) => setNovaEx({ ...novaEx, descricao: e.target.value })} />
                </Campo>
                <button
                  className="btn primario"
                  onClick={() => {
                    if (!novaEx.data) return;
                    setAg({ ...ag, excecoes: [...ag.excecoes.filter((x) => x.data !== novaEx.data), { ...novaEx }].sort((a, b) => a.data.localeCompare(b.data)) });
                  }}
                >
                  <LuPlus /> Adicionar
                </button>
              </div>
              <div className="lista" style={{ marginTop: 12 }}>
                {ag.excecoes.length === 0 && <div className="vazio">Nenhuma exceção cadastrada.</div>}
                {ag.excecoes.map((x) => (
                  <div key={x.data} className="linha-item">
                    <span className="hora-item">{x.data.split("-").reverse().join("/")}</span>
                    <span>
                      <b>{x.perfil ? ag.perfis.find((p) => p.id === x.perfil)?.nome ?? x.perfil : "Dia suspenso"}</b> {x.descricao && <span className="muted">— {x.descricao}</span>}
                    </span>
                    <button className="btn icone perigo" onClick={() => setAg({ ...ag, excecoes: ag.excecoes.filter((y) => y.data !== x.data) })} aria-label="Remover">
                      <LuTrash2 />
                    </button>
                  </div>
                ))}
              </div>
            </>
          )}

          {aba === "previa" && (
            <>
              <div className="flex" style={{ marginBottom: 10 }}>
                <input type="date" value={diaPrevia} onChange={(e) => setDiaPrevia(e.target.value || hojeIso())} />
                <span className="muted pequeno">Mostra a agenda em edição (mesmo antes de salvar).</span>
              </div>
              <AgendaDia data={diaPrevia} agenda={ag} />
            </>
          )}

          {aba === "regras" && (
            <div className="campos">
              <Campo rotulo="Tolerância de atraso (segundos)" ajuda="Evento pontual atrasado além disso não toca.">
                <input type="number" min={5} max={300} value={ag.toleranciaSegundos} onChange={(e) => setAg({ ...ag, toleranciaSegundos: Number(e.target.value) })} />
              </Campo>
              <Campo rotulo="Detectar suspensão/relógio (segundos)" ajuda="Lacuna no relógio maior que isso = reavaliação sem tocar eventos perdidos.">
                <input type="number" min={30} max={600} value={ag.saltoSegundos} onChange={(e) => setAg({ ...ag, saltoSegundos: Number(e.target.value) })} />
              </Campo>
              <Campo rotulo="Espera máxima de evento 'Aguardar' (segundos)">
                <input type="number" min={10} max={1800} value={ag.esperaMaxSegundos} onChange={(e) => setAg({ ...ag, esperaMaxSegundos: Number(e.target.value) })} />
              </Campo>
              <div className="campo">
                <span className="rotulo">Reinício dentro de recepção/recreio</span>
                <Checa v={ag.retomarJanelaAposReinicio} onChange={(b) => setAg({ ...ag, retomarJanelaAposReinicio: b })}>
                  Retomar a playlist da janela atual (nunca toca sinais que já passaram)
                </Checa>
              </div>
            </div>
          )}

          {aba === "backup" && <Backup />}
        </div>

        <div className="cartao">
          <h2>
            Verificação {erros === 0 ? <span className="chip ok"><LuCheck /> sem erros</span> : <span className="chip erro">{erros} erro(s)</span>}
          </h2>
          <div className="avisos" style={{ maxHeight: "60vh", overflow: "auto" }}>
            {avisos.length === 0 && <div className="aviso Info">Nenhum conflito encontrado.</div>}
            {avisos.map((a, i) => (
              <div key={i} className={"aviso " + a.gravidade}>
                <b>{a.gravidade === "Erro" ? "Erro" : a.gravidade === "Alerta" ? "Atenção" : "Info"}:</b> {a.mensagem}
                {a.sugestao && <small>Sugestão: {a.sugestao}</small>}
              </div>
            ))}
          </div>
        </div>
      </div>

      {editando && <EditorEvento ev={editando} agenda={ag} salvar={gravarEvento} cancelar={() => setEditando(null)} />}
    </div>
  );
}

function Backup() {
  const { toast, recarregarConfig } = useApp();
  return (
    <div>
      <p className="muted">Salve a programação, ajustes, pastas e playlists num arquivo local (pendrive, por exemplo). O PIN nunca é exportado.</p>
      <div className="flex">
        <button
          className="btn primario"
          onClick={async () => {
            const c = await salvarArquivo("Salvar backup", `radio-escolar-backup-${hojeIso()}.json`, "json");
            if (!c) return;
            chamar("exportar_config", { caminho: c }).then(() => toast("Backup salvo.", "ok")).catch((e) => toast(e.message, "erro"));
          }}
        >
          <LuDownload /> Exportar backup
        </button>
        <button
          className="btn"
          onClick={async () => {
            const c = await escolherArquivo("Abrir backup", ["json"]);
            if (!c) return;
            try {
              await chamar("importar_config", { caminho: c });
              await recarregarConfig();
              toast("Backup importado. Recarregue esta tela para ver a programação importada.", "ok");
            } catch (e: any) {
              toast(e.message, "erro");
            }
          }}
        >
          <LuUpload /> Importar backup
        </button>
      </div>
    </div>
  );
}

function EditorEvento({ ev, agenda, salvar, cancelar }: { ev: Evento; agenda: Agenda; salvar: (e: Evento) => void; cancelar: () => void }) {
  const [e, setE] = useState<Evento>(ev);
  const [pastas, setPastas] = useState<Pasta[]>([]);
  const [playlists, setPlaylists] = useState<Playlist[]>([]);
  const [musicas, setMusicas] = useState<Faixa[]>([]);
  const [vinhetas, setVinhetas] = useState<Faixa[]>([]);
  const [recados, setRecados] = useState<Faixa[]>([]);
  useEffect(() => {
    chamar<Pasta[]>("listar_pastas").then(setPastas);
    chamar<Playlist[]>("listar_playlists").then(setPlaylists);
    chamar<Faixa[]>("listar_faixas", { texto: "", categoria: "musica", status: "ok", limite: 3000 }).then(setMusicas);
    chamar<Faixa[]>("listar_faixas", { texto: "", categoria: "vinheta", status: "ok", limite: 500 }).then(setVinhetas);
    chamar<Faixa[]>("listar_faixas", { texto: "", categoria: "recado", status: "ok", limite: 500 }).then(setRecados);
  }, []);
  const janela = e.tipo === "Recepcao" || e.tipo === "Recreio";
  const fonte = e.selecao === "FaixaFixa" ? "faixa" : e.playlistId ? "playlist" : e.pasta ? "pasta" : "todas";
  const set = (p: Partial<Evento>) => setE({ ...e, ...p });

  return (
    <Modal
      titulo={ev.nome ? `Editar: ${ev.nome}` : "Novo evento"}
      largo
      fechar={cancelar}
      rodape={
        <>
          <button className="btn" onClick={cancelar}>
            Cancelar
          </button>
          <button className="btn primario" onClick={() => salvar({ ...e, nome: e.nome.trim() || `${NOMES_TIPO[e.tipo]} ${e.inicio}` })}>
            <LuCheck /> Aplicar (depois clique em Salvar programação)
          </button>
        </>
      }
    >
      <div className="campos">
        <Campo rotulo="Nome">
          <input type="text" value={e.nome} onChange={(x) => set({ nome: x.target.value })} placeholder="Ex.: Troca 2ª aula" autoFocus />
        </Campo>
        <Campo rotulo="Tipo">
          <select
            value={e.tipo}
            onChange={(x) => {
              const t = x.target.value as TipoEvento;
              const j = t === "Recepcao" || t === "Recreio";
              set({ tipo: t, fim: j ? e.fim ?? "07:20" : null, prioridade: t === "Recreio" ? 80 : t === "Recepcao" ? 60 : t === "Recado" ? 40 : 20 });
            }}
          >
            {(Object.keys(NOMES_TIPO) as TipoEvento[]).map((t) => (
              <option key={t} value={t}>
                {NOMES_TIPO[t]}
              </option>
            ))}
          </select>
        </Campo>
        <Campo rotulo="Início">
          <input type="time" value={e.inicio} onChange={(x) => set({ inicio: x.target.value })} />
        </Campo>
        {janela && (
          <Campo rotulo="Fim das músicas" ajuda="Ex.: aula volta 09:45, música até 09:47">
            <input type="time" value={e.fim ?? ""} onChange={(x) => set({ fim: x.target.value || null })} />
          </Campo>
        )}
        <div className="campo">
          <span className="rotulo">Situação</span>
          <Checa v={e.ativo} onChange={(b) => set({ ativo: b })}>
            Evento ativo
          </Checa>
        </div>
      </div>

      <h3>Dias da semana</h3>
      <div className="dias">
        {DIAS.map((d, i) => (
          <button key={d} className={e.dias.includes(i) ? "on" : ""} onClick={() => set({ dias: e.dias.includes(i) ? e.dias.filter((x) => x !== i) : [...e.dias, i].sort() })} aria-pressed={e.dias.includes(i)}>
            {d}
          </button>
        ))}
      </div>
      <h3>Vale nos perfis de dia</h3>
      <div className="dias">
        {agenda.perfis.map((p) => (
          <button key={p.id} className={e.perfis.includes(p.id) ? "on" : ""} onClick={() => set({ perfis: e.perfis.includes(p.id) ? e.perfis.filter((x) => x !== p.id) : [...e.perfis, p.id] })}>
            {p.nome}
          </button>
        ))}
      </div>

      {(
        <>
          <h3>{e.tipo === "Recado" ? "Música depois do recado (opcional)" : "De onde vêm as músicas"}</h3>
          <div className="campos">
            <Campo rotulo="Fonte">
              <select
                value={fonte}
                onChange={(x) => {
                  const v = x.target.value;
                  if (v === "todas") set({ playlistId: null, pasta: null, faixaId: null, selecao: e.selecao === "FaixaFixa" ? "Aleatorio" : e.selecao });
                  if (v === "playlist") set({ playlistId: playlists[0]?.id ?? null, pasta: null, faixaId: null, selecao: e.selecao === "FaixaFixa" ? "Aleatorio" : e.selecao });
                  if (v === "pasta") set({ pasta: pastas.find((p) => p.categoria === "musica")?.caminho ?? null, playlistId: null, faixaId: null, selecao: e.selecao === "FaixaFixa" ? "Aleatorio" : e.selecao });
                  if (v === "faixa") set({ selecao: "FaixaFixa", faixaId: musicas[0]?.id ?? null, playlistId: null, pasta: null });
                }}
              >
                <option value="todas">{e.tipo === "Recado" ? "Nenhuma música (só o recado)" : "Todas as músicas da biblioteca"}</option>
                <option value="playlist">Uma playlist</option>
                <option value="pasta">Uma pasta</option>
                <option value="faixa">Uma música específica</option>
              </select>
            </Campo>
            {fonte === "playlist" && (
              <Campo rotulo="Playlist">
                <select value={e.playlistId ?? ""} onChange={(x) => set({ playlistId: x.target.value || null })}>
                  {playlists.map((p) => (
                    <option key={p.id} value={p.id}>
                      {p.nome}
                    </option>
                  ))}
                </select>
              </Campo>
            )}
            {fonte === "pasta" && (
              <Campo rotulo="Pasta">
                <select value={e.pasta ?? ""} onChange={(x) => set({ pasta: x.target.value || null })}>
                  {pastas
                    .filter((p) => p.categoria === "musica")
                    .map((p) => (
                      <option key={p.id} value={p.caminho}>
                        {p.caminho}
                      </option>
                    ))}
                </select>
              </Campo>
            )}
            {fonte === "faixa" && (
              <Campo rotulo="Música">
                <select value={e.faixaId ?? ""} onChange={(x) => set({ faixaId: x.target.value || null })}>
                  {musicas.map((f) => (
                    <option key={f.id} value={f.id}>
                      {f.titulo || f.nome}
                    </option>
                  ))}
                </select>
              </Campo>
            )}
            {fonte !== "faixa" && !(e.tipo === "Recado" && fonte === "todas") && (
              <Campo rotulo="Ordem" ajuda="Aleatório evita repetir as últimas tocadas">
                <select value={e.selecao} onChange={(x) => set({ selecao: x.target.value as any })}>
                  <option value="Aleatorio">Aleatória sem repetição recente</option>
                  <option value="Sequencial">Em sequência</option>
                </select>
              </Campo>
            )}
          </div>
        </>
      )}

      <h3>Vinheta e recado</h3>
      <div className="campos">
        <Campo rotulo={janela ? "Vinheta de abertura" : "Vinheta antes"}>
          <select value={e.vinhetaId ?? ""} onChange={(x) => set({ vinhetaId: x.target.value || null })}>
            <option value="">Nenhuma</option>
            {vinhetas.map((f) => (
              <option key={f.id} value={f.id}>
                {f.nome}
              </option>
            ))}
          </select>
        </Campo>
        <Campo rotulo={e.tipo === "Recado" ? "Recado ou vinheta (obrigatório)" : "Recado antes da música"} ajuda="Sequência: vinheta → recado → música. Vinheta programada: escolha uma vinheta aqui.">
          <select value={e.recadoId ?? ""} onChange={(x) => set({ recadoId: x.target.value || null })}>
            <option value="">Nenhum</option>
            {recados.map((f) => (
              <option key={f.id} value={f.id}>
                {f.nome}
              </option>
            ))}
            {e.tipo === "Recado" && vinhetas.length > 0 && (
              <optgroup label="Vinhetas (tocam sozinhas, no horário)">
                {vinhetas.map((f) => (
                  <option key={"v" + f.id} value={f.id}>
                    {f.nome}
                  </option>
                ))}
              </optgroup>
            )}
          </select>
        </Campo>
      </div>

      <h3>Volume e transições</h3>
      <div className="campos">
        <Campo rotulo="Volume deste evento">
          <Deslizante rotulo="Volume do evento" valor={e.volume} onChange={(v) => set({ volume: v })} />
        </Campo>
        <Campo rotulo="Fade-in (entrada)">
          <Deslizante rotulo="Fade-in" valor={e.fadeInMs / 1000} min={0} max={10} passo={0.5} sufixo=" s" onChange={(v) => set({ fadeInMs: Math.round(v * 1000) })} />
        </Campo>
        <Campo rotulo="Fade-out (saída)">
          <Deslizante rotulo="Fade-out" valor={e.fadeOutMs / 1000} min={0.5} max={15} passo={0.5} sufixo=" s" onChange={(v) => set({ fadeOutMs: Math.round(v * 1000) })} />
        </Campo>
        {(e.tipo === "Sinal" || e.tipo === "Saida") && (
          <Checa v={e.avisoTroca} onChange={(b) => set({ avisoTroca: b })}>Tocar o aviso de troca (ding-dong) antes da música</Checa>
        )}
        <Checa v={e.avisoAtraso} onChange={(b) => set({ avisoAtraso: b })}>Tocar o aviso de atraso (campainha) antes do evento</Checa>
        {(e.tipo === "Sinal" || e.tipo === "Saida") && (
          <Campo rotulo="Limite de duração (trecho)" ajuda="Vazio = música completa (padrão)">
            <input type="number" min={10} max={900} placeholder="música completa" value={e.limiteSegundos ?? ""} onChange={(x) => set({ limiteSegundos: x.target.value ? Number(x.target.value) : null })} />
          </Campo>
        )}
        {janela && (
          <>
            <Campo rotulo="No horário final">
              <select value={e.fimJanela} onChange={(x) => set({ fimJanela: x.target.value as any })}>
                <option value="FadeOut">Abaixar suavemente e parar (padrão)</option>
                <option value="TerminarFaixa">Deixar a música atual terminar</option>
              </select>
            </Campo>
            {e.fimJanela === "TerminarFaixa" && (
              <Campo rotulo="Teto após o horário (segundos)" ajuda="Se a música for longa, para com fade após este tempo">
                <input type="number" min={0} max={900} value={e.extraMaxSegundos} onChange={(x) => set({ extraMaxSegundos: Number(x.target.value) })} />
              </Campo>
            )}
          </>
        )}
      </div>

      <h3>Prioridade e conflitos</h3>
      <div className="campos">
        <Campo rotulo="Prioridade" ajuda="Maior vence no mesmo minuto. Padrão: recreio 80, recepção 60, recado 40, troca 20">
          <input type="number" min={0} max={100} value={e.prioridade} onChange={(x) => set({ prioridade: Number(x.target.value) })} />
        </Campo>
        <Campo rotulo="Se algo ainda estiver tocando">
          <select value={e.conflito} onChange={(x) => set({ conflito: x.target.value as any })}>
            <option value="Interromper">Encerrar o anterior com fade e tocar este (padrão)</option>
            <option value="Aguardar">Esperar o anterior terminar</option>
            <option value="Ignorar">Não tocar este</option>
          </select>
        </Campo>
        <Campo rotulo="Observação">
          <input type="text" value={e.observacao} onChange={(x) => set({ observacao: x.target.value })} />
        </Campo>
      </div>
    </Modal>
  );
}
