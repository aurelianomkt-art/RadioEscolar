// Tipos espelhados do backend Rust (serde camelCase).

export type TipoEvento = "Recepcao" | "Sinal" | "Saida" | "Recreio" | "Recado";
export type ModoSelecao = "Aleatorio" | "Sequencial" | "FaixaFixa";
export type FimJanela = "FadeOut" | "TerminarFaixa";
export type PoliticaConflito = "Interromper" | "Aguardar" | "Ignorar";
export type Modo = "Escolar" | "Livre";
export type Repetir = "Nao" | "Fila" | "Faixa";

export interface Evento {
  id: string;
  nome: string;
  tipo: TipoEvento;
  ativo: boolean;
  dias: number[];
  perfis: string[];
  inicio: string;
  fim: string | null;
  playlistId: string | null;
  pasta: string | null;
  faixaId: string | null;
  selecao: ModoSelecao;
  volume: number;
  fadeInMs: number;
  fadeOutMs: number;
  limiteSegundos: number | null;
  fimJanela: FimJanela;
  extraMaxSegundos: number;
  vinhetaId: string | null;
  recadoId: string | null;
  prioridade: number;
  conflito: PoliticaConflito;
  observacao: string;
  /** Aviso de troca (ding-dong) antes da música. */
  avisoTroca: boolean;
  /** Aviso de atraso (campainha diferente) antes do evento. */
  avisoAtraso: boolean;
}

export interface Perfil { id: string; nome: string }
export interface Excecao { data: string; perfil: string | null; descricao: string }
/** Interruptor por toque: evento desligado só na data indicada (AAAA-MM-DD). */
export interface Desligado { data: string; eventoId: string }

export interface Agenda {
  eventos: Evento[];
  perfis: Perfil[];
  perfilPorDiaSemana: (string | null)[];
  excecoes: Excecao[];
  toleranciaSegundos: number;
  saltoSegundos: number;
  retomarJanelaAposReinicio: boolean;
  esperaMaxSegundos: number;
  desligados: Desligado[];
}

export type PerfilDia = { tipo: "Perfil"; valor: string } | { tipo: "Suspenso"; valor: string } | { tipo: "SemAula" };

export interface ItemAgenda {
  eventoId: string;
  nome: string;
  tipo: TipoEvento;
  data: string;
  inicio: string;
  fim: string | null;
  emSegundos: number;
  suprimidoPor: string | null;
}

export interface Aviso {
  gravidade: "Erro" | "Alerta" | "Info";
  eventoId: string | null;
  mensagem: string;
  sugestao: string | null;
}

export interface VolumesTipo { recepcao: number; sinal: number; recreio: number; recado: number; vinheta: number; livre: number }

export interface Ajustes {
  volumeGeral: number;
  volumeMaximo: number;
  limiteVinheta: number;
  volumesTipo: VolumesTipo;
  fadeInPadraoMs: number;
  fadeOutPadraoMs: number;
  evitarRepeticaoUltimas: number;
  dispositivoSaida: string | null;
  beepAntesRecado: boolean;
  beepFaixaId: string | null;
  duckModo: "Pausar" | "Abaixar";
  duckNivel: number;
  alertaBateria: number;
  indexarSubpastas: boolean;
  reindexarAutoMin: number;
  iniciarComWindows: boolean;
  iniciarMinimizado: boolean;
  minimizarParaBandeja: boolean;
  escala: number;
  altoContraste: boolean;
  logoArquivo: string | null;
  nomeEscola: string;
  assistenteConcluido: boolean;
  micGanho: number;
  micPortao: number;
  micDispositivo: string | null;
  pinValidadeMin: number;
  temPin?: boolean;
}

export interface ConfigCompleta { agenda: Agenda; ajustes: Ajustes; versao: string; dadosDir: string; portatil: boolean }

