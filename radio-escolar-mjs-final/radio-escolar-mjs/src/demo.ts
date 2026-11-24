// BACKEND DE DEMONSTRAÇÃO — DADOS FICTÍCIOS.
// Usado só quando a interface é aberta fora do aplicativo (navegador), para
// visualizar telas e fluxos. Não toca áudio, não acessa pastas e não agenda nada.

import type {
  Agenda, Ajustes, Evento, Faixa, ItemAgenda, Painel, Pasta, Playlist, Registro, Aviso, TipoEvento, Diagnostico, ItemFilaView, Modo,
} from "./tipos";

type Ouvinte = (p: any) => void;
const ouvintes: Record<string, Ouvinte[]> = {};
function emitir(ev: string, p: any) {
  (ouvintes[ev] || []).forEach((f) => f(p));
}
export async function ouvir<T>(ev: string, cb: (p: T) => void) {
  (ouvintes[ev] ||= []).push(cb as Ouvinte);
  return () => {
    ouvintes[ev] = (ouvintes[ev] || []).filter((f) => f !== cb);
  };
}
export function caminhoFicticio(tipo: string) {
  return tipo === "pasta" ? "C:\\Users\\Escola\\Música\\Rádio (DEMO)" : "C:\\Users\\Escola\\Documentos\\exemplo-demo.json";
}

function ev(id: string, nome: string, tipo: TipoEvento, inicio: string, fim: string | null = null): Evento {
  const janela = tipo === "Recepcao" || tipo === "Recreio";
  return {
    id, nome, tipo, ativo: true, dias: [0, 1, 2, 3, 4], perfis: ["normal", "prova", "sabado_letivo"], inicio, fim,
    playlistId: null, pasta: null, faixaId: null, selecao: "Aleatorio", volume: janela ? 60 : 55,
    fadeInMs: janela ? 3000 : 1500, fadeOutMs: janela ? 6000 : 3000, limiteSegundos: null, fimJanela: "FadeOut",
    extraMaxSegundos: 240, vinhetaId: null, recadoId: null,
    prioridade: tipo === "Recreio" ? 80 : tipo === "Recepcao" ? 60 : tipo === "Recado" ? 40 : 20,
    conflito: "Interromper", observacao: "",
    avisoTroca: tipo === "Sinal" || tipo === "Saida", avisoAtraso: false,
  };
}

export function agendaPadrao(): Agenda {
  const e = [
    ev("manha-recepcao", "Recepção da manhã", "Recepcao", "06:45", "07:05"),
    ev("manha-sinal-0750", "Troca 07:50", "Sinal", "07:50"),
    ev("manha-sinal-0840", "Troca 08:40", "Sinal", "08:40"),
    ev("manha-sinal-0930", "Troca 09:30", "Sinal", "09:30"),
    ev("manha-recreio", "Recreio da manhã", "Recreio", "09:30", "09:47"),
    ev("manha-sinal-1035", "Troca 10:35", "Sinal", "10:35"),
    ev("manha-sinal-1125", "Troca 11:25", "Sinal", "11:25"),
    ev("manha-saida-1215", "Saída 12:15", "Saida", "12:15"),
    ev("tarde-recepcao", "Recepção da tarde", "Recepcao", "12:45", "13:05"),
    ev("tarde-sinal-1350", "Troca 13:50", "Sinal", "13:50"),
    ev("tarde-sinal-1440", "Troca 14:40", "Sinal", "14:40"),
    ev("tarde-sinal-1530", "Troca 15:30", "Sinal", "15:30"),
    ev("tarde-recreio", "Recreio da tarde", "Recreio", "15:30", "15:47"),
    ev("tarde-sinal-1635", "Troca 16:35", "Sinal", "16:35"),
    ev("tarde-saida-1725", "Saída 17:25", "Saida", "17:25"),
  ];
  return {
    eventos: e,
    perfis: [
      { id: "normal", nome: "Dia normal" }, { id: "prova", nome: "Dia de prova" }, { id: "evento", nome: "Dia de evento" },
      { id: "reuniao", nome: "Reunião" }, { id: "feriado", nome: "Feriado" }, { id: "sabado_letivo", nome: "Sábado letivo" },
      { id: "reduzido", nome: "Dia reduzido" },
    ],
    perfilPorDiaSemana: ["normal", "normal", "normal", "normal", "normal", null, null],
    excecoes: [{ data: "2026-12-08", perfil: "feriado", descricao: "Feriado (exemplo DEMO)" }],
    toleranciaSegundos: 60, saltoSegundos: 90, retomarJanelaAposReinicio: true, esperaMaxSegundos: 300, desligados: [],
  };
}

