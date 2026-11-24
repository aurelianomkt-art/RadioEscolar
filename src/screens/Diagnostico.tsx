import React, { useEffect, useState } from "react";
import { LuRefreshCw, LuSpeaker, LuTriangleAlert, LuCheck } from "react-icons/lu";
import { chamar } from "../api";
import type { Diagnostico } from "../tipos";
import { Cabecalho, Deslizante, useApp } from "../ui";

export default function DiagnosticoTela() {
  const { toast, painel, confirmar } = useApp();
  const [d, setD] = useState<Diagnostico | null>(null);
  const [teste, setTeste] = useState(25);
  const carregar = () => chamar<Diagnostico>("diagnostico").then(setD).catch((e) => toast(e.message, "erro"));
  useEffect(() => {
    carregar();
  }, []);
  const a = painel?.audio;
  if (!d) return <div className="muted">Lendo dispositivos…</div>;

  const escolher = async (nome: string | null) => {
    const interno = nome && /alto-falante|speaker|realtek|interno/i.test(nome) && !/bluetooth/i.test(nome);
    if (interno && !(await confirmar("Usar os alto-falantes do notebook?", `"${nome}" parece ser o alto-falante interno, não a caixa da escola. Confirmar mesmo assim?`, "Usar este"))) return;
    if (a && !a.dispositivoOk) chamar("confirmar_dispositivo", { nome });
    else chamar("selecionar_dispositivo", { nome });
    toast(`Saída: ${nome ?? "padrão do Windows"}`, "ok");
    setTimeout(carregar, 800);
  };

  return (
    <div>
      <Cabecalho titulo="Diagnóstico do áudio">
        <button className="btn" onClick={carregar}>
          <LuRefreshCw /> Atualizar
        </button>
      </Cabecalho>
      {a && !a.dispositivoOk && (
        <div className="aviso Erro" style={{ marginBottom: 12 }}>
          <b>Saída de áudio perdida.</b> A rádio foi pausada e não mudou sozinha para outro alto-falante. Ligue/reconecte a caixa Bluetooth no Windows e escolha abaixo a saída para confirmar. A música
          continua pausada até alguém tocar Play.
        </div>
      )}
      <div className="grade d2">
        <div className="cartao">
          <h2>
            <LuSpeaker /> Saída de áudio
          </h2>
          <p>
            Atual: <b>{a?.dispositivo || "—"}</b> {a?.dispositivoOk ? <span className="chip ok"><LuCheck /> funcionando</span> : <span className="chip erro">indisponível</span>}
            <br />
            <span className="pequeno muted">
              {a?.dispositivoPadraoWindows ? "Seguindo o dispositivo padrão do Windows." : "Dispositivo escolhido explicitamente."} Padrão do Windows agora: {d.dispositivos.padrao ?? "nenhum"}.
            </span>
          </p>
          {d.dicaHandsFree && (
            <div className="aviso Alerta">
              <LuTriangleAlert /> O nome indica perfil "Hands-Free/Headset" do Bluetooth, que tem qualidade baixa (telefone). No Windows, escolha a saída "Fones de ouvido/Alto-falante (estéreo)" da mesma caixa.
            </div>
          )}
          <h3>Escolher saída</h3>
          <div className="lista">
            <button className={"linha-item"} style={{ textAlign: "left", cursor: "pointer" }} onClick={() => escolher(null)}>
              <LuSpeaker />
              <span>
                <b>Padrão do Windows</b> <span className="pequeno muted">(recomendado se a caixa Bluetooth for a saída padrão)</span>
              </span>
              {a?.dispositivoPadraoWindows && <span className="chip ok">em uso</span>}
            </button>
            {d.dispositivos.saidas.map((n) => (
              <button key={n} className={"linha-item"} style={{ textAlign: "left", cursor: "pointer" }} onClick={() => escolher(n)}>
                <LuSpeaker />
                <span>{n}</span>
                {!a?.dispositivoPadraoWindows && a?.dispositivo === n && <span className="chip ok">em uso</span>}
              </button>
            ))}
          </div>
          <h3>Testar volume com som suave</h3>
          <Deslizante rotulo="Volume do teste" valor={teste} max={60} onChange={setTeste} />
          <button className="btn acao" style={{ marginTop: 8 }} onClick={() => chamar("tocar_teste", { volume: teste })}>
            Tocar som de teste
          </button>
        </div>
        <div className="cartao">
          <h2>Informações</h2>
          <table className="tabela">
            <tbody>
              <tr><td>Versão</td><td>{d.versao}{d.portatil ? " (portátil)" : ""}</td></tr>
              <tr><td>Sistema de áudio</td><td>{d.dispositivos.host}</td></tr>
              <tr><td>Saídas encontradas</td><td>{d.dispositivos.saidas.length}</td></tr>
              <tr><td>Microfones encontrados</td><td>{d.dispositivos.entradas.length}</td></tr>
              <tr><td>Parece Bluetooth?</td><td>{d.pareceBluetooth ? "Sim (pelo nome)" : "Não identificado pelo nome"}</td></tr>
              <tr><td>Última verificação do agendador</td><td>{d.ultimoTick ?? "—"}</td></tr>
              <tr><td>Bateria</td><td>{d.sistema.bateria.disponivel ? (d.sistema.bateria.semBateria ? "Sem bateria (tomada)" : `${d.sistema.bateria.percentual ?? "?"}% ${d.sistema.bateria.carregando ? "carregando" : ""}`) : "indisponível"}</td></tr>
              <tr><td>Wi-Fi</td><td>{d.sistema.wifi.disponivel ? (d.sistema.wifi.conectado ? `${d.sistema.wifi.ssid ?? ""} ${d.sistema.wifi.sinal ?? ""}%` : "desconectado") : "indisponível"} (a rádio não precisa de internet)</td></tr>
              <tr><td>Banco local</td><td>{(d.bancoBytes / 1024).toFixed(0)} KB</td></tr>
              <tr><td>Pasta de dados</td><td style={{ wordBreak: "break-all" }}>{d.dadosDir}</td></tr>
              <tr>
                <td>Biblioteca</td>
                <td>{d.biblioteca.map(([c, s, n]) => `${c}/${s}: ${n}`).join(" · ") || "vazia"}</td>
              </tr>
            </tbody>
          </table>
          <p className="pequeno muted">O status do Bluetooth é inferido pela presença do dispositivo de saída no Windows. Bateria e Wi-Fi vêm do Windows; quando não há leitura confiável, aparece "indisponível".</p>
        </div>
      </div>
    </div>
  );
}
