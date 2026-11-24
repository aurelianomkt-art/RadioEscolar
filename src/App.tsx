import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  LuBatteryCharging, LuBatteryFull, LuBatteryLow, LuBluetooth, LuCalendarDays, LuHistory, LuHouse, LuLibrary, LuMegaphone,
  LuRadio, LuSettings, LuSpeaker, LuTriangleAlert, LuVolumeX, LuWifi, LuWifiOff, LuActivity, LuMusic, LuPlay,
} from "react-icons/lu";
import { chamar, dataExtenso, definirPedidoDePin, fmtContagem, modoDemo, ouvir } from "./api";
import type { ConfigCompleta, Painel, Tela } from "./tipos";
import { NOMES_TIPO } from "./tipos";
import { AppCtx, Modal } from "./ui";
import Inicio from "./screens/Inicio";
import RadioEscolar from "./screens/RadioEscolar";
import RadioLivre from "./screens/RadioLivre";
import Programacao from "./screens/Programacao";
import Biblioteca from "./screens/Biblioteca";
import Recados from "./screens/Recados";
import Historico from "./screens/Historico";
import Configuracoes from "./screens/Configuracoes";
import DiagnosticoTela from "./screens/Diagnostico";
import Assistente from "./screens/Assistente";

interface Toast { id: number; msg: string; tipo: string }