const faixas: Faixa[] = [
  ...Array.from({ length: 10 }, (_, i) => ({
    id: `demo-m${i + 1}`, caminho: `C:\\Música\\Rádio (DEMO)\\DEMO - Música ${String(i + 1).padStart(2, "0")}.mp3`,
    nome: `DEMO - Música ${String(i + 1).padStart(2, "0")}`, titulo: `Música fictícia ${i + 1}`, artista: "Artista DEMO",
    duracaoMs: 150000 + i * 9000, formato: "mp3", categoria: "musica", pastaId: 1, status: i === 9 ? "ausente" : "ok",
    erro: i === 9 ? "Arquivo removido, renomeado ou movido" : null, volume: 100, tocadaEm: null, vezes: i % 3,
  })),
  { id: "demo-v1", caminho: "C:\\Vinhetas (DEMO)\\DEMO - Vinheta abertura.wav", nome: "DEMO - Vinheta abertura", titulo: null, artista: null, duracaoMs: 4000, formato: "wav", categoria: "vinheta", pastaId: 2, status: "ok", erro: null, volume: 80, tocadaEm: null, vezes: 0 },
  { id: "demo-r1", caminho: "C:\\Recados (DEMO)\\DEMO - Recado fila da merenda.mp3", nome: "DEMO - Recado fila da merenda", titulo: null, artista: null, duracaoMs: 18000, formato: "mp3", categoria: "recado", pastaId: 3, status: "ok", erro: null, volume: 100, tocadaEm: null, vezes: 0 },
  { id: "demo-r2", caminho: "C:\\Recados (DEMO)\\DEMO - Recado reunião de pais.ogg", nome: "DEMO - Recado reunião de pais", titulo: null, artista: null, duracaoMs: 25000, formato: "ogg", categoria: "recado", pastaId: 3, status: "ok", erro: null, volume: 100, tocadaEm: null, vezes: 0 },
];

let pastas: Pasta[] = [
  { id: 1, caminho: "C:\\Música\\Rádio (DEMO)", categoria: "musica", subpastas: true, ultimaIndexacao: "2026-11-17 07:00:00", acessivel: true },
  { id: 2, caminho: "C:\\Vinhetas (DEMO)", categoria: "vinheta", subpastas: true, ultimaIndexacao: "2026-11-17 07:00:00", acessivel: true },
  { id: 3, caminho: "C:\\Recados (DEMO)", categoria: "recado", subpastas: true, ultimaIndexacao: "2026-11-17 07:00:00", acessivel: true },
];
let playlists: Playlist[] = [{ id: "pl-recreio", nome: "Recreio calmo (DEMO)", faixas: ["demo-m1", "demo-m2", "demo-m3", "demo-m4"] }];
let agenda = agendaPadrao();
let ajustes: Ajustes = {
  volumeGeral: 70, volumeMaximo: 85, limiteVinheta: 60,
  volumesTipo: { recepcao: 60, sinal: 55, recreio: 60, recado: 70, vinheta: 50, livre: 60 },
  fadeInPadraoMs: 2000, fadeOutPadraoMs: 4000, evitarRepeticaoUltimas: 15, dispositivoSaida: null,
  beepAntesRecado: true, beepFaixaId: null, duckModo: "Pausar", duckNivel: 20, alertaBateria: 25,
  indexarSubpastas: true, reindexarAutoMin: 30, iniciarComWindows: false, iniciarMinimizado: false,
  minimizarParaBandeja: true, escala: 1, altoContraste: false, logoArquivo: null,
  nomeEscola: "Rádio Escolar Mestre João Silvério", assistenteConcluido: true, micGanho: 50, micPortao: 8,
  micDispositivo: null, pinValidadeMin: 10, temPin: false,
};
let modo: Modo = "Escolar";
let silenciado = false;
let tocando = true;
let pausado = false;
let posicao = 42000;
let atual = faixas[2];
let fila: ItemFilaView[] = faixas.slice(0, 5).map((f, i) => ({ uid: i + 1, faixaId: f.id, faixa: f }));
let indiceFila: number | null = null;
let origem: any = { tipo: "evento", eventoId: "manha-recreio", nome: "Recreio da manhã (DEMO)", tipoEvento: "Recreio", fim: "09:47", faixasTocadas: 2 };
const historico: Registro[] = [
  { id: 3, quando: "2026-11-17 09:30:00", origem: "agendador", nivel: "Info", eventoId: "manha-recreio", mensagem: "Iniciado: Recreio da manhã — \"Música fictícia 2\" (DEMO)" },
  { id: 2, quando: "2026-11-17 09:30:00", origem: "agendador", nivel: "Info", eventoId: "manha-sinal-0930", mensagem: "Troca 09:30 suprimido: coincide com Recreio da manhã (prioridade maior). (DEMO)" },
  { id: 1, quando: "2026-11-17 06:40:12", origem: "sistema", nivel: "Info", eventoId: null, mensagem: "Aplicativo iniciado (DEMO). Eventos anteriores a agora não serão tocados." },
];
let nivel = 0;

