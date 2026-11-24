import React, { useEffect, useState } from "react";
import { LuDownload, LuRefreshCw, LuSearch } from "react-icons/lu";
import { chamar, hojeIso, salvarArquivo } from "../api";
import type { Registro } from "../tipos";
import { Cabecalho, useApp } from "../ui";

export default function Historico() {
  const { toast } = useApp();
  const [regs, setRegs] = useState<Registro[]>([]);
  const [texto, setTexto] = useState("");
  const [nivel, setNivel] = useState("");
  const [data, setData] = useState("");
  const carregar = () => chamar<Registro[]>("listar_historico", { texto, nivel, data, limite: 1000 }).then(setRegs).catch((e) => toast(e.message, "erro"));
  useEffect(() => {
    carregar();
  }, [texto, nivel, data]);

  return (
    <div>
      <Cabecalho titulo="Histórico (auditoria local)">
        <button className="btn" onClick={carregar}>
          <LuRefreshCw /> Atualizar
        </button>
        <button
          className="btn"
          onClick={async () => {
            const c = await salvarArquivo("Exportar histórico", `historico-radio-${hojeIso()}.csv`, "csv");
            if (c) chamar<number>("exportar_historico", { caminho: c }).then((n) => toast(`${n} registros exportados`, "ok")).catch((e) => toast(e.message, "erro"));
          }}
        >
          <LuDownload /> Exportar CSV
        </button>
      </Cabecalho>
      <div className="cartao">
        <div className="flex" style={{ marginBottom: 10 }}>
          <LuSearch />
          <input type="search" placeholder="Buscar na mensagem" value={texto} onChange={(e) => setTexto(e.target.value)} style={{ flex: 1, minWidth: "14rem" }} />
          <select value={nivel} onChange={(e) => setNivel(e.target.value)} aria-label="Nível">
            <option value="">Todos os níveis</option>
            <option value="Info">Informação</option>
            <option value="Aviso">Aviso</option>
            <option value="Erro">Erro / falha</option>
          </select>
          <input type="date" value={data} onChange={(e) => setData(e.target.value)} aria-label="Data" />
          {data && (
            <button className="btn" onClick={() => setData("")}>
              Todas as datas
            </button>
          )}
        </div>
        <div className="rolagem" style={{ maxHeight: "62vh" }}>
          <table className="tabela">
            <thead>
              <tr>
                <th>Quando</th>
                <th>Origem</th>
                <th>Nível</th>
                <th>Mensagem</th>
              </tr>
            </thead>
            <tbody>
              {regs.map((r) => (
                <tr key={r.id}>
                  <td style={{ whiteSpace: "nowrap", fontVariantNumeric: "tabular-nums" }}>{r.quando}</td>
                  <td>{{ agendador: "Agendador", manual: "Operador", audio: "Áudio", sistema: "Sistema", biblioteca: "Biblioteca", config: "Configuração" }[r.origem] ?? r.origem}</td>
                  <td>
                    <span className={"chip " + (r.nivel === "Erro" ? "erro" : r.nivel === "Aviso" ? "Recreio" : "ok")}>{r.nivel}</span>
                  </td>
                  <td>{r.mensagem}</td>
                </tr>
              ))}
            </tbody>
          </table>
          {regs.length === 0 && <div className="vazio">Nenhum registro.</div>}
        </div>
        <p className="pequeno muted">Registros dos últimos 180 dias: execuções, falhas, ações manuais, mudanças de configuração e quedas de áudio.</p>
      </div>
    </div>
  );
}