export default function App() {
  const [painel, setPainel] = useState<Painel | null>(null);
  const [config, setConfig] = useState<ConfigCompleta | null>(null);
  const [tela, setTela] = useState<Tela>("inicio");
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [logo, setLogo] = useState("/emblema.png");
  const [pin, setPin] = useState<{ resolver: (b: boolean) => void } | null>(null);
  const [pinTexto, setPinTexto] = useState("");
  const [pinErro, setPinErro] = useState("");
  const [conf, setConf] = useState<{ titulo: string; texto: string; ok: string; resolver: (b: boolean) => void } | null>(null);
  const idToast = useRef(0);
  const [claro, setClaro] = useState(() => {
    try {
      return localStorage.getItem("rme.modoClaro") === "1";
    } catch {
      return false;
    }
  });
  useEffect(() => {
    document.body.classList.toggle("claro", claro);
    try {
      localStorage.setItem("rme.modoClaro", claro ? "1" : "0");
    } catch {
      /* sem armazenamento: o modo vale só nesta sessão */
    }
  }, [claro]);

  const toast = useCallback((msg: string, tipo: "ok" | "erro" | "info" = "info") => {
    const id = ++idToast.current;
    setToasts((t) => [...t.slice(-3), { id, msg, tipo }]);
    setTimeout(() => setToasts((t) => t.filter((x) => x.id !== id)), tipo === "erro" ? 7000 : 4000);
  }, []);

  const recarregarConfig = useCallback(async () => {
    const c = await chamar<ConfigCompleta>("obter_config");
    setConfig(c);
    const l = await chamar<string | null>("obter_logo");
    setLogo(l || "/emblema.png");
  }, []);

  const confirmar = useCallback(
    (titulo: string, texto: string, ok = "Confirmar") => new Promise<boolean>((resolver) => setConf({ titulo, texto, ok, resolver })),
    []
  );

  useEffect(() => {
    definirPedidoDePin(
      () =>
        new Promise<boolean>((resolver) => {
          setPinTexto("");
          setPinErro("");
          setPin({ resolver });
        })
    );
    recarregarConfig().catch((e) => toast(String(e.message || e), "erro"));
    chamar<Painel>("painel_agora").then(setPainel).catch(() => {});
    let desligar: (() => void)[] = [];
    ouvir<Painel>("painel", setPainel).then((f) => desligar.push(f));
    ouvir<any>("indexacao", (r) => {
      if (r) toast(`Biblioteca atualizada: ${r.novasOuAlteradas} nova(s)/alterada(s), ${r.ausentes} ausente(s), ${r.invalidas} inválida(s).`, r.pastasInacessiveis?.length ? "erro" : "ok");
    }).then((f) => desligar.push(f));
    return () => desligar.forEach((f) => f());
  }, []);

  // Primeira execução: abre o assistente.
  const assistenteMostrado = useRef(false);
  useEffect(() => {
    if (config && !config.ajustes.assistenteConcluido && !assistenteMostrado.current) {
      assistenteMostrado.current = true;
      setTela("assistente");
    }
  }, [config]);

  // Escala da interface e contraste.
  useEffect(() => {
    const e = config?.ajustes.escala ?? 1;
    document.documentElement.style.fontSize = `${16 * e}px`;
    document.body.classList.toggle("alto-contraste", !!config?.ajustes.altoContraste);
  }, [config?.ajustes.escala, config?.ajustes.altoContraste]);

  const trocarModo = useCallback(
    async (m: "Escolar" | "Livre") => {
      try {
        await chamar("definir_modo", { modo: m, confirmar: false });
      } catch (e: any) {
        if (e.message === "CONFIRMAR") {
          const ok = await confirmar(
            m === "Livre" ? "Ativar Rádio Livre?" : "Voltar ao Rádio Escolar?",
            m === "Livre"
              ? "Há áudio tocando agora. Ele será encerrado com fade e a programação automática ficará SUSPENSA até você voltar ao Rádio Escolar."
              : "Há áudio tocando no Rádio Livre. Ele será encerrado com fade. Eventos que já passaram NÃO serão tocados; só o que ainda estiver valendo.",
            m === "Livre" ? "Ativar Rádio Livre" : "Voltar ao Rádio Escolar"
          );
          if (!ok) return;
          await chamar("definir_modo", { modo: m, confirmar: true });
        } else {
          toast(e.message, "erro");
          return;
        }
      }
      setTela(m === "Livre" ? "livre" : "inicio");
      toast(m === "Livre" ? "Rádio Livre ativo. Programação automática suspensa." : "Rádio Escolar ativo.", "ok");
    },
    [confirmar, toast]
  );

  const silenciar = useCallback(() => {
    chamar("silenciar_agora").catch(() => {});
  }, []);

  // Atalhos: Alt+S = Silenciar agora; Espaço = tocar/pausar (fora de campos de texto).
  useEffect(() => {
    const h = (e: KeyboardEvent) => {
      const alvo = e.target as HTMLElement;
      const emCampo = ["INPUT", "SELECT", "TEXTAREA"].includes(alvo?.tagName);
      if (e.altKey && (e.key === "s" || e.key === "S")) {
        e.preventDefault();
        silenciar();
      } else if (e.code === "Space" && !emCampo && alvo?.tagName !== "BUTTON") {
        e.preventDefault();
        chamar("tocar_pausar").catch((x) => toast(x.message, "erro"));
      }
    };
    window.addEventListener("keydown", h);
    return () => window.removeEventListener("keydown", h);
  }, [silenciar, toast]);

  const ctx = useMemo(() => ({ painel, config, recarregarConfig, toast, ir: setTela, confirmar, logo, trocarModo }), [painel, config, recarregarConfig, toast, confirmar, logo, trocarModo]);

  const p = painel;
  const modo = p?.modo ?? "Escolar";
  const prox = p?.proximos.find((x) => !x.suprimidoPor) ?? p?.proximos[0];

  let conteudo: React.ReactNode = null;
  switch (tela) {
    case "inicio": conteudo = <Inicio />; break;
    case "escolar": conteudo = <RadioEscolar />; break;
    case "livre": conteudo = <RadioLivre />; break;
    case "programacao": conteudo = <Programacao />; break;
    case "biblioteca": conteudo = <Biblioteca />; break;
    case "recados": conteudo = <Recados />; break;
    case "historico": conteudo = <Historico />; break;
    case "config": conteudo = <Configuracoes />; break;
    case "diagnostico": conteudo = <DiagnosticoTela />; break;
    case "assistente": conteudo = <Assistente />; break;
  }

  const bat = p?.sistema.bateria;
  const wifi = p?.sistema.wifi;
  const aud = p?.audio;

  return (
    <AppCtx.Provider value={ctx}>
      <div className="app">
        <header className="topo">
          <button className="marca" onClick={() => setTela("inicio")} aria-label="Ir ao painel">
            <img src={logo} alt="Logo da Rádio Escolar" onError={() => setLogo("/emblema.png")} />
            <span className="nome">
              Rádio Escolar
              <small>Mestre João Silvério</small>
            </span>
          </button>
          <div className="relogio" aria-live="off">
            <div className="hora">{p?.agora ?? "--:--:--"}</div>
            <div className="data">{p ? dataExtenso(p.dataIso) : ""}</div>
          </div>
          <div className="status-sis">
            <span
              className={"pilula " + (aud ? (aud.dispositivoOk ? "ok" : "erro") : "")}
              title={aud?.dispositivoPadraoWindows ? "Usando a saída padrão do Windows" : "Saída escolhida nas configurações"}
            >
              {aud?.dispositivo?.toLowerCase().includes("bluetooth") ? <LuBluetooth /> : <LuSpeaker />}
              {aud ? (aud.dispositivoOk ? aud.dispositivo || "Saída de áudio" : "Saída de áudio PERDIDA") : "Saída: verificando…"}
            </span>
            <span className={"pilula " + (wifi?.disponivel ? (wifi.conectado ? "ok" : "alerta") : "")}>
              {wifi?.disponivel && wifi.conectado ? <LuWifi /> : <LuWifiOff />}
              {!wifi?.disponivel ? "Wi-Fi: indisponível" : wifi.conectado ? `${wifi.ssid ?? "Wi-Fi"}${wifi.sinal != null ? ` · ${wifi.sinal}%` : ""}` : "Wi-Fi desconectado"}
            </span>
            <span
              className={
                "pilula " +
                (bat?.disponivel && bat.percentual != null ? (bat.carregando === false && bat.percentual <= (config?.ajustes.alertaBateria ?? 25) ? "erro" : "ok") : "")
              }
            >
              {bat?.carregando ? <LuBatteryCharging /> : bat?.percentual != null && bat.percentual <= 25 ? <LuBatteryLow /> : <LuBatteryFull />}
              {!bat?.disponivel ? "Bateria: indisponível" : bat.semBateria ? "Na tomada (sem bateria)" : `${bat.percentual ?? "?"}%${bat.carregando ? " · carregando" : bat.carregando === false ? " · na bateria" : ""}`}
            </span>
          </div>
          <button className="silenciar" onClick={silenciar} title="Silenciar agora (Alt+S)">
            <LuVolumeX size={22} /> SILENCIAR AGORA
          </button>
        </header>

        <div>
          <div className="modos" role="group" aria-label="Modo de operação">
            <button className={"botao-modo escolar" + (modo === "Escolar" ? " ativo" : "")} onClick={() => (modo === "Escolar" ? setTela("escolar") : trocarModo("Escolar"))} aria-pressed={modo === "Escolar"}>
              <LuRadio size={22} /> Rádio Escolar {modo === "Escolar" && <span className="tag">MODO ATIVO</span>}
            </button>
            <button className={"botao-modo livre" + (modo === "Livre" ? " ativo" : "")} onClick={() => (modo === "Livre" ? setTela("livre") : trocarModo("Livre"))} aria-pressed={modo === "Livre"}>
              <LuMusic size={22} /> Rádio Livre {modo === "Livre" && <span className="tag">MODO ATIVO</span>}
            </button>
          </div>
          {modoDemo && (
            <div className="faixa-aviso amarelo">
              <LuTriangleAlert /> PROTÓTIPO / DEMONSTRAÇÃO: dados fictícios, sem áudio real. Esta visualização não substitui o aplicativo instalado.
            </div>
          )}
          {p?.silenciado && (
            <div className="faixa-aviso vermelho" role="alert">
              <LuVolumeX /> RÁDIO SILENCIADA. Nada tocará (nem a programação) até alguém tocar em Retomar.
              <span className="espaco" />
              <button className="btn primario" onClick={() => chamar("retomar").then(() => toast("Rádio retomada", "ok"))}>
                <LuPlay /> Retomar
              </button>
            </div>
          )}
          {modo === "Livre" && (
            <div className="faixa-aviso laranja" role="status">
              <LuTriangleAlert /> Programação automática suspensa (Rádio Livre ativo). Os horários não serão tocados até voltar ao Rádio Escolar.
            </div>
          )}
          {p?.alertas.map((a) => (
            <div key={a} className="faixa-aviso vermelho" role="alert">
              <LuTriangleAlert /> {a}
              <span className="espaco" />
              {a.startsWith("Saída de áudio") && (
                <button className="btn" onClick={() => setTela("diagnostico")}>
                  Resolver
                </button>
              )}
              <button className="btn" onClick={() => chamar("limpar_alertas")}>
                Ciente
              </button>
            </div>
          ))}
        </div>

        <main className="conteudo">{conteudo}</main>

        <footer className="rodape">
          <div className="proximo">
            {prox ? (
              <>
                <div className="cont" aria-label="Contagem regressiva">{fmtContagem(prox.emSegundos)}</div>
                <div className="info">
                  <div>
                    Próximo: <b>{prox.nome}</b> às <b>{prox.inicio}</b>
                    {prox.data !== p?.dataIso ? ` (${prox.data.split("-").reverse().join("/")})` : ""} · {NOMES_TIPO[prox.tipo]}
                  </div>
                  <div className="pequeno muted">
                    {modo === "Livre"
                      ? "Agendador suspenso (Rádio Livre)"
                      : p?.silenciado
                      ? "Agendador bloqueado: rádio silenciada"
                      : p?.audio.dispositivoOk === false
                      ? "Agendador bloqueado: sem saída de áudio"
                      : "Agendador ativo"}
                    {p?.avisosConfig.length ? ` · ⚠ ${p.avisosConfig[0]}` : ""}
                  </div>
                </div>
              </>
            ) : (
              <div className="muted">Nenhum evento nos próximos 14 dias. Confira Programação e Calendário.</div>
            )}
          </div>
          <nav className="nav" aria-label="Navegação">
            {[
              ["inicio", "Painel", LuHouse],
              ["programacao", "Programação", LuCalendarDays],
              ["biblioteca", "Biblioteca", LuLibrary],
              ["recados", "Recados", LuMegaphone],
              ["historico", "Histórico", LuHistory],
              ["diagnostico", "Áudio", LuActivity],
              ["config", "Configurações", LuSettings],
            ].map(([t, n, I]: any) => (
              <button key={t} className={tela === t ? "on" : ""} onClick={() => setTela(t)}>
                <I />
                {n}
              </button>
            ))}
          </nav>
          <button className="btn" onClick={() => setClaro((c) => !c)} aria-pressed={claro} title="Alternar entre modo escuro e claro">
            {claro ? "Modo escuro" : "Modo claro"}
          </button>
          <div className="credito">Powered by Aureliano Amaral</div>
        </footer>
      </div>

      {modoDemo && <div className="demo-selo">DEMO</div>}

      <div className="toasts" aria-live="polite">
        {toasts.map((t) => (
          <div key={t.id} className={"toast " + t.tipo}>
            {t.msg}
          </div>
        ))}
      </div>

      {conf && (
        <Modal
          titulo={conf.titulo}
          fechar={() => {
            conf.resolver(false);
            setConf(null);
          }}
          rodape={
            <>
              <button className="btn" onClick={() => { conf.resolver(false); setConf(null); }}>
                Cancelar
              </button>
              <button className="btn primario" autoFocus onClick={() => { conf.resolver(true); setConf(null); }}>
                {conf.ok}
              </button>
            </>
          }
        >
          <p style={{ lineHeight: 1.5 }}>{conf.texto}</p>
        </Modal>
      )}

      {pin && (
        <Modal
          titulo="PIN necessário"
          fechar={() => { pin.resolver(false); setPin(null); }}
          rodape={
            <>
              <button className="btn" onClick={() => { pin.resolver(false); setPin(null); }}>
                Cancelar
              </button>
              <button
                className="btn primario"
                onClick={async () => {
                  const ok = await chamar<boolean>("desbloquear", { pin: pinTexto });
                  if (ok) {
                    pin.resolver(true);
                    setPin(null);
                  } else setPinErro("PIN incorreto.");
                }}
              >
                Desbloquear
              </button>
            </>
          }
        >
          <p className="muted">Esta alteração é protegida. Digite o PIN do responsável. (Silenciar nunca exige PIN.)</p>
          <input
            type="password"
            inputMode="numeric"
            autoFocus
            value={pinTexto}
            onChange={(e) => setPinTexto(e.target.value)}
            onKeyDown={async (e) => {
              if (e.key === "Enter") {
                const ok = await chamar<boolean>("desbloquear", { pin: pinTexto });
                if (ok) { pin.resolver(true); setPin(null); } else setPinErro("PIN incorreto.");
              }
            }}
            style={{ fontSize: "1.4rem", letterSpacing: "0.3em", width: "100%" }}
            aria-label="PIN"
          />
          {pinErro && <p style={{ color: "var(--vermelho)", fontWeight: 600 }}>{pinErro}</p>}
        </Modal>
      )}
    </AppCtx.Provider>
  );
}
