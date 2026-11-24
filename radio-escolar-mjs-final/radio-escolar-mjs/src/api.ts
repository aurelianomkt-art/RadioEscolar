// Ponte com o processo nativo (Tauri). Fora do Tauri (navegador), usa o backend de
// DEMONSTRAÇÃO com dados fictícios — apenas para visualizar a interface.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";
import * as demo from "./demo";

export const noTauri: boolean = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
export const modoDemo = !noTauri;

let pedirPin: (() => Promise<boolean>) | null = null;
export function definirPedidoDePin(f: () => Promise<boolean>) {
  pedirPin = f;
}

function mensagem(e: unknown): string {
  if (typeof e === "string") return e;
  if (e && typeof e === "object" && "message" in e) return String((e as any).message);
  return String(e);
}

export async function chamar<T = any>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  const exec = () => (noTauri ? invoke<T>(cmd, args) : demo.chamar<T>(cmd, args));
  try {
    return await exec();
  } catch (e) {
    const m = mensagem(e);
    if (m === "PIN_NECESSARIO" && pedirPin) {
      const ok = await pedirPin();
      if (ok) return await exec();
      throw new Error("Operação cancelada: PIN não informado.");
    }
    throw new Error(m);
  }
}

export async function ouvir<T>(evento: string, cb: (p: T) => void): Promise<() => void> {
  if (noTauri) return listen<T>(evento, (e) => cb(e.payload));
  return demo.ouvir<T>(evento, cb);
}

export async function escolherPasta(titulo = "Escolha a pasta"): Promise<string | null> {
  if (!noTauri) return demo.caminhoFicticio("pasta");
  const r = await open({ directory: true, multiple: false, title: titulo });
  return typeof r === "string" ? r : null;
}

export async function escolherArquivo(titulo: string, extensoes: string[]): Promise<string | null> {
  if (!noTauri) return demo.caminhoFicticio("arquivo");
  const r = await open({ directory: false, multiple: false, title: titulo, filters: [{ name: "Arquivos", extensions: extensoes }] });
  return typeof r === "string" ? r : null;
}

export async function salvarArquivo(titulo: string, nomePadrao: string, extensao: string): Promise<string | null> {
  if (!noTauri) return demo.caminhoFicticio("salvar");
  const r = await save({ title: titulo, defaultPath: nomePadrao, filters: [{ name: extensao.toUpperCase(), extensions: [extensao] }] });
  return r ?? null;
}

export function fmtTempo(ms: number | null | undefined): string {
  if (ms == null || !isFinite(ms) || ms < 0) return "--:--";
  const s = Math.floor(ms / 1000);
  const m = Math.floor(s / 60);
  const h = Math.floor(m / 60);
  const ss = String(s % 60).padStart(2, "0");
  return h > 0 ? `${h}:${String(m % 60).padStart(2, "0")}:${ss}` : `${m}:${ss}`;
}

export function fmtContagem(seg: number): string {
  if (seg <= 0) return "agora";
  const d = Math.floor(seg / 86400);
  const h = Math.floor((seg % 86400) / 3600);
  const m = Math.floor((seg % 3600) / 60);
  const s = seg % 60;
  if (d > 0) return `${d}d ${h}h`;
  if (h > 0) return `${h}h ${String(m).padStart(2, "0")}min`;
  return `${m}:${String(s).padStart(2, "0")}`;
}

export function dataExtenso(iso: string): string {
  const [a, m, d] = iso.split("-").map(Number);
  const dt = new Date(a, m - 1, d);
  const t = dt.toLocaleDateString("pt-BR", { weekday: "long", day: "2-digit", month: "long", year: "numeric" });
  return t.charAt(0).toUpperCase() + t.slice(1);
}

export function hojeIso(): string {
  const d = new Date();
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
}
