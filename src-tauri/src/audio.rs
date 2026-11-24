//! Motor de áudio local (thread própria).
//!
//! * rodio/cpal sobre WASAPI: respeita o dispositivo padrão do Windows ou um dispositivo escolhido.
//! * Um único canal "principal" (músicas) + um canal de "anúncio" (recados/microfone).
//!   A música é pausada (ou abaixada) ANTES do anúncio: nunca há duas músicas misturadas.
//! * Fades por rampa de volume (sem picos). Volume máximo limitado pelos ajustes.
//! * Perda do dispositivo: pausa e alerta. NUNCA troca sozinho para os alto-falantes internos.

use rodio::cpal;
use rodio::cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use serde::Serialize;
use std::collections::VecDeque;
use std::fs::File;
use std::io::BufReader;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Um item a tocar (arquivo ou beep interno).
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemAudio {
    /// None = beep suave interno.
    pub caminho: Option<String>,
    pub faixa_id: Option<String>,
    pub titulo: String,
    pub artista: Option<String>,
    pub duracao_ms: Option<u64>,
    /// Ganho 0.0–1.0 deste item (volume do evento × volume individual × tetos).
    pub ganho: f32,
}

/// Marca interna para o som de aviso de atraso (campainha diferente do ding-dong).
const MARCA_ATRASO: &str = "@aviso-atraso";

impl ItemAudio {
    pub fn beep(ganho: f32) -> ItemAudio {
        ItemAudio { caminho: None, faixa_id: None, titulo: "Aviso de troca".into(), artista: None, duracao_ms: Some(2300), ganho }
    }
    pub fn beep_atraso(ganho: f32) -> ItemAudio {
        ItemAudio { caminho: None, faixa_id: Some(MARCA_ATRASO.into()), titulo: "Aviso de atraso".into(), artista: None, duracao_ms: Some(780), ganho }
    }
}

#[derive(Debug)]
pub enum Cmd {
    /// Toca uma sequência no canal principal (ex.: vinheta → música). Substitui o que estiver tocando
    /// (com fade-out de `fade_saida_ms` antes, se houver algo).
    Tocar { itens: Vec<ItemAudio>, fade_in_ms: u32, fade_saida_ms: u32, limite_ms: Option<u64>, fade_fim_ms: u32, id: u64, posicao_ms: u64 },
    /// Recado/vinheta por cima: pausa ou abaixa a música, toca os itens, depois retoma.
    Anunciar { itens: Vec<ItemAudio>, pausar_musica: bool, nivel_duck: f32 },
    Pausar { fade_ms: u32 },
    Continuar { fade_ms: u32 },
    Parar { fade_ms: u32 },
    /// Corte imediato de tudo (microfone inclusive).
    Silenciar,
    Volume { geral: f32 },
    Buscar { ms: u64 },
    SelecionarDispositivo { nome: Option<String> },
    /// Confirmação do operador após perda do dispositivo.
    Reabrir { nome: Option<String> },
    Teste { ganho: f32 },
    MicAbrir { nome: Option<String> },
    MicFechar,
    MicFalar { falar: bool, pausar_musica: bool },
    MicAjuste { ganho: f32, portao: f32 },
}