function agoraSeg() {
  const d = new Date();
  return d.getHours() * 3600 + d.getMinutes() * 60 + d.getSeconds();
}
function hm(s: string) {
  const [h, m] = s.split(":").map(Number);
  return h * 3600 + m * 60;
}
function isoDe(d: Date) {
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
}
function perfilDe(iso: string): any {
  const ex = agenda.excecoes.find((x) => x.data === iso);
  if (ex) return ex.perfil ? { tipo: "Perfil", valor: ex.perfil } : { tipo: "Suspenso", valor: ex.descricao };
  const [a, m, d] = iso.split("-").map(Number);
  const ds = (new Date(a, m - 1, d).getDay() + 6) % 7;
  const p = agenda.perfilPorDiaSemana[ds];
  return p ? { tipo: "Perfil", valor: p } : { tipo: "SemAula" };
}
function itensDia(iso: string, base: Agenda = agenda): ItemAgenda[] {
  const p = perfilDe(iso);
  if (p.tipo !== "Perfil") return [];
  const [a, m, d] = iso.split("-").map(Number);
  const dt = new Date(a, m - 1, d);
  const ds = (dt.getDay() + 6) % 7;
  const evs = base.eventos.filter((e) => e.ativo && e.perfis.includes(p.valor) && e.dias.includes(ds)).sort((x, y) => hm(x.inicio) - hm(y.inicio) || y.prioridade - x.prioridade);
  const hoje = new Date();
  const offsetDias = Math.round((dt.getTime() - new Date(hoje.getFullYear(), hoje.getMonth(), hoje.getDate()).getTime()) / 86400000);
  return evs.map((e) => {
    const venc = evs.find((o) => o.id !== e.id && Math.floor(hm(o.inicio) / 60) === Math.floor(hm(e.inicio) / 60) && o.prioridade > e.prioridade);
    return {
      eventoId: e.id, nome: e.nome, tipo: e.tipo, data: iso, inicio: e.inicio, fim: e.fim,
      emSegundos: offsetDias * 86400 + hm(e.inicio) - agoraSeg(), suprimidoPor: venc ? venc.nome : null,
    };
  });
}
function proximos(n: number): ItemAgenda[] {
  const out: ItemAgenda[] = [];
  for (let k = 0; k < 14 && out.length < n; k++) {
    const d = new Date();
    d.setDate(d.getDate() + k);
    for (const it of itensDia(isoDe(d))) if (it.emSegundos > 0 && out.length < n) out.push(it);
  }
  return out;
}
function validarDemo(a: Agenda): Aviso[] {
  const av: Aviso[] = [];
  for (const e of a.eventos) {
    if (!/^\d{2}:\d{2}$/.test(e.inicio)) av.push({ gravidade: "Erro", eventoId: e.id, mensagem: `${e.nome}: horário inicial inválido`, sugestao: "Use HH:MM" });
    if ((e.tipo === "Recreio" || e.tipo === "Recepcao") && !e.fim) av.push({ gravidade: "Erro", eventoId: e.id, mensagem: `${e.nome}: precisa de horário final`, sugestao: null });
  }
  for (const e of a.eventos) for (const o of a.eventos)
    if (e.id < o.id && e.inicio === o.inicio && e.dias.some((d) => o.dias.includes(d))) {
      const v = e.prioridade >= o.prioridade ? e : o;
      av.push({ gravidade: "Info", eventoId: null, mensagem: `${e.nome} e ${o.nome} começam juntos às ${e.inicio}: tocará só "${v.nome}"`, sugestao: "Comportamento padrão: a playlist do recreio substitui a sinalização." });
    }
  return av;
}
function log(nivelL: string, msg: string) {
  historico.unshift({ id: historico.length + 1, quando: new Date().toLocaleString("sv-SE").replace("T", " "), origem: "manual", nivel: nivelL, eventoId: null, mensagem: msg + " (DEMO)" });
}

