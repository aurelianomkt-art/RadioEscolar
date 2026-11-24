//! Dados de demonstração: áudios SINTETIZADOS e claramente marcados "DEMO",
//! gerados localmente (sem internet) para testar agenda, recreio, recados e Rádio Livre
//! antes de a escola escolher suas próprias músicas.

use std::f32::consts::PI;
use std::io::Write;
use std::path::Path;

const TAXA: u32 = 22_050;

fn gravar_wav(caminho: &Path, amostras: &[f32]) -> std::io::Result<()> {
    let mut f = std::io::BufWriter::new(std::fs::File::create(caminho)?);
    let dados = (amostras.len() * 2) as u32;
    f.write_all(b"RIFF")?;
    f.write_all(&(36 + dados).to_le_bytes())?;
    f.write_all(b"WAVEfmt ")?;
    f.write_all(&16u32.to_le_bytes())?;
    f.write_all(&1u16.to_le_bytes())?; // PCM
    f.write_all(&1u16.to_le_bytes())?; // mono
    f.write_all(&TAXA.to_le_bytes())?;
    f.write_all(&(TAXA * 2).to_le_bytes())?;
    f.write_all(&2u16.to_le_bytes())?;
    f.write_all(&16u16.to_le_bytes())?;
    f.write_all(b"data")?;
    f.write_all(&dados.to_le_bytes())?;
    for s in amostras {
        let v = (s.clamp(-1.0, 1.0) * 32000.0) as i16;
        f.write_all(&v.to_le_bytes())?;
    }
    f.flush()
}

fn nota(freq: f32, dur: f32, vol: f32, out: &mut Vec<f32>) {
    let n = (dur * TAXA as f32) as usize;
    for i in 0..n {
        let t = i as f32 / TAXA as f32;
        // envelope suave (ataque 20 ms, decaimento exponencial)
        let env = (t / 0.02).min(1.0) * (-2.2 * t / dur).exp();
        let s = (2.0 * PI * freq * t).sin() * 0.7 + (2.0 * PI * freq * 2.0 * t).sin() * 0.2 + (2.0 * PI * freq * 3.0 * t).sin() * 0.1;
        out.push(s * env * vol);
    }
}

fn melodia(semente: u32, segundos: f32) -> Vec<f32> {
    // Escala pentatônica maior, andamento calmo.
    let base = [261.63f32, 293.66, 329.63, 392.0, 440.0, 523.25, 587.33, 659.25];
    let mut x = semente.wrapping_mul(2654435761).max(1);
    let mut out = Vec::new();
    let mut i = 3usize;
    while (out.len() as f32) < segundos * TAXA as f32 {
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        let passo = (x % 5) as i32 - 2;
        i = (i as i32 + passo).clamp(0, base.len() as i32 - 1) as usize;
        let dur = if x % 7 == 0 { 0.9 } else { 0.45 };
        nota(base[i], dur, 0.35, &mut out);
    }
    // fade final de 1,5 s
    let n = out.len();
    let f = (1.5 * TAXA as f32) as usize;
    for k in 0..f.min(n) {
        out[n - 1 - k] *= k as f32 / f as f32;
    }
    out
}

/// Cria os arquivos e devolve (pasta músicas, pasta vinhetas, pasta recados).
pub fn criar(dados: &Path) -> std::io::Result<(String, String, String)> {
    let raiz = dados.join("DEMO - audios sintetizados");
    let m = raiz.join("Musicas DEMO");
    let v = raiz.join("Vinhetas DEMO");
    let r = raiz.join("Recados DEMO");
    for p in [&m, &v, &r] {
        std::fs::create_dir_all(p)?;
    }
    for k in 1..=8u32 {
        gravar_wav(&m.join(format!("DEMO - Melodia {:02} (sintetizada).wav", k)), &melodia(k * 7919, 30.0 + (k % 3) as f32 * 10.0))?;
    }
    let mut vin = Vec::new();
    for f in [523.25f32, 659.25, 783.99, 1046.5] {
        nota(f, 0.35, 0.4, &mut vin);
    }
    gravar_wav(&v.join("DEMO - Vinheta de abertura.wav"), &vin)?;
    let mut rec = Vec::new();
    for _ in 0..3 {
        nota(440.0, 0.25, 0.4, &mut rec);
        rec.extend(std::iter::repeat(0.0).take((0.25 * TAXA as f32) as usize));
    }
    nota(392.0, 1.2, 0.4, &mut rec);
    gravar_wav(&r.join("DEMO - Recado (tons, sem voz).wav"), &rec)?;
    Ok((m.to_string_lossy().to_string(), v.to_string_lossy().to_string(), r.to_string_lossy().to_string()))
}