#[derive(Debug, Clone)]
pub enum EventoAudio {
    /// A sequência `id` terminou naturalmente (ou pelo limite de duração).
    Terminou { id: u64 },
    Erro { id: u64, mensagem: String, faixa_id: Option<String> },
    DispositivoPerdido { nome: String, detalhe: String },
    DispositivoReaberto { nome: String },
    AnuncioTerminou,
    Microfonia,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusAudio {
    pub tocando: bool,
    pub pausado: bool,
    pub id: u64,
    pub item: Option<ItemAudio>,
    pub indice_item: usize,
    pub total_itens: usize,
    pub posicao_ms: u64,
    pub duracao_ms: Option<u64>,
    pub nivel: f32,
    pub volume_geral: f32,
    pub em_anuncio: bool,
    pub dispositivo: String,
    pub dispositivo_padrao_windows: bool,
    pub dispositivo_ok: bool,
    pub alerta_dispositivo: Option<String>,
    pub mic_aberto: bool,
    pub mic_falando: bool,
    pub mic_nivel: f32,
    pub mic_dispositivo: Option<String>,
    pub erro_mic: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InfoDispositivos {
    pub saidas: Vec<String>,
    pub padrao: Option<String>,
    pub entradas: Vec<String>,
    pub entrada_padrao: Option<String>,
    pub host: String,
}

pub fn listar_dispositivos() -> InfoDispositivos {
    let host = cpal::default_host();
    let saidas = host.output_devices().map(|it| it.filter_map(|d| d.name().ok()).collect()).unwrap_or_default();
    let entradas = host.input_devices().map(|it| it.filter_map(|d| d.name().ok()).collect()).unwrap_or_default();
    InfoDispositivos {
        saidas,
        padrao: host.default_output_device().and_then(|d| d.name().ok()),
        entradas,
        entrada_padrao: host.default_input_device().and_then(|d| d.name().ok()),
        host: format!("{:?}", host.id()),
    }
}

// ---------------------------------------------------------------------------
// Fontes auxiliares
// ---------------------------------------------------------------------------

/// Mede o pico do sinal para o visualizador de níveis.
struct Medidor<S> {
    inner: S,
    pico: Arc<AtomicU32>,
    acc: f32,
    n: u32,
}

impl<S: Source<Item = f32>> Iterator for Medidor<S> {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        let s = self.inner.next()?;
        let a = s.abs();
        if a > self.acc {
            self.acc = a;
        }
        self.n += 1;
        if self.n >= 2048 {
            self.pico.store(self.acc.to_bits(), Ordering::Relaxed);
            self.acc = 0.0;
            self.n = 0;
        }
        Some(s)
    }
}

impl<S: Source<Item = f32>> Source for Medidor<S> {
    fn current_frame_len(&self) -> Option<usize> {
        self.inner.current_frame_len()
    }
    fn channels(&self) -> u16 {
        self.inner.channels()
    }
    fn sample_rate(&self) -> u32 {
        self.inner.sample_rate()
    }
    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }
    fn try_seek(&mut self, pos: Duration) -> Result<(), rodio::source::SeekError> {
        self.inner.try_seek(pos)
    }
}

/// Áudio do microfone ao vivo (ganho, portão de ruído e limitador suave).
struct FonteMic {
    buf: Arc<Mutex<VecDeque<f32>>>,
    canais: u16,
    taxa: u32,
    ganho: Arc<AtomicU32>,
    portao: Arc<AtomicU32>,
    ativo: Arc<AtomicBool>,
    env: f32,
}

impl Iterator for FonteMic {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        if !self.ativo.load(Ordering::Relaxed) {
            return None;
        }
        let s = self.buf.lock().ok().and_then(|mut b| b.pop_front()).unwrap_or(0.0);
        let g = f32::from_bits(self.ganho.load(Ordering::Relaxed));
        let limiar = f32::from_bits(self.portao.load(Ordering::Relaxed));
        let a = s.abs();
        self.env = if a > self.env { a } else { self.env * 0.9995 };
        let x = if self.env < limiar { 0.0 } else { s * g };
        // limitador suave
        let y = if x.abs() < 1.0 { x - x * x * x / 3.0 } else { x.signum() * 2.0 / 3.0 };
        Some(y)
    }
}

impl Source for FonteMic {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> u16 {
        self.canais
    }
    fn sample_rate(&self) -> u32 {
        self.taxa
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

fn abrir_fonte(item: &ItemAudio, pico: &Arc<AtomicU32>) -> Result<Box<dyn Source<Item = f32> + Send>, String> {
    let g = item.ganho.clamp(0.0, 1.0);
    match &item.caminho {
        None if item.faixa_id.as_deref() == Some(MARCA_ATRASO) => {
            // Aviso de atraso: três notas em descida rápida (campainha de corredor).
            let notas: [f32; 3] = [880.0, 698.0, 523.0];
            let mut partes: Vec<Box<dyn Source<Item = f32> + Send>> = Vec::new();
            for freq in notas.iter() {
                let mut t = rodio::source::SineWave::new(*freq).take_duration(Duration::from_millis(260));
                t.set_filter_fadeout();
                partes.push(Box::new(t.fade_in(Duration::from_millis(15)).amplify(0.28 * g)));
            }
            let cadeia = rodio::source::from_iter(partes);
            Ok(Box::new(Medidor { inner: cadeia, pico: pico.clone(), acc: 0.0, n: 0 }))
        }
        None => {
            // Aviso de troca estilo aeroporto (ding-dong): tom agudo e depois tom mais grave,
            // ambos com entrada suave e cauda longa. Suave, sem ser um beep seco.
            let mut a = rodio::source::SineWave::new(1319.0).take_duration(Duration::from_millis(900));
            a.set_filter_fadeout();
            let mut b = rodio::source::SineWave::new(988.0).take_duration(Duration::from_millis(1400));
            b.set_filter_fadeout();
            let a = a.fade_in(Duration::from_millis(25)).amplify(0.30 * g);
            let b = b.fade_in(Duration::from_millis(25)).amplify(0.26 * g);
            let cadeia = rodio::source::from_iter(vec![
                Box::new(a) as Box<dyn Source<Item = f32> + Send>,
                Box::new(b) as Box<dyn Source<Item = f32> + Send>,
            ]);
            Ok(Box::new(Medidor { inner: cadeia, pico: pico.clone(), acc: 0.0, n: 0 }))
        }
        Some(c) => {
            let f = File::open(c).map_err(|e| format!("Arquivo inacessível: {} ({})", c, e))?;
            let d = Decoder::new(BufReader::new(f)).map_err(|e| format!("Arquivo inválido ou formato não suportado: {} ({})", c, e))?;
            let s = d.convert_samples::<f32>().amplify(g);
            Ok(Box::new(Medidor { inner: s, pico: pico.clone(), acc: 0.0, n: 0 }))
        }
    }
}

// ---------------------------------------------------------------------------
// Rampas de volume
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq)]
enum AoFim {
    Nada,
    Pausar,
    Parar,
}