function painel(): Painel {
  const d = new Date();
  if (tocando && !pausado && !silenciado) {
    posicao += 250;
    nivel = 0.35 + Math.random() * 0.5;
    if (atual.duracaoMs && posicao > atual.duracaoMs) {
      posicao = 0;
      atual = faixas[(faixas.indexOf(atual) + 1) % 8];
    }
  } else nivel *= 0.8;
  return {
    agora: d.toLocaleTimeString("pt-BR"), dataIso: isoDe(d), modo, silenciado,
    audio: {
      tocando: tocando && !pausado && !silenciado, pausado, id: 1,
      item: tocando && !silenciado ? { caminho: atual.caminho, faixaId: atual.id, titulo: atual.titulo || atual.nome, artista: atual.artista, duracaoMs: atual.duracaoMs, ganho: 0.6 } : null,
      indiceItem: 0, totalItens: 1, posicaoMs: posicao, duracaoMs: atual.duracaoMs, nivel, volumeGeral: ajustes.volumeGeral / 100,
      emAnuncio: false, dispositivo: "Caixa de Som Bluetooth (DEMO)", dispositivoPadraoWindows: true, dispositivoOk: true,
      alertaDispositivo: null, micAberto: false, micFalando: false, micNivel: 0, micDispositivo: null, erroMic: null,
    },
    origem: silenciado ? { tipo: "nenhuma" } : origem, proximos: proximos(4), perfilHoje: perfilDe(isoDe(d)),
    alertas: [], avisosConfig: ["MODO DEMONSTRAÇÃO: dados fictícios, sem áudio real. Instale o aplicativo para usar de verdade."],
    ultimoTick: d.toLocaleString("sv-SE"), ultimaAcao: historico[0]?.mensagem ?? null,
    sistema: { bateria: { disponivel: true, percentual: 78, carregando: false, semBateria: false }, wifi: { disponivel: true, conectado: true, ssid: "Escola (DEMO)", sinal: 82 }, atualizadoEm: d.toLocaleTimeString("pt-BR") },
    indiceFila, repetir: "Nao", aleatorio: false, tamanhoFila: fila.length, volumeGeral: ajustes.volumeGeral, volumeMaximo: ajustes.volumeMaximo, indexando: false,
  };
}
setInterval(() => emitir("painel", painel()), 250);

const pausa = (ms = 120) => new Promise((r) => setTimeout(r, ms));

