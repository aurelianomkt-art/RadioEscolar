import React, { useEffect, useState } from "react";
import { LuBellRing, LuCalendarDays, LuMegaphone, LuMic, LuMicOff, LuPlay } from "react-icons/lu";
import { chamar, fmtTempo } from "../api";
import type { Evento, Faixa, InfoDispositivos } from "../tipos";
import { DIAS } from "../tipos";
import { Cabecalho, Campo, Checa, Deslizante, Modal, useApp } from "../ui";

export default function Recados() {
  const { toast, painel, config, recarregarConfig, confirmar, ir } = useApp();
  const [recados, setRecados] = useState<Faixa[]>([]);
  const [vinhetas, setVinhetas] = useState<Faixa[]>([]);
  const [vinhetaSel, setVinhetaSel] = useState<string>("");
  const [beep, setBeep] = useState(true);
  const [agendar, setAgendar] = useState<{ recado: Faixa; inicio: string; dias: number[] } | null>(null);
  const [disp, setDisp] = useState<InfoDispositivos | null>(null);
  const [mic, setMic] = useState<string>("");
  const [ganho, setGanho] = useState(config?.ajustes.micGanho ?? 50);
  const [portao, setPortao] = useState(config?.ajustes.micPortao ?? 8);

  const carregar = () => {
    chamar<Faixa[]>("listar_faixas", { texto: "", categoria: "recado", status: "", limite: 500 }).then(setRecados);
    chamar<Faixa[]>("listar_faixas", { texto: "", categoria: "vinheta", status: "", limite: 500 }).then(setVinhetas);
  };
  useEffect(() => {
    carregar();
    chamar<InfoDispositivos>("dispositivos").then(setDisp);
  }, []);
  useEffect(() => setBeep(config?.ajustes.beepAntesRecado ?? true), [config?.ajustes.beepAntesRecado]);

  const tocar = async (r: Faixa | null) => {
    const musica = painel?.audio.tocando;
    if (musica && !(await confirmar("Tocar recado agora?", `A música será ${config?.ajustes.duckModo === "Abaixar" ? "abaixada" : "pausada"} suavemente e volta sozinha depois do recado.`, "Tocar recado"))) return;
    chamar("tocar_recado", { recadoId: r?.id ?? null, vinhetaId: vinhetaSel || null, beep })
      .then(() => toast("Recado no ar", "ok"))
      .catch((e) => toast(e.message, "erro"));
  };

  const salvarAgendamento = async () => {
    if (!agendar || !config) return;
    const ev: Evento = {
      id: `recado-${Date.now().toString(36)}`, nome: `Recado: ${agendar.recado.nome}`, tipo: "Recado", ativo: true, dias: agendar.dias, perfis: ["normal"],
      inicio: agendar.inicio, fim: null, playlistId: null, pasta: null, faixaId: null, selecao: "Aleatorio", volume: config.ajustes.volumesTipo.recado,
      fadeInMs: 300, fadeOutMs: 1500, limiteSegundos: null, fimJanela: "FadeOut", extraMaxSegundos: 240, vinhetaId: vinhetaSel || null,
      recadoId: agendar.recado.id, prioridade: 40, conflito: "Aguardar", observacao: "Criado pela tela Recados",
      avisoTroca: false, avisoAtraso: false,
    };
    try {
      await chamar("salvar_agenda", { agenda: { ...config.agenda, eventos: [...config.agenda.eventos, ev] } });
      await recarregarConfig();
      toast("Recado agendado. Confira em Programação.", "ok");
      setAgendar(null);
    } catch (e: any) {
      toast(e.message, "erro");
    }
  };

  const a = painel?.audio;

  return (
    <div>
      <Cabecalho titulo="Recados e vinhetas" />
      <div className="grade d2">
        <div className="grade" style={{ alignContent: "start" }}>
          <div className="cartao">
            <h2>
              <LuMegaphone /> Recados
            </h2>
            <div className="flex" style={{ marginBottom: 10 }}>
              <Checa v={beep} onChange={setBeep}>
                Beep suave antes
              </Checa>
              <select value={vinhetaSel} onChange={(e) => setVinhetaSel(e.target.value)} aria-label="Vinheta antes do recado">
                <option value="">Sem vinheta (usa o beep, se marcado)</option>
                {vinhetas.filter((v) => v.status === "ok").map((v) => (
                  <option key={v.id} value={v.id}>
                    Vinheta: {v.nome}
                  </option>
                ))}
              </select>
            </div>
            <div className="lista">
              {recados.length === 0 && (
                <div className="vazio">
                  Nenhum recado. Adicione uma pasta com categoria "Recados" na <button className="btn" onClick={() => ir("biblioteca")}>Biblioteca</button>
                </div>
              )}
              {recados.map((r) => (
                <div key={r.id} className="linha-item" style={{ gridTemplateColumns: "1fr 9rem auto" }}>
                  <span style={{ minWidth: 0 }}>
                    <b>{r.nome}</b>
                    <div className="pequeno muted">
                      {fmtTempo(r.duracaoMs)} · {r.formato.toUpperCase()} {r.status !== "ok" && <span className="chip erro">{r.status}</span>}
                    </div>
                  </span>
                  <input
                    type="range"
                    min={0}
                    max={100}
                    defaultValue={r.volume}
                    onChange={(e) => chamar("definir_volume_faixa", { id: r.id, volume: Number(e.target.value) })}
                    aria-label={`Volume individual de ${r.nome}`}
                    title="Volume individual"
                  />
                  <span className="flex" style={{ flexWrap: "nowrap" }}>
                    <button className="btn acao" disabled={r.status !== "ok"} onClick={() => tocar(r)}>
                      <LuPlay /> Tocar agora
                    </button>
                    <button className="btn icone" disabled={r.status !== "ok"} onClick={() => setAgendar({ recado: r, inicio: "10:00", dias: [0, 1, 2, 3, 4] })} aria-label="Agendar">
                      <LuCalendarDays />
                    </button>
                  </span>
                </div>
              ))}
            </div>
          </div>

          <div className="cartao">
            <h2>
              <LuBellRing /> Vinhetas
            </h2>
            <p className="pequeno muted">Volume das vinhetas limitado a {config?.ajustes.limiteVinheta}% (Configurações). Use uma vinheta como "beep" padrão antes de recados.</p>
            <div className="lista">
              {vinhetas.length === 0 && <div className="vazio">Nenhuma vinheta cadastrada. O beep suave interno é usado por padrão.</div>}
              {vinhetas.map((v) => (
                <div key={v.id} className="linha-item" style={{ gridTemplateColumns: "1fr 9rem auto" }}>
                  <span>
                    <b>{v.nome}</b> <span className="pequeno muted">· {fmtTempo(v.duracaoMs)}</span>
                    {config?.ajustes.beepFaixaId === v.id && <span className="chip ok" style={{ marginLeft: 6 }}>beep padrão</span>}
                  </span>
                  <input type="range" min={0} max={100} defaultValue={v.volume} onChange={(e) => chamar("definir_volume_faixa", { id: v.id, volume: Number(e.target.value) })} aria-label={`Volume de ${v.nome}`} />
                  <span className="flex" style={{ flexWrap: "nowrap" }}>
                    <button className="btn" disabled={v.status !== "ok"} onClick={() => chamar("tocar_recado", { recadoId: null, vinhetaId: v.id, beep: false }).catch((e) => toast(e.message, "erro"))}>
                      <LuPlay /> Tocar
                    </button>
                    <button
                      className="btn"
                      onClick={() =>
                        config &&
                        chamar("salvar_ajustes", { ajustes: { ...config.ajustes, beepFaixaId: config.ajustes.beepFaixaId === v.id ? null : v.id } })
                          .then(recarregarConfig)
                          .catch((e) => toast(e.message, "erro"))
                      }
                    >
                      {config?.ajustes.beepFaixaId === v.id ? "Usar beep interno" : "Usar como beep"}
                    </button>
                  </span>
                </div>
              ))}
            </div>
          </div>
        </div>

        <div className="cartao" style={{ alignSelf: "start" }}>
          <h2>
            <LuMic /> Microfone ao vivo (opcional)
          </h2>
          <p className="pequeno muted" style={{ lineHeight: 1.5 }}>
            Nunca liga sozinho. Ao falar, a música é pausada (ou abaixada) e volta depois. Para evitar microfonia: fique longe da caixa de som, comece com ganho baixo e use fone se possível.
            Se o nível ficar saturado por mais de 1,5 s, o microfone é desligado automaticamente. O Windows pode pedir permissão de microfone.
          </p>
          <div className="campos">
            <Campo rotulo="Microfone">
              <select value={mic} onChange={(e) => setMic(e.target.value)}>
                <option value="">Padrão do Windows{disp?.entradaPadrao ? ` (${disp.entradaPadrao})` : ""}</option>
                {disp?.entradas.map((n) => (
                  <option key={n} value={n}>
                    {n}
                  </option>
                ))}
              </select>
            </Campo>
            <Campo rotulo="Ganho">
              <Deslizante rotulo="Ganho do microfone" valor={ganho} onChange={(v) => { setGanho(v); chamar("mic_ajuste", { ganho: v, portao }); }} />
            </Campo>
            <Campo rotulo="Portão de ruído" ajuda="Corta chiado de fundo quando ninguém fala">
              <Deslizante rotulo="Portão de ruído" valor={portao} max={40} onChange={(v) => { setPortao(v); chamar("mic_ajuste", { ganho, portao: v }); }} />
            </Campo>
          </div>
          <div className="flex" style={{ marginTop: 10 }}>
            {!a?.micAberto ? (
              <button className="btn" onClick={() => chamar("mic_abrir", { nome: mic || null })}>
                <LuMic /> Abrir microfone (só monitorar nível)
              </button>
            ) : (
              <button className="btn" onClick={() => chamar("mic_fechar")}>
                <LuMicOff /> Fechar microfone
              </button>
            )}
          </div>
          <div style={{ marginTop: 12 }}>
            <span className="rotulo">Nível de entrada {a?.micDispositivo ? `— ${a.micDispositivo}` : ""}</span>
            <div className="medidor" style={{ marginTop: 6 }}>
              <div style={{ width: `${Math.min(100, (a?.micNivel ?? 0) * 100)}%` }} />
            </div>
          </div>
          {a?.erroMic && <div className="aviso Erro" style={{ marginTop: 10 }}>{a.erroMic}</div>}
          <button
            className={"btn grande " + (a?.micFalando ? "perigo" : "acao")}
            style={{ marginTop: 14, width: "100%" }}
            disabled={!a?.micAberto}
            onClick={() => chamar("mic_falar", { falar: !a?.micFalando }).catch((e) => toast(e.message, "erro"))}
          >
            {a?.micFalando ? (
              <>
                <LuMicOff /> PARAR DE FALAR (no ar)
              </>
            ) : (
              <>
                <LuMic /> Falar ao vivo
              </>
            )}
          </button>
        </div>
      </div>

      {agendar && (
        <Modal
          titulo={`Agendar: ${agendar.recado.nome}`}
          fechar={() => setAgendar(null)}
          rodape={
            <>
              <button className="btn" onClick={() => setAgendar(null)}>
                Cancelar
              </button>
              <button className="btn primario" onClick={salvarAgendamento}>
                Agendar
              </button>
            </>
          }
        >
          <div className="campos">
            <Campo rotulo="Horário">
              <input type="time" value={agendar.inicio} onChange={(e) => setAgendar({ ...agendar, inicio: e.target.value })} />
            </Campo>
          </div>
          <h3>Dias</h3>
          <div className="dias">
            {DIAS.map((d, i) => (
              <button key={d} className={agendar.dias.includes(i) ? "on" : ""} onClick={() => setAgendar({ ...agendar, dias: agendar.dias.includes(i) ? agendar.dias.filter((x) => x !== i) : [...agendar.dias, i] })}>
                {d}
              </button>
            ))}
          </div>
          <p className="pequeno muted">Se houver música tocando no horário, o recado espera ela terminar (até 5 min). Vinheta escolhida acima é incluída. Ajuste detalhes em Programação.</p>
        </Modal>
      )}
    </div>
  );
}