#[derive(Clone, Copy, Debug)]
struct Rampa {
    de: f32,
    para: f32,
    inicio: Instant,
    dur: Duration,
    ao_fim: AoFim,
}

impl Rampa {
    fn valor(&self) -> (f32, bool) {
        if self.dur.is_zero() {
            return (self.para, true);
        }
        let t = self.inicio.elapsed().as_secs_f32() / self.dur.as_secs_f32();
        if t >= 1.0 {
            (self.para, true)
        } else {
            // curva suave (cosseno) para evitar degraus
            let k = 0.5 - 0.5 * (std::f32::consts::PI * t).cos();
            (self.de + (self.para - self.de) * k, false)
        }
    }
}

struct Canal {
    sink: Sink,
    env: f32,
    rampa: Option<Rampa>,
}

impl Canal {
    fn rampa(&mut self, para: f32, ms: u32, ao_fim: AoFim) {
        self.rampa = Some(Rampa { de: self.env, para, inicio: Instant::now(), dur: Duration::from_millis(ms as u64), ao_fim });
    }
    /// Atualiza volume; devolve a ação de fim se a rampa terminou.
    fn atualizar(&mut self, master: f32) -> AoFim {
        let mut fim = AoFim::Nada;
        if let Some(r) = self.rampa {
            let (v, acabou) = r.valor();
            self.env = v;
            if acabou {
                self.rampa = None;
                fim = r.ao_fim;
            }
        }
        self.sink.set_volume((self.env * master).clamp(0.0, 1.0));
        fim
    }
}

// ---------------------------------------------------------------------------
// Thread de áudio
// ---------------------------------------------------------------------------

pub struct Audio {
    pub tx: Sender<Cmd>,
    pub status: Arc<Mutex<StatusAudio>>,
}

pub fn iniciar(dispositivo: Option<String>, volume_geral: f32, eventos: Sender<EventoAudio>) -> Audio {
    let (tx, rx) = std::sync::mpsc::channel::<Cmd>();
    let status = Arc::new(Mutex::new(StatusAudio { volume_geral, ..Default::default() }));
    let st2 = status.clone();
    let sondagem = iniciar_sondagem();
    std::thread::Builder::new()
        .name("audio".into())
        .spawn(move || {
            let mut m = Motor::new(dispositivo, volume_geral, st2, eventos, sondagem);
            m.rodar(rx);
        })
        .expect("thread de áudio");
    Audio { tx, status }
}

/// Snapshot dos dispositivos de saída, atualizado por uma thread própria.
/// Assim a enumeração (que pode demorar no WASAPI) nunca roda na thread de áudio.
#[derive(Default)]
struct Sondagem {
    saidas: Vec<String>,
    padrao: Option<String>,
    ok: bool,
}

fn iniciar_sondagem() -> Arc<Mutex<Sondagem>> {
    let s = Arc::new(Mutex::new(Sondagem::default()));
    let s2 = s.clone();
    let _ = std::thread::Builder::new().name("sondagem-audio".into()).spawn(move || {
        loop {
            let host = cpal::default_host();
            let saidas: Vec<String> = host.output_devices().map(|it| it.filter_map(|d| d.name().ok()).collect()).unwrap_or_default();
            let padrao = host.default_output_device().and_then(|d| d.name().ok());
            if let Ok(mut g) = s2.lock() {
                *g = Sondagem { saidas, padrao, ok: true };
            }
            std::thread::sleep(Duration::from_secs(5));
        }
    });
    s
}

struct Pendente {
    itens: Vec<ItemAudio>,
    fade_in_ms: u32,
    limite_ms: Option<u64>,
    fade_fim_ms: u32,
    id: u64,
    posicao_ms: u64,
}