export interface ItemAudio { caminho: string | null; faixaId: string | null; titulo: string; artista: string | null; duracaoMs: number | null; ganho: number }

export interface StatusAudio {
  tocando: boolean;
  pausado: boolean;
  id: number;
  item: ItemAudio | null;
  indiceItem: number;
  totalItens: number;
  posicaoMs: number;
  duracaoMs: number | null;
  nivel: number;
  volumeGeral: number;
  emAnuncio: boolean;
  dispositivo: string;
  dispositivoPadraoWindows: boolean;
  dispositivoOk: boolean;
  alertaDispositivo: string | null;
  micAberto: boolean;
  micFalando: boolean;
  micNivel: number;
  micDispositivo: string | null;
  erroMic: string | null;
}

export type Origem =
  | { tipo: "nenhuma" }
  | { tipo: "evento"; eventoId: string; nome: string; tipoEvento: TipoEvento; fim: string | null; faixasTocadas: number }
  | { tipo: "livre"; uid: number };

export interface StatusSistema {
  bateria: { disponivel: boolean; percentual: number | null; carregando: boolean | null; semBateria: boolean };
  wifi: { disponivel: boolean; conectado: boolean | null; ssid: string | null; sinal: number | null };
  atualizadoEm: string;
}

export interface Painel {
  agora: string;
  dataIso: string;
  modo: Modo;
  silenciado: boolean;
  audio: StatusAudio;
  origem: Origem;
  proximos: ItemAgenda[];
  perfilHoje: PerfilDia;
  alertas: string[];
  avisosConfig: string[];
  ultimoTick: string | null;
  ultimaAcao: string | null;
  sistema: StatusSistema;
  indiceFila: number | null;
  repetir: Repetir;
  aleatorio: boolean;
  tamanhoFila: number;
  volumeGeral: number;
  volumeMaximo: number;
  indexando: boolean;
}

export interface Faixa {
  id: string;
  caminho: string;
  nome: string;
  titulo: string | null;
  artista: string | null;
  duracaoMs: number | null;
  formato: string;
  categoria: "musica" | "vinheta" | "recado" | string;
  pastaId: number;
  status: "ok" | "ausente" | "invalido" | string;
  erro: string | null;
  volume: number;
  tocadaEm: string | null;
  vezes: number;
}

export interface Pasta { id: number; caminho: string; categoria: string; subpastas: boolean; ultimaIndexacao: string | null; acessivel: boolean }
export interface Playlist { id: string; nome: string; faixas: string[] }
export interface ItemFilaView { uid: number; faixaId: string; faixa: Faixa | null }
export interface Registro { id: number; quando: string; origem: string; nivel: string; eventoId: string | null; mensagem: string }

export interface InfoDispositivos { saidas: string[]; padrao: string | null; entradas: string[]; entradaPadrao: string | null; host: string }

export interface Diagnostico {
  dispositivos: InfoDispositivos;
  audio: StatusAudio;
  dicaHandsFree: boolean;
  pareceBluetooth: boolean;
  versao: string;
  dadosDir: string;
  bancoBytes: number;
  ultimoTick: string | null;
  biblioteca: [string, string, number][];
  sistema: StatusSistema;
  portatil: boolean;
}

export type Tela =
  | "inicio"
  | "escolar"
  | "livre"
  | "programacao"
  | "biblioteca"
  | "recados"
  | "historico"
  | "config"
  | "diagnostico"
  | "assistente";

export const NOMES_TIPO: Record<TipoEvento, string> = {
  Recepcao: "Recepção",
  Sinal: "Sinalização (troca)",
  Saida: "Saída do dia",
  Recreio: "Recreio",
  Recado: "Recado agendado",
};

export const DIAS = ["Seg", "Ter", "Qua", "Qui", "Sex", "Sáb", "Dom"];
export const CATEGORIAS: Record<string, string> = { musica: "Músicas", vinheta: "Vinhetas", recado: "Recados" };
