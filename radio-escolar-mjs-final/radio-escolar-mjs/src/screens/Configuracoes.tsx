import React, { useEffect, useState } from "react";
import { LuLock, LuSave, LuWandSparkles, LuZoomIn, LuZoomOut, LuUndo2 } from "react-icons/lu";
import { chamar, escolherArquivo } from "../api";
import type { Ajustes, Faixa } from "../tipos";
import { Cabecalho, Campo, Checa, Deslizante, useApp } from "../ui";
import EnergiaCartao from "../components/EnergiaCartao";

export default function Configuracoes() {
  const { config, recarregarConfig, toast, ir, confirmar } = useApp();
  const [a, setA] = useState<Ajustes | null>(null);
  const [vinhetas, setVinhetas] = useState<Faixa[]>([]);
  const [pin, setPin] = useState({ atual: "", novo: "", conf: "" });

  useEffect(() => {
    if (config) setA(structuredClone(config.ajustes));
  }, [config]);
  useEffect(() => {
    chamar<Faixa[]>("listar_faixas", { texto: "", categoria: "vinheta", status: "ok", limite: 300 }).then(setVinhetas);
  }, []);
  if (!a || !config) return <div className="muted">Carregando…</div>;
  const set = (p: Partial<Ajustes>) => setA({ ...a, ...p });
  const alterado = JSON.stringify(a) !== JSON.stringify(config.ajustes);

  const salvar = async () => {
    try {
      await chamar("salvar_ajustes", { ajustes: a });
      if (a.iniciarComWindows !== config.ajustes.iniciarComWindows) await chamar("definir_autostart", { ativo: a.iniciarComWindows });
      await recarregarConfig();
      toast("Configurações salvas", "ok");
    } catch (e: any) {
      toast(e.message, "erro");
    }
  };

  return (
    <div>
      <Cabecalho titulo="Configurações">
        <button className="btn" onClick={() => ir("assistente")}>
          <LuWandSparkles /> Assistente
        </button>
        <button className="btn" disabled={!alterado} onClick={() => setA(structuredClone(config.ajustes))}>
          <LuUndo2 /> Descartar
        </button>
        <button className="btn primario" disabled={!alterado} onClick={salvar}>
          <LuSave /> Salvar
        </button>
      </Cabecalho>

      <div className="grade d2">
        <div className="grade" style={{ alignContent: "start" }}>
          <div className="cartao">
            <h2>Volume e segurança auditiva</h2>
            <p className="pequeno muted">Volumes moderados e transições suaves ajudam num ambiente inclusivo, mas não existe música "universalmente adequada" — observe a reação dos estudantes e ajuste.</p>
            <div className="campos">
              <Campo rotulo="Volume geral">
                <Deslizante rotulo="Volume geral" valor={a.volumeGeral} max={a.volumeMaximo} onChange={(v) => set({ volumeGeral: v })} />
              </Campo>
              <Campo rotulo="Volume máximo permitido" ajuda="Ninguém consegue passar deste valor">
                <Deslizante rotulo="Volume máximo" valor={a.volumeMaximo} min={10} onChange={(v) => set({ volumeMaximo: v, volumeGeral: Math.min(a.volumeGeral, v) })} />
              </Campo>
              <Campo rotulo="Limite para vinhetas e beep">
                <Deslizante rotulo="Limite de vinheta" valor={a.limiteVinheta} onChange={(v) => set({ limiteVinheta: v })} />
              </Campo>
            </div>
            <h3>Volume por tipo de evento</h3>
            <div className="campos">
              {(
                [
                  ["recepcao", "Recepção"],
                  ["sinal", "Trocas de professor"],
                  ["recreio", "Recreio"],
                  ["recado", "Recados"],
                  ["vinheta", "Vinhetas"],
                  ["livre", "Rádio Livre"],
                ] as const
              ).map(([k, n]) => (
                <Campo key={k} rotulo={n}>
                  <Deslizante rotulo={n} valor={a.volumesTipo[k]} onChange={(v) => set({ volumesTipo: { ...a.volumesTipo, [k]: v } })} />
                </Campo>
              ))}
            </div>
            <p className="pequeno muted">Cada evento da programação tem volume próprio (editável no evento). Estes valores valem para Rádio Livre, recados e vinhetas avulsos, e servem de ponto de partida para novos eventos.</p>
            <h3>Transições padrão</h3>
            <p className="pequeno muted">Usadas no Rádio Livre e como ponto de partida de novos eventos (cada evento pode ter as suas).</p>
            <div className="campos">
              <Campo rotulo="Fade-in padrão">
                <Deslizante rotulo="Fade-in padrão" valor={a.fadeInPadraoMs / 1000} min={0} max={10} passo={0.5} sufixo=" s" onChange={(v) => set({ fadeInPadraoMs: Math.round(v * 1000) })} />
              </Campo>
              <Campo rotulo="Fade-out padrão">
                <Deslizante rotulo="Fade-out padrão" valor={a.fadeOutPadraoMs / 1000} min={0.5} max={15} passo={0.5} sufixo=" s" onChange={(v) => set({ fadeOutPadraoMs: Math.round(v * 1000) })} />
              </Campo>
              <Campo rotulo="Evitar repetir as últimas" ajuda="Músicas recentes ficam fora do sorteio">
                <input type="number" min={0} max={200} value={a.evitarRepeticaoUltimas} onChange={(e) => set({ evitarRepeticaoUltimas: Number(e.target.value) })} />
              </Campo>
            </div>
          </div>

          <div className="cartao">
            <h2>Recados</h2>
            <div className="campos">
              <div className="campo">
                <span className="rotulo">Antes de cada recado</span>
                <Checa v={a.beepAntesRecado} onChange={(b) => set({ beepAntesRecado: b })}>
                  Tocar beep/vinheta suave
                </Checa>
              </div>
              <Campo rotulo="Som do beep">
                <select value={a.beepFaixaId ?? ""} onChange={(e) => set({ beepFaixaId: e.target.value || null })}>
                  <option value="">Beep suave interno</option>
                  {vinhetas.map((v) => (
                    <option key={v.id} value={v.id}>
                      {v.nome}
                    </option>
                  ))}
                </select>
              </Campo>
              <Campo rotulo="Música durante o recado">
                <select value={a.duckModo} onChange={(e) => set({ duckModo: e.target.value as any })}>
                  <option value="Pausar">Pausar suavemente e retomar depois (padrão)</option>
                  <option value="Abaixar">Abaixar o volume durante o recado</option>
                </select>
              </Campo>
              {a.duckModo === "Abaixar" && (
                <Campo rotulo="Nível da música durante o recado">
                  <Deslizante rotulo="Nível durante recado" valor={a.duckNivel} max={60} onChange={(v) => set({ duckNivel: v })} />
                </Campo>
              )}
            </div>
          </div>

          <div className="cartao">
            <h2>Biblioteca</h2>
            <div className="campos">
              <div className="campo">
                <span className="rotulo">Novas pastas</span>
                <Checa v={a.indexarSubpastas} onChange={(b) => set({ indexarSubpastas: b })}>
                  Incluir subpastas por padrão
                </Checa>
              </div>
              <Campo rotulo="Reindexação automática (minutos)" ajuda="0 = desligada">
                <input type="number" min={0} max={1440} value={a.reindexarAutoMin} onChange={(e) => set({ reindexarAutoMin: Number(e.target.value) })} />
              </Campo>
            </div>
          </div>
        </div>

        <div className="grade" style={{ alignContent: "start" }}>
          <div className="cartao">
            <h2>Aparência e acessibilidade</h2>
            <div className="campos">
              <Campo rotulo="Tamanho da interface (zoom)">
                <div className="flex">
                  <button className="btn icone" onClick={() => set({ escala: Math.max(0.8, +(a.escala - 0.1).toFixed(1)) })} aria-label="Diminuir">
                    <LuZoomOut />
                  </button>
                  <b style={{ minWidth: "3.5rem", textAlign: "center" }}>{Math.round(a.escala * 100)}%</b>
                  <button className="btn icone" onClick={() => set({ escala: Math.min(1.6, +(a.escala + 0.1).toFixed(1)) })} aria-label="Aumentar">
                    <LuZoomIn />
                  </button>
                </div>
              </Campo>
              <div className="campo">
                <span className="rotulo">Contraste</span>
                <Checa v={a.altoContraste} onChange={(b) => set({ altoContraste: b })}>
                  Alto contraste
                </Checa>
              </div>
            </div>
            <h3>Logo</h3>
            <div className="flex">
              <button
                className="btn"
                onClick={async () => {
                  const c = await escolherArquivo("Escolha a logo", ["png", "jpg", "jpeg", "svg", "webp"]);
                  if (c) chamar("definir_logo", { caminho: c }).then(recarregarConfig).then(() => toast("Logo atualizada", "ok")).catch((e) => toast(e.message, "erro"));
                }}
              >
                Trocar logo (arquivo local)
              </button>
              <button className="btn" onClick={() => chamar("definir_logo", { caminho: null }).then(recarregarConfig)}>
                Voltar à logo padrão
              </button>
            </div>
          </div>

          <EnergiaCartao />
          <div className="cartao">
            <h2>Windows</h2>
            <div className="lista">
              <Checa v={a.iniciarComWindows} onChange={(b) => set({ iniciarComWindows: b })}>
                Iniciar junto com o Windows
              </Checa>
              <Checa v={a.iniciarMinimizado} onChange={(b) => set({ iniciarMinimizado: b })}>
                Ao iniciar com o Windows, ficar só na bandeja
              </Checa>
              <Checa v={a.minimizarParaBandeja} onChange={(b) => set({ minimizarParaBandeja: b })}>
                Fechar a janela mantém a rádio funcionando na bandeja (recomendado)
              </Checa>
              <Campo rotulo="Alerta de bateria baixa (%)">
                <input type="number" min={5} max={80} value={a.alertaBateria} onChange={(e) => set({ alertaBateria: Number(e.target.value) })} />
              </Campo>
            </div>
          </div>

          <div className="cartao">
            <h2>
              <LuLock /> PIN do responsável
            </h2>
            <p className="pequeno muted">
              {config.ajustes.temPin ? "PIN ativo: protege programação, configurações, remoção de pastas e importação." : "Sem PIN: qualquer pessoa pode alterar configurações."} Silenciar nunca exige PIN.
            </p>
            <div className="campos">
              {config.ajustes.temPin && (
                <Campo rotulo="PIN atual">
                  <input type="password" inputMode="numeric" value={pin.atual} onChange={(e) => setPin({ ...pin, atual: e.target.value })} />
                </Campo>
              )}
              <Campo rotulo="Novo PIN" ajuda="Ao menos 4 números. Vazio = remover PIN">
                <input type="password" inputMode="numeric" value={pin.novo} onChange={(e) => setPin({ ...pin, novo: e.target.value })} />
              </Campo>
              <Campo rotulo="Repita o novo PIN">
                <input type="password" inputMode="numeric" value={pin.conf} onChange={(e) => setPin({ ...pin, conf: e.target.value })} />
              </Campo>
              <Campo rotulo="Desbloqueio vale por (min)">
                <input type="number" min={1} max={120} value={a.pinValidadeMin} onChange={(e) => set({ pinValidadeMin: Number(e.target.value) })} />
              </Campo>
            </div>
            <div className="flex" style={{ marginTop: 10 }}>
              <button
                className="btn primario"
                onClick={async () => {
                  if (pin.novo !== pin.conf) return toast("Os PINs não conferem", "erro");
                  if (!pin.novo && !(await confirmar("Remover PIN?", "Qualquer pessoa poderá alterar as configurações.", "Remover PIN"))) return;
                  chamar("definir_pin", { atual: pin.atual, novo: pin.novo })
                    .then(() => {
                      toast(pin.novo ? "PIN definido" : "PIN removido", "ok");
                      setPin({ atual: "", novo: "", conf: "" });
                      recarregarConfig();
                    })
                    .catch((e) => toast(e.message, "erro"));
                }}
              >
                {config.ajustes.temPin ? "Alterar PIN" : "Definir PIN"}
              </button>
              {config.ajustes.temPin && (
                <button className="btn" onClick={() => chamar("bloquear").then(() => toast("Configurações bloqueadas", "ok"))}>
                  Bloquear agora
                </button>
              )}
            </div>
          </div>

          <div className="cartao">
            <h2>Dados</h2>
            <p className="pequeno muted" style={{ wordBreak: "break-all" }}>
              Pasta de dados: {config.dadosDir} {config.portatil && "(versão portátil)"} · versão {config.versao}
            </p>
            <button
              className="btn"
              onClick={async () => {
                if (await confirmar("Criar áudios de demonstração?", "Gera melodias SINTETIZADAS marcadas \"DEMO\", uma vinheta e um recado de teste (sem voz) na pasta de dados, e adiciona à biblioteca. Útil para testar antes de escolher as músicas da escola.", "Criar"))
                  chamar<string>("criar_demo").then((p) => toast(`Áudios DEMO criados em ${p}`, "ok")).catch((e) => toast(e.message, "erro"));
              }}
            >
              Criar áudios de demonstração
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