struct Motor {
    _stream: Option<OutputStream>,
    handle: Option<OutputStreamHandle>,
    nome_dispositivo: String,
    escolhido: Option<String>,
    perdido: bool,
    principal: Option<Canal>,
    anuncio: Option<Canal>,
    itens: Vec<ItemAudio>,
    id: u64,
    limite_ms: Option<u64>,
    fade_fim_ms: u32,
    encerrando_limite: bool,
    pausado: bool,
    pausado_para_anuncio: bool,
    duck_ativo: bool,
    proximo: Option<Pendente>,
    geral: f32,
    pico: Arc<AtomicU32>,
    status: Arc<Mutex<StatusAudio>>,
    ev: Sender<EventoAudio>,
    ultima_verificacao: Instant,
    sondagem: Arc<Mutex<Sondagem>>,
    // microfone
    mic_stream: Option<cpal::Stream>,
    mic_buf: Arc<Mutex<VecDeque<f32>>>,
    mic_pico: Arc<AtomicU32>,
    mic_ganho: Arc<AtomicU32>,
    mic_portao: Arc<AtomicU32>,
    mic_ativo: Arc<AtomicBool>,
    mic_formato: (u16, u32),
    mic_nome: Option<String>,
    mic_falando: bool,
    mic_saturado_desde: Option<Instant>,
    erro_mic: Option<String>,
}

impl Motor {
    fn new(dispositivo: Option<String>, geral: f32, status: Arc<Mutex<StatusAudio>>, ev: Sender<EventoAudio>, sondagem: Arc<Mutex<Sondagem>>) -> Motor {
        let mut m = Motor {
            _stream: None,
            handle: None,
            nome_dispositivo: String::new(),
            escolhido: dispositivo.clone(),
            perdido: false,
            principal: None,
            anuncio: None,
            itens: vec![],
            id: 0,
            limite_ms: None,
            fade_fim_ms: 3000,
            encerrando_limite: false,
            pausado: false,
            pausado_para_anuncio: false,
            duck_ativo: false,
            proximo: None,
            geral,
            pico: Arc::new(AtomicU32::new(0)),
            status,
            ev,
            ultima_verificacao: Instant::now(),
            sondagem,
            mic_stream: None,
            mic_buf: Arc::new(Mutex::new(VecDeque::new())),
            mic_pico: Arc::new(AtomicU32::new(0)),
            mic_ganho: Arc::new(AtomicU32::new(0.5f32.to_bits())),
            mic_portao: Arc::new(AtomicU32::new(0.02f32.to_bits())),
            mic_ativo: Arc::new(AtomicBool::new(false)),
            mic_formato: (1, 48_000),
            mic_nome: None,
            mic_falando: false,
            mic_saturado_desde: None,
            erro_mic: None,
        };
        if let Err(e) = m.abrir_saida(dispositivo) {
            m.perdido = true;
            let _ = m.ev.send(EventoAudio::DispositivoPerdido { nome: m.escolhido.clone().unwrap_or_else(|| "padrão do Windows".into()), detalhe: e });
        }
        m
    }

    fn abrir_saida(&mut self, nome: Option<String>) -> Result<(), String> {
        self.principal = None;
        self.anuncio = None;
        self._stream = None;
        self.handle = None;
        let host = cpal::default_host();
        let dev = match &nome {
            Some(n) => host
                .output_devices()
                .map_err(|e| e.to_string())?
                .find(|d| d.name().map(|x| &x == n).unwrap_or(false))
                .ok_or_else(|| format!("Dispositivo \"{}\" não encontrado. Verifique se o Bluetooth está ligado e pareado.", n))?,
            None => host.default_output_device().ok_or("Nenhum dispositivo de saída de áudio no Windows")?,
        };
        let dn = dev.name().unwrap_or_else(|_| "desconhecido".into());
        let (s, h) = OutputStream::try_from_device(&dev).map_err(|e| format!("Não foi possível abrir \"{}\": {}", dn, e))?;
        self._stream = Some(s);
        self.handle = Some(h);
        self.nome_dispositivo = dn;
        self.escolhido = nome;
        self.perdido = false;
        Ok(())
    }

    fn novo_canal(&self) -> Option<Canal> {
        let h = self.handle.as_ref()?;
        let sink = Sink::try_new(h).ok()?;
        sink.set_volume(0.0);
        Some(Canal { sink, env: 0.0, rampa: None })
    }

    fn master(&self) -> f32 {
        self.geral.clamp(0.0, 1.0)
    }

