import React, { useState } from "react";
import { LuPower, LuSettings } from "react-icons/lu";
import { chamar } from "../api";
import { Campo, Checa, useApp } from "../ui";

const K_TAMPA = "rme.tampaManter";
const K_DESLIGAR = "rme.desligarEm";

function ler(chave: string): string | null {
  try {
    return localStorage.getItem(chave);
  } catch {
    return null;
  }
}
function gravar(chave: string, valor: string | null) {
  try {
    if (valor == null) localStorage.removeItem(chave);
    else localStorage.setItem(chave, valor);
  } catch {
    /* sem armazenamento: a configuração vale só nesta sessão */
  }
}

/** Energia do notebook: tampa fechada, desligamento diário e atalho para as configurações do Windows. */
export default function EnergiaCartao() {
  const { toast } = useApp();
  const [tampa, setTampa] = useState(ler(K_TAMPA) === "1");
  const [desligarEm, setDesligarEm] = useState<string | null>(ler(K_DESLIGAR));
  const [hora, setHora] = useState(ler(K_DESLIGAR) ?? "23:00");

  const aplicarTampa = (manter: boolean) =>
    chamar("energia_tampa", { manter })
      .then(() => {
        setTampa(manter);
        gravar(K_TAMPA, manter ? "1" : "0");
        toast(manter ? "Com a tampa fechada, a rádio continua tocando" : "Tampa voltou ao padrão do Windows (suspender)", "ok");
      })
      .catch((e) => toast(e.message, "erro"));

  const ativarDesligar = () =>
    chamar("energia_desligar_em", { hora })
      .then(() => {
        setDesligarEm(hora);
        gravar(K_DESLIGAR, hora);
        toast(`O notebook vai desligar todo dia às ${hora}`, "ok");
      })
      .catch((e) => toast(e.message, "erro"));

  const cancelarDesligar = () =>
    chamar("energia_desligar_em", { hora: null })
      .then(() => {
        setDesligarEm(null);
        gravar(K_DESLIGAR, null);
        toast("Desligamento automático cancelado", "ok");
      })
      .catch((e) => toast(e.message, "erro"));

  const restaurar = async () => {
    try {
      await chamar("energia_tampa", { manter: false });
      await chamar("energia_desligar_em", { hora: null });
      setTampa(false);
      setDesligarEm(null);
      gravar(K_TAMPA, "0");
      gravar(K_DESLIGAR, null);
      toast("Energia restaurada ao padrão do Windows", "ok");
    } catch (e: any) {
      toast(e.message, "erro");
    }
  };

  return (
    <div className="cartao">
      <h2>
        <LuPower /> Energia do notebook
      </h2>
      <div className="lista">
        <Checa v={tampa} onChange={(b) => aplicarTampa(b)}>
          Manter a rádio funcionando com a tampa fechada
        </Checa>
        <Campo rotulo="Desligar sozinho todo dia às" ajuda="O Windows avisa e desliga em 60 segundos">
          <div className="flex">
            <input type="time" value={hora} onChange={(e) => setHora(e.target.value)} aria-label="Horário para desligar" />
            <button className="btn" onClick={ativarDesligar}>
              Ativar
            </button>
            <button className="btn" onClick={cancelarDesligar} disabled={!desligarEm}>
              Cancelar
            </button>
          </div>
        </Campo>
        <div className="pequeno muted">{desligarEm ? `Desligamento programado para ${desligarEm}.` : "Desligamento automático desligado."}</div>
        <div className="pequeno">
          Ligar sozinho: o Windows não consegue ligar o notebook depois de desligado. Esse recurso fica no BIOS (UEFI) do Lenovo, em algo como
          "Scheduled Power On" ou "Power On Schedule". Sem ele, alguém precisa ligar o notebook antes do primeiro horário.
        </div>
      </div>
      <div className="flex" style={{ marginTop: 12 }}>
        <button className="btn" onClick={() => chamar("energia_abrir_configuracoes").catch((e) => toast(e.message, "erro"))}>
          <LuSettings /> Configurar energia do Windows
        </button>
        <button className="btn" onClick={restaurar}>
          Restaurar padrão
        </button>
      </div>
    </div>
  );
}