export async function chamar<T>(cmd: string, a: any): Promise<T> {
  await pausa(60);
  const r = (x: any) => x as T;
  switch (cmd) {
    case "painel_agora": return r(painel());
    case "obter_config": return r({ agenda: structuredClone(agenda), ajustes: structuredClone(ajustes), versao: "1.0.0 (DEMO)", dadosDir: "C:\\Users\\Escola\\AppData\\Roaming\\br.mg.escola.mestrejoaosilverio.radio (DEMO)", portatil: false });
    case "definir_modo":
      if (!a.confirmar && tocando && !silenciado) throw "CONFIRMAR";
      modo = a.modo; tocando = false; origem = { tipo: "nenhuma" };
      log("Info", modo === "Livre" ? "Modo Rádio Livre ativado: programação automática suspensa" : "Modo Rádio Escolar ativado");
      return r(null);
    case "silenciar_agora": silenciado = true; tocando = false; log("Aviso", "SILENCIAR AGORA acionado"); return r(null);
    case "retomar": silenciado = false; log("Info", "Rádio retomada pelo operador"); return r(null);
    case "tocar_pausar":
      if (silenciado) throw "A rádio está silenciada. Toque em \"Retomar\" primeiro.";
      if (tocando) { pausado = !pausado; return r(null); }
      if (modo === "Livre" && fila.length) { tocando = true; pausado = false; indiceFila = indiceFila ?? 0; atual = fila[indiceFila].faixa!; posicao = 0; origem = { tipo: "livre", uid: fila[indiceFila].uid }; return r(null); }
      throw "Nada programado tocando agora. Para tocar manualmente, use o Rádio Livre.";
    case "proxima":
      if (modo === "Livre" && fila.length) { indiceFila = ((indiceFila ?? -1) + 1) % fila.length; atual = fila[indiceFila].faixa!; posicao = 0; tocando = true; origem = { tipo: "livre", uid: fila[indiceFila].uid }; }
      else { atual = faixas[(faixas.indexOf(atual) + 1) % 8]; posicao = 0; }
      return r(null);
    case "anterior": posicao = 0; return r(null);
    case "definir_volume": ajustes.volumeGeral = Math.min(a.volume, ajustes.volumeMaximo); return r(ajustes.volumeGeral);
    case "buscar": posicao = a.ms; return r(null);
    case "fila_obter": return r(structuredClone(fila));
    case "fila_adicionar": {
      let uid = Math.max(0, ...fila.map((f) => f.uid));
      for (const id of a.ids) fila.push({ uid: ++uid, faixaId: id, faixa: faixas.find((f) => f.id === id) || null });
      emitir("fila", fila.length); return r(null);
    }
    case "fila_adicionar_playlist": {
      const p = playlists.find((x) => x.id === a.id);
      if (a.substituir) fila = [];
      let uid = Math.max(0, ...fila.map((f) => f.uid));
      for (const id of p?.faixas || []) fila.push({ uid: ++uid, faixaId: id, faixa: faixas.find((f) => f.id === id) || null });
      emitir("fila", fila.length); return r(p?.faixas.length || 0);
    }
    case "fila_reordenar": fila = a.uids.map((u: number) => fila.find((f) => f.uid === u)).filter(Boolean); emitir("fila", fila.length); return r(null);
    case "fila_remover": fila = fila.filter((f) => f.uid !== a.uid); emitir("fila", fila.length); return r(null);
    case "fila_limpar": fila = []; indiceFila = null; emitir("fila", 0); return r(null);
    case "fila_tocar": {
      if (modo !== "Livre") throw "Ative o Rádio Livre para tocar itens da fila.";
      const i = fila.findIndex((f) => f.uid === a.uid); indiceFila = i; atual = fila[i].faixa!; posicao = 0; tocando = true; pausado = false; origem = { tipo: "livre", uid: a.uid };
      return r(null);
    }
    case "definir_repetir": case "definir_aleatorio": return r(null);
    case "listar_pastas": return r(structuredClone(pastas));
    case "adicionar_pasta": pastas.push({ id: pastas.length + 1, caminho: a.caminho, categoria: a.categoria, subpastas: a.subpastas, ultimaIndexacao: null, acessivel: true }); setTimeout(() => emitir("indexacao", { pastas: pastas.length, pastasInacessiveis: [], novasOuAlteradas: 0, inalteradas: faixas.length, ausentes: 1, invalidas: 0 }), 800); return r(pastas.length);
    case "remover_pasta": pastas = pastas.filter((p) => p.id !== a.id); return r(null);
    case "reindexar": setTimeout(() => emitir("indexacao", { pastas: pastas.length, pastasInacessiveis: [], novasOuAlteradas: 0, inalteradas: faixas.length, ausentes: 1, invalidas: 0 }), 800); return r(null);
    case "listar_faixas": {
      const t = (a.texto || "").toLowerCase();
      return r(faixas.filter((f) => (!a.categoria || f.categoria === a.categoria) && (!a.status || f.status === a.status) && (!t || (f.nome + (f.titulo || "") + (f.artista || "")).toLowerCase().includes(t))));
    }
    case "definir_categoria_faixa": { const f = faixas.find((x) => x.id === a.id); if (f) f.categoria = a.categoria; return r(null); }
    case "definir_volume_faixa": { const f = faixas.find((x) => x.id === a.id); if (f) f.volume = a.volume; return r(null); }
    case "resumo_biblioteca": return r([["musica", "ok", 9], ["musica", "ausente", 1], ["vinheta", "ok", 1], ["recado", "ok", 2]]);
    case "listar_playlists": return r(structuredClone(playlists));
    case "salvar_playlist": { const p = { ...a.playlist, id: a.playlist.id || `pl-${Date.now()}` }; playlists = [...playlists.filter((x) => x.id !== p.id), p]; return r(p); }
    case "excluir_playlist": playlists = playlists.filter((x) => x.id !== a.id); return r(null);
    case "tocar_recado": if (silenciado) throw "A rádio está silenciada. Toque em \"Retomar\" antes de enviar um recado."; log("Info", "Recado imediato enviado"); return r(null);
    case "validar_agenda": return r(validarDemo(a.agenda));
    case "salvar_agenda": { const v = validarDemo(a.agenda); if (v.some((x) => x.gravidade === "Erro")) throw "A programação tem erros. Corrija os itens em vermelho antes de salvar."; agenda = structuredClone(a.agenda); log("Info", "Programação salva"); return r(v); }
    case "agenda_dia": return r({ perfil: perfilDe(a.data), itens: itensDia(a.data, a.agenda || agenda) });
    case "agenda_inicial": return r(agendaPadrao());
    case "salvar_ajustes": ajustes = { ...structuredClone(a.ajustes), temPin: ajustes.temPin }; return r(structuredClone(ajustes));
    case "definir_pin": ajustes.temPin = !!a.novo; return r(null);
    case "desbloquear": return r(true);
    case "bloquear": return r(null);
    case "esta_desbloqueado": return r(true);
    case "definir_autostart": ajustes.iniciarComWindows = a.ativo; return r(a.ativo);
    case "concluir_assistente": ajustes.assistenteConcluido = true; return r(null);
    case "obter_logo": case "definir_logo": case "capa": return r(null);
    case "dispositivos": return r({ saidas: ["Caixa de Som Bluetooth (DEMO)", "Alto-falantes internos (DEMO)"], padrao: "Caixa de Som Bluetooth (DEMO)", entradas: ["Microfone interno (DEMO)"], entradaPadrao: "Microfone interno (DEMO)", host: "Wasapi (DEMO)" });
    case "selecionar_dispositivo": case "confirmar_dispositivo": case "tocar_teste": case "limpar_alertas": return r(null);
    case "diagnostico": { const p = painel(); const d: Diagnostico = { dispositivos: await chamar("dispositivos", {}), audio: p.audio, dicaHandsFree: false, pareceBluetooth: true, versao: "1.0.0 (DEMO)", dadosDir: "(DEMO)", bancoBytes: 245760, ultimoTick: p.ultimoTick, biblioteca: await chamar("resumo_biblioteca", {}), sistema: p.sistema, portatil: false }; return r(d); }
    case "mic_abrir": case "mic_fechar": case "mic_falar": case "mic_ajuste": return r(null);
    case "listar_historico": return r(historico.filter((h) => (!a.nivel || h.nivel === a.nivel) && (!a.texto || h.mensagem.toLowerCase().includes(String(a.texto).toLowerCase())) && (!a.data || h.quando.startsWith(a.data))));
    case "exportar_historico": return r(historico.length);
    case "exportar_config": case "importar_config": return r([]);
    case "criar_demo": return r("(DEMO)");
  }
  throw `Comando não disponível na demonstração: ${cmd}`;
}