    fn rodar(&mut self, rx: Receiver<Cmd>) {
        loop {
            match rx.recv_timeout(Duration::from_millis(25)) {
                Ok(c) => self.comando(c),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
            self.passo();
        }
    }

    fn comando(&mut self, c: Cmd) {
        match c {
            Cmd::Tocar { itens, fade_in_ms, fade_saida_ms, limite_ms, fade_fim_ms, id, posicao_ms } => {
                let p = Pendente { itens, fade_in_ms, limite_ms, fade_fim_ms, id, posicao_ms };
                let tocando = self.principal.as_ref().map(|c| !c.sink.empty() && c.env > 0.01).unwrap_or(false);
                if tocando && fade_saida_ms > 0 {
                    // Fade-out do atual; o novo começa só quando terminar (sem mistura).
                    if let Some(c) = &mut self.principal {
                        c.rampa(0.0, fade_saida_ms, AoFim::Parar);
                    }
                    self.proximo = Some(p);
                } else {
                    self.principal = None;
                    self.iniciar_principal(p);
                }
            }
            Cmd::Anunciar { itens, pausar_musica, nivel_duck } => self.anunciar(itens, pausar_musica, nivel_duck),
            Cmd::Pausar { fade_ms } => {
                if let Some(c) = &mut self.principal {
                    c.rampa(0.0, fade_ms, AoFim::Pausar);
                    self.pausado = true;
                }
            }
            Cmd::Continuar { fade_ms } => {
                if self.perdido {
                    return;
                }
                if let Some(c) = &mut self.principal {
                    c.sink.play();
                    c.rampa(1.0, fade_ms, AoFim::Nada);
                    self.pausado = false;
                }
            }
            Cmd::Parar { fade_ms } => {
                self.proximo = None;
                if let Some(c) = &mut self.principal {
                    if fade_ms == 0 {
                        c.sink.stop();
                        self.principal = None;
                    } else {
                        c.rampa(0.0, fade_ms, AoFim::Parar);
                    }
                }
                self.id = 0;
            }
            Cmd::Silenciar => {
                self.proximo = None;
                if let Some(c) = &self.principal {
                    c.sink.set_volume(0.0);
                    c.sink.stop();
                }
                if let Some(c) = &self.anuncio {
                    c.sink.set_volume(0.0);
                    c.sink.stop();
                }
                self.principal = None;
                self.anuncio = None;
                self.mic_ativo.store(false, Ordering::Relaxed);
                self.mic_falando = false;
                self.pausado = false;
                self.pausado_para_anuncio = false;
                self.duck_ativo = false;
                self.id = 0;
                self.itens.clear();
            }
            Cmd::Volume { geral } => self.geral = geral.clamp(0.0, 1.0),
            Cmd::Buscar { ms } => {
                if let Some(c) = &self.principal {
                    let _ = c.sink.try_seek(Duration::from_millis(ms));
                }
            }
            Cmd::SelecionarDispositivo { nome } | Cmd::Reabrir { nome } => self.reabrir(nome),
            Cmd::Teste { ganho } => {
                // Som suave de teste no canal de anúncio (não interfere na música).
                self.anunciar(vec![ItemAudio::beep(ganho)], false, 1.0);
            }
            Cmd::MicAbrir { nome } => self.mic_abrir(nome),
            Cmd::MicFechar => {
                self.mic_ativo.store(false, Ordering::Relaxed);
                self.mic_stream = None;
                self.mic_falando = false;
                self.mic_nome = None;
                self.terminar_anuncio();
            }
            Cmd::MicFalar { falar, pausar_musica } => self.mic_falar(falar, pausar_musica),
            Cmd::MicAjuste { ganho, portao } => {
                self.mic_ganho.store(ganho.clamp(0.0, 1.5).to_bits(), Ordering::Relaxed);
                self.mic_portao.store(portao.clamp(0.0, 0.5).to_bits(), Ordering::Relaxed);
            }
        }
    }

    fn iniciar_principal(&mut self, p: Pendente) {
        self.id = p.id;
        self.itens = p.itens.clone();
        self.limite_ms = p.limite_ms;
        self.fade_fim_ms = p.fade_fim_ms;
        self.encerrando_limite = false;
        self.pausado = false;
        if self.perdido {
            let _ = self.ev.send(EventoAudio::Erro { id: p.id, mensagem: "Saída de áudio indisponível; nada foi tocado".into(), faixa_id: None });
            self.id = 0;
            return;
        }
        let mut canal = match self.novo_canal() {
            Some(c) => c,
            None => {
                let _ = self.ev.send(EventoAudio::Erro { id: p.id, mensagem: "Não foi possível criar a saída de áudio".into(), faixa_id: None });
                self.id = 0;
                return;
            }
        };
        let mut algum = false;
        let mut validos = Vec::new();
        for it in &p.itens {
            match abrir_fonte(it, &self.pico) {
                Ok(s) => {
                    canal.sink.append(s);
                    validos.push(it.clone());
                    algum = true;
                }
                Err(m) => {
                    let _ = self.ev.send(EventoAudio::Erro { id: p.id, mensagem: m, faixa_id: it.faixa_id.clone() });
                }
            }
        }
        if !algum {
            let _ = self.ev.send(EventoAudio::Terminou { id: p.id });
            self.id = 0;
            return;
        }
        self.itens = validos;
        if p.posicao_ms > 0 {
            let _ = canal.sink.try_seek(Duration::from_millis(p.posicao_ms));
        }
        canal.sink.play();
        canal.rampa(1.0, p.fade_in_ms.max(30), AoFim::Nada);
        // Se um anúncio estiver no ar, a nova música espera pausada.
        if self.anuncio.is_some() && self.pausado_para_anuncio {
            canal.sink.pause();
            canal.rampa = None;
            canal.env = 0.0;
        }
        self.principal = Some(canal);
    }

    fn anunciar(&mut self, itens: Vec<ItemAudio>, pausar_musica: bool, nivel_duck: f32) {
        if self.perdido {
            let _ = self.ev.send(EventoAudio::Erro { id: 0, mensagem: "Saída de áudio indisponível; recado não tocado".into(), faixa_id: None });
            return;
        }
        let mut canal = match self.anuncio.take().or_else(|| self.novo_canal()) {
            Some(c) => c,
            None => return,
        };
        // Música: pausa com fade (ou abaixa).
        if let Some(c) = &mut self.principal {
            if !self.pausado {
                if pausar_musica {
                    c.rampa(0.0, 800, AoFim::Pausar);
                    self.pausado_para_anuncio = true;
                } else {
                    c.rampa(nivel_duck.clamp(0.0, 1.0), 800, AoFim::Nada);
                    self.duck_ativo = true;
                }
            }
        }
        for it in &itens {
            match abrir_fonte(it, &self.pico) {
                Ok(s) => canal.sink.append(s),
                Err(m) => {
                    let _ = self.ev.send(EventoAudio::Erro { id: 0, mensagem: m, faixa_id: it.faixa_id.clone() });
                }
            }
        }
        canal.env = 1.0;
        canal.sink.set_volume(self.master());
        canal.sink.play();
        self.anuncio = Some(canal);
    }

    fn terminar_anuncio(&mut self) {
        if self.mic_falando {
            return;
        }
        self.anuncio = None;
        if let Some(c) = &mut self.principal {
            if self.pausado_para_anuncio && !self.pausado {
                c.sink.play();
                c.rampa(1.0, 1500, AoFim::Nada);
            } else if self.duck_ativo {
                c.rampa(1.0, 1500, AoFim::Nada);
            }
        }
        self.pausado_para_anuncio = false;
        self.duck_ativo = false;
        let _ = self.ev.send(EventoAudio::AnuncioTerminou);
    }

    fn reabrir(&mut self, nome: Option<String>) {
        // Guarda o ponto para retomar PAUSADO (retomada explícita pelo operador).
        let pos = self.principal.as_ref().map(|c| c.sink.get_pos().as_millis() as u64).unwrap_or(0);
        let itens = self.itens.clone();
        let id = self.id;
        let idx = self.indice_atual();
        match self.abrir_saida(nome) {
            Ok(()) => {
                let _ = self.ev.send(EventoAudio::DispositivoReaberto { nome: self.nome_dispositivo.clone() });
                if id != 0 && !itens.is_empty() {
                    let resto: Vec<ItemAudio> = itens.into_iter().skip(idx).collect();
                    self.iniciar_principal(Pendente { itens: resto, fade_in_ms: 1500, limite_ms: self.limite_ms, fade_fim_ms: self.fade_fim_ms, id, posicao_ms: pos });
                    if let Some(c) = &mut self.principal {
                        c.sink.pause();
                        c.rampa = None;
                        c.env = 0.0;
                        self.pausado = true;
                    }
                }
            }
            Err(e) => {
                self.perdido = true;
                let _ = self.ev.send(EventoAudio::DispositivoPerdido { nome: self.escolhido.clone().unwrap_or_else(|| "padrão do Windows".into()), detalhe: e });
            }
        }
    }

    fn indice_atual(&self) -> usize {
        match &self.principal {
            Some(c) => self.itens.len().saturating_sub(c.sink.len()).min(self.itens.len().saturating_sub(1)),
            None => 0,
        }
    }

    fn verificar_dispositivo(&mut self) {
        if self.perdido || self.handle.is_none() {
            return;
        }
        // Lê só o snapshot da thread de sondagem (rápido, sem COM na thread de áudio).
        let (nomes, padrao) = match self.sondagem.lock() {
            Ok(g) if g.ok => (g.saidas.clone(), g.padrao.clone()),
            _ => return,
        };
        let presente = nomes.iter().any(|n| *n == self.nome_dispositivo);
        let detalhe = if !presente {
            Some(format!("\"{}\" foi desconectado.", self.nome_dispositivo))
        } else if self.escolhido.is_none() && padrao.as_deref().map(|p| p != self.nome_dispositivo).unwrap_or(false) {
            Some(format!(
                "O Windows trocou a saída padrão de \"{}\" para \"{}\" (provável queda do Bluetooth).",
                self.nome_dispositivo,
                padrao.clone().unwrap_or_default()
            ))
        } else {
            None
        };
        if let Some(d) = detalhe {
            self.perdido = true;
            if let Some(c) = &mut self.principal {
                c.sink.pause();
                c.env = 0.0;
                c.rampa = None;
                c.sink.set_volume(0.0);
                self.pausado = true;
            }
            if let Some(c) = &self.anuncio {
                c.sink.stop();
            }
            self.anuncio = None;
            let _ = self.ev.send(EventoAudio::DispositivoPerdido { nome: self.nome_dispositivo.clone(), detalhe: d });
        }
    }

    fn mic_abrir(&mut self, nome: Option<String>) {
        self.mic_stream = None;
        self.erro_mic = None;
        let host = cpal::default_host();
        let dev = match &nome {
            Some(n) => host.input_devices().ok().and_then(|mut it| it.find(|d| d.name().map(|x| &x == n).unwrap_or(false))),
            None => host.default_input_device(),
        };
        let Some(dev) = dev else {
            self.erro_mic = Some("Microfone não encontrado. Verifique a permissão de microfone do Windows.".into());
            return;
        };
        let cfg = match dev.default_input_config() {
            Ok(c) => c,
            Err(e) => {
                self.erro_mic = Some(format!("Microfone indisponível: {}", e));
                return;
            }
        };
        let canais = cfg.channels();
        let taxa = cfg.sample_rate().0;
        let sc: cpal::StreamConfig = cfg.clone().into();
        let limite = (taxa as usize * canais as usize) / 2; // no máx. 0,5 s de atraso
        let buf = self.mic_buf.clone();
        let pico = self.mic_pico.clone();
        let falando = self.mic_ativo.clone();
        let empurra = move |amostras: &mut dyn Iterator<Item = f32>| {
            let mut p = 0f32;
            let ligado = falando.load(Ordering::Relaxed);
            if let Ok(mut b) = buf.lock() {
                for s in amostras {
                    p = p.max(s.abs());
                    if ligado {
                        b.push_back(s);
                    }
                }
                while b.len() > limite {
                    b.pop_front();
                }
                if !ligado {
                    b.clear();
                }
            }
            pico.store(p.to_bits(), Ordering::Relaxed);
        };
        let erro = |e: cpal::StreamError| eprintln!("erro no microfone: {}", e);
        let res = match cfg.sample_format() {
            cpal::SampleFormat::F32 => {
                let f = empurra;
                dev.build_input_stream(&sc, move |d: &[f32], _: &cpal::InputCallbackInfo| f(&mut d.iter().copied()), erro, None)
            }
            cpal::SampleFormat::I16 => {
                let f = empurra;
                dev.build_input_stream(&sc, move |d: &[i16], _: &cpal::InputCallbackInfo| f(&mut d.iter().map(|s| *s as f32 / 32768.0)), erro, None)
            }
            cpal::SampleFormat::U16 => {
                let f = empurra;
                dev.build_input_stream(
                    &sc,
                    move |d: &[u16], _: &cpal::InputCallbackInfo| f(&mut d.iter().map(|s| (*s as f32 - 32768.0) / 32768.0)),
                    erro,
                    None,
                )
            }
            outro => {
                self.erro_mic = Some(format!("Formato de microfone não suportado: {:?}", outro));
                return;
            }
        };
        match res {
            Ok(s) => {
                if let Err(e) = s.play() {
                    self.erro_mic = Some(format!("Não foi possível iniciar o microfone: {}", e));
                    return;
                }
                self.mic_stream = Some(s);
                self.mic_formato = (canais, taxa);
                self.mic_nome = Some(dev.name().unwrap_or_default());
            }
            Err(e) => self.erro_mic = Some(format!("Não foi possível abrir o microfone: {}", e)),
        }
    }

    fn mic_falar(&mut self, falar: bool, pausar_musica: bool) {
        if falar {
            if self.mic_stream.is_none() || self.perdido {
                self.erro_mic = Some("Abra o microfone e confira a saída de áudio antes de falar".into());
                return;
            }
            self.mic_ativo.store(true, Ordering::Relaxed);
            let fonte = FonteMic {
                buf: self.mic_buf.clone(),
                canais: self.mic_formato.0,
                taxa: self.mic_formato.1,
                ganho: self.mic_ganho.clone(),
                portao: self.mic_portao.clone(),
                ativo: self.mic_ativo.clone(),
                env: 0.0,
            };
            self.mic_falando = true;
            self.mic_saturado_desde = None;
            // Pausa/abaixa música e usa o canal de anúncio para a voz.
            self.anunciar(vec![], pausar_musica, 0.15);
            if let Some(c) = &self.anuncio {
                c.sink.append(fonte);
            }
        } else {
            self.mic_ativo.store(false, Ordering::Relaxed);
            self.mic_falando = false;
            if let Some(c) = &self.anuncio {
                c.sink.stop();
            }
            self.terminar_anuncio();
        }
    }

    fn passo(&mut self) {
        let master = self.master();
        // Canal principal
        let mut fim_principal = AoFim::Nada;
        if let Some(c) = &mut self.principal {
            fim_principal = c.atualizar(master);
        }
        match fim_principal {
            AoFim::Pausar => {
                if let Some(c) = &self.principal {
                    c.sink.pause();
                }
            }
            AoFim::Parar => {
                if let Some(c) = &self.principal {
                    c.sink.stop();
                }
                self.principal = None;
                let id_antigo = self.id;
                if let Some(p) = self.proximo.take() {
                    self.iniciar_principal(p);
                } else if id_antigo != 0 {
                    self.id = 0;
                    let _ = self.ev.send(EventoAudio::Terminou { id: id_antigo });
                }
            }
            AoFim::Nada => {}
        }
        // Limite de duração (trecho) com fade.
        if let (Some(lim), Some(c)) = (self.limite_ms, &mut self.principal) {
            let pos = c.sink.get_pos().as_millis() as u64;
            if !self.encerrando_limite && c.sink.len() <= 1 && pos + self.fade_fim_ms as u64 >= lim {
                self.encerrando_limite = true;
                c.rampa(0.0, self.fade_fim_ms, AoFim::Parar);
            }
        }
        // Fim natural.
        if let Some(c) = &self.principal {
            if c.sink.empty() && !self.perdido {
                self.principal = None;
                if self.id != 0 {
                    let id = self.id;
                    self.id = 0;
                    let _ = self.ev.send(EventoAudio::Terminou { id });
                }
                if let Some(p) = self.proximo.take() {
                    self.iniciar_principal(p);
                }
            }
        }
        // Canal de anúncio
        if let Some(c) = &mut self.anuncio {
            c.atualizar(master);
        }
        if let Some(c) = &self.anuncio {
            if c.sink.empty() && !self.mic_falando {
                self.terminar_anuncio();
            }
        }
        // Microfonia: nível saturado por 1,5 s enquanto fala → corta e alerta.
        if self.mic_falando {
            let p = f32::from_bits(self.mic_pico.load(Ordering::Relaxed));
            if p > 0.97 {
                let desde = *self.mic_saturado_desde.get_or_insert_with(Instant::now);
                if desde.elapsed() > Duration::from_millis(1500) {
                    self.mic_falar(false, true);
                    let _ = self.ev.send(EventoAudio::Microfonia);
                }
            } else {
                self.mic_saturado_desde = None;
            }
        }
        if self.ultima_verificacao.elapsed() > Duration::from_secs(5) {
            self.ultima_verificacao = Instant::now();
            self.verificar_dispositivo();
        }
        self.publicar();
    }

    fn publicar(&mut self) {
        let idx = self.indice_atual();
        let pico = f32::from_bits(self.pico.swap(0, Ordering::Relaxed));
        if let Ok(mut s) = self.status.lock() {
            s.tocando = self.principal.as_ref().map(|c| !c.sink.empty() && !c.sink.is_paused()).unwrap_or(false);
            s.pausado = self.pausado;
            s.id = self.id;
            s.item = if self.principal.is_some() { self.itens.get(idx).cloned() } else { None };
            s.indice_item = idx;
            s.total_itens = if self.principal.is_some() { self.itens.len() } else { 0 };
            s.posicao_ms = self.principal.as_ref().map(|c| c.sink.get_pos().as_millis() as u64).unwrap_or(0);
            s.duracao_ms = s.item.as_ref().and_then(|i| i.duracao_ms);
            if pico > 0.0 {
                s.nivel = pico;
            } else {
                s.nivel *= 0.85;
            }
            s.volume_geral = self.geral;
            s.em_anuncio = self.anuncio.is_some();
            s.dispositivo = self.nome_dispositivo.clone();
            s.dispositivo_padrao_windows = self.escolhido.is_none();
            s.dispositivo_ok = !self.perdido;
            if !self.perdido {
                s.alerta_dispositivo = None;
            }
            s.mic_aberto = self.mic_stream.is_some();
            s.mic_falando = self.mic_falando;
            s.mic_nivel = f32::from_bits(self.mic_pico.load(Ordering::Relaxed));
            s.mic_dispositivo = self.mic_nome.clone();
            s.erro_mic = self.erro_mic.clone();
        }
    }
}
