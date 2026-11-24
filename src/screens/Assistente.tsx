import React, { useEffect, useState } from "react";
import { LuCheck, LuFolderPlus, LuSpeaker } from "react-icons/lu";
import AgendaDia from "../components/AgendaDia";
import { chamar, escolherPasta } from "../api";
import type { InfoDispositivos, Pasta } from "../tipos";
import { Checa, Deslizante, useApp } from "../ui";

const ETAPAS = ["Pasta de músicas", "Saída de áudio", "Testar volume", "Confirmar agenda", "Inicialização"];

export default function Assistente() {
  const { toast, ir, painel, recarregarConfig, config } = useApp();
  const [etapa, setEtapa] = useState(0);
  const [pastas, setPastas] = useState<Pasta[]>([]);
  const [disp, setDisp] = useState<InfoDispositivos | null>(null);
  const [volTeste, setVolTeste] = useState(20);
  const [auto, setAuto] = useState(true);

  useEffect(() => {
    chamar<Pasta[]>("listar_pastas").then(setPastas);
    chamar<InfoDispositivos>("dispositivos").then(setDisp);
  }, [etapa]);

  const concluir = async () => {
    try {
      if (auto) await chamar("definir_autostart", { ativo: true }).catch((e) => toast(e.message, "erro"));
      await chamar("concluir_assistente");
      await recarregarConfig();
      toast("Tudo pronto! A rádio seguirá a programação automaticamente.", "ok");
      ir("inicio");
    } catch (e: any) {
      toast(e.message, "erro");
    }
  };

  return (
    <div style={{ maxWidth: "60rem", margin: "0 auto" }}>
      <h1 style={{ color: "var(--marinho)", marginTop: 0 }}>Primeira configuração</h1>
      <div className="etapas">
        {ETAPAS.map((n, i) => (
          <span key={n} className={i === etapa ? "on" : i < etapa ? "feito" : ""}>
            {i + 1}. {n}
          </span>
        ))}
      </div>
      <div className="cartao">
        {etapa === 0 && (
          <>
            <h2>1. Onde estão as músicas?</h2>
            <p>Escolha a pasta do notebook com as músicas da rádio. Os arquivos não são movidos nem alterados.</p>
            <div className="flex">
              <button
                className="btn acao grande"
                onClick={async () => {
                  const c = await escolherPasta("Pasta das músicas");
                  if (c) chamar("adicionar_pasta", { caminho: c, categoria: "musica", subpastas: true }).then(() => chamar<Pasta[]>("listar_pastas").then(setPastas)).catch((e) => toast(e.message, "erro"));
                }}
              >
                <LuFolderPlus /> Escolher pasta de músicas
              </button>
              <button
                className="btn"
                onClick={() => chamar("criar_demo").then(() => { toast("Áudios DEMO criados", "ok"); chamar<Pasta[]>("listar_pastas").then(setPastas); }).catch((e) => toast(e.message, "erro"))}
              >
                Ainda não tenho músicas: criar áudios DEMO
              </button>
            </div>
            <div className="lista" style={{ marginTop: 12 }}>
              {pastas.map((p) => (
                <div key={p.id} className="linha-item">
                  <LuCheck color="var(--verde)" />
                  <span style={{ wordBreak: "break-all" }}>{p.caminho}</span>
                  <span className="chip">{p.categoria}</span>
                </div>
              ))}
            </div>
            <p className="pequeno muted">Vinhetas e recados podem ser adicionados depois, na Biblioteca.</p>
          </>
        )}
        {etapa === 1 && (
          <>
            <h2>2. Saída de áudio</h2>
            <p>
              Ligue e pareie a caixa Bluetooth no Windows. Recomendado: deixar a caixa como <b>dispositivo padrão do Windows</b>. Se preferir, escolha a saída específica.
            </p>
            <p>
              Em uso agora: <b>{painel?.audio.dispositivo || "—"}</b>
            </p>
            <div className="lista">
              <button className="linha-item" style={{ textAlign: "left", cursor: "pointer" }} onClick={() => chamar("selecionar_dispositivo", { nome: null }).then(() => toast("Usando a saída padrão do Windows", "ok"))}>
                <LuSpeaker /> <b>Padrão do Windows ({disp?.padrao ?? "nenhum"})</b> <span />
              </button>
              {disp?.saidas.map((n) => (
                <button key={n} className="linha-item" style={{ textAlign: "left", cursor: "pointer" }} onClick={() => chamar("selecionar_dispositivo", { nome: n }).then(() => toast(`Saída: ${n}`, "ok"))}>
                  <LuSpeaker /> <span>{n}</span> <span />
                </button>
              ))}
            </div>
          </>
        )}
        {etapa === 2 && (
          <>
            <h2>3. Testar volume com som suave</h2>
            <p>Toque o som de teste e ajuste o volume da caixa. Comece baixo.</p>
            <Deslizante rotulo="Volume do teste" valor={volTeste} max={60} onChange={setVolTeste} />
            <button className="btn acao grande" style={{ marginTop: 10 }} onClick={() => chamar("tocar_teste", { volume: volTeste })}>
              Tocar som de teste
            </button>
            <p className="pequeno muted">Volume máximo atual: {config?.ajustes.volumeMaximo}% (altere em Configurações).</p>
          </>
        )}
        {etapa === 3 && (
          <>
            <h2>4. Confirme a agenda de hoje</h2>
            <p className="pequeno muted">Horários padrão da escola (segunda a sexta). Edite depois em Programação. Às 09:30 e 15:30 o recreio prevalece sobre a sinalização.</p>
            <AgendaDia compacta />
          </>
        )}
        {etapa === 4 && (
          <>
            <h2>5. Iniciar com o Windows</h2>
            <Checa v={auto} onChange={setAuto}>
              Abrir a rádio automaticamente quando o notebook ligar (recomendado)
            </Checa>
            <p className="pequeno muted">Ao fechar a janela, a rádio continua na bandeja (perto do relógio do Windows). Para sair de verdade, use o ícone da bandeja → Sair.</p>
          </>
        )}
        <div className="flex" style={{ marginTop: 18 }}>
          <button className="btn" onClick={() => ir("inicio")}>
            Pular por agora
          </button>
          <span className="espaco" />
          {etapa > 0 && (
            <button className="btn" onClick={() => setEtapa(etapa - 1)}>
              Voltar
            </button>
          )}
          {etapa < ETAPAS.length - 1 ? (
            <button className="btn primario grande" onClick={() => setEtapa(etapa + 1)}>
              Continuar
            </button>
          ) : (
            <button className="btn primario grande" onClick={concluir}>
              <LuCheck /> Concluir
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
