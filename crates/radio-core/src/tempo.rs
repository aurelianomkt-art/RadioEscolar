//! Datas e horários locais, sem dependências externas.
//!
//! O agendador trabalha sempre com o relógio LOCAL do Windows, convertido
//! pelo processo nativo para `Momento` (data + segundos desde a meia-noite).

use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Data {
    pub ano: i32,
    pub mes: u8,
    pub dia: u8,
}

impl Data {
    pub fn new(ano: i32, mes: u8, dia: u8) -> Option<Data> {
        if !(1..=12).contains(&mes) || dia == 0 || dia > dias_no_mes(ano, mes) {
            return None;
        }
        Some(Data { ano, mes, dia })
    }

    /// Aceita "AAAA-MM-DD".
    pub fn parse(s: &str) -> Option<Data> {
        let p: Vec<&str> = s.trim().split('-').collect();
        if p.len() != 3 {
            return None;
        }
        Data::new(p[0].parse().ok()?, p[1].parse().ok()?, p[2].parse().ok()?)
    }

    /// Dias desde 1970-01-01 (algoritmo de Howard Hinnant).
    pub fn para_dias(&self) -> i64 {
        let y = if self.mes <= 2 { self.ano as i64 - 1 } else { self.ano as i64 };
        let era = if y >= 0 { y } else { y - 399 } / 400;
        let yoe = y - era * 400;
        let m = self.mes as i64;
        let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + self.dia as i64 - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146097 + doe - 719468
    }

    pub fn de_dias(z: i64) -> Data {
        let z = z + 719468;
        let era = if z >= 0 { z } else { z - 146096 } / 146097;
        let doe = z - era * 146097;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = (doy - (153 * mp + 2) / 5 + 1) as u8;
        let m = if mp < 10 { mp + 3 } else { mp - 9 } as u8;
        Data { ano: (if m <= 2 { y + 1 } else { y }) as i32, mes: m, dia: d }
    }

    /// 0 = segunda-feira … 6 = domingo.
    pub fn dia_semana(&self) -> u8 {
        // 1970-01-01 foi quinta-feira (índice 3).
        ((self.para_dias() + 3).rem_euclid(7)) as u8
    }

    pub fn somar_dias(&self, n: i64) -> Data {
        Data::de_dias(self.para_dias() + n)
    }
}

impl fmt::Display for Data {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.ano, self.mes, self.dia)
    }
}

pub fn bissexto(a: i32) -> bool {
    (a % 4 == 0 && a % 100 != 0) || a % 400 == 0
}

pub fn dias_no_mes(a: i32, m: u8) -> u8 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if bissexto(a) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

/// Instante local: data + segundos desde a meia-noite.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Momento {
    pub data: Data,
    pub seg: u32,
}

impl Momento {
    pub fn new(data: Data, seg: u32) -> Momento {
        Momento { data, seg: seg.min(86_399) }
    }
    /// Segundos absolutos (para diferenças).
    pub fn abs(&self) -> i64 {
        self.data.para_dias() * 86_400 + self.seg as i64
    }
    pub fn de_abs(a: i64) -> Momento {
        Momento { data: Data::de_dias(a.div_euclid(86_400)), seg: a.rem_euclid(86_400) as u32 }
    }
    pub fn somar(&self, s: i64) -> Momento {
        Momento::de_abs(self.abs() + s)
    }
}

impl fmt::Display for Momento {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.data, fmt_hms(self.seg))
    }
}

/// Aceita "HH:MM" ou "HH:MM:SS" e devolve segundos desde a meia-noite.
pub fn parse_hora(s: &str) -> Option<u32> {
    let p: Vec<&str> = s.trim().split(':').collect();
    if p.len() < 2 || p.len() > 3 {
        return None;
    }
    let h: u32 = p[0].parse().ok()?;
    let m: u32 = p[1].parse().ok()?;
    let sec: u32 = if p.len() == 3 { p[2].parse().ok()? } else { 0 };
    if h > 23 || m > 59 || sec > 59 {
        return None;
    }
    Some(h * 3600 + m * 60 + sec)
}

pub fn fmt_hm(seg: u32) -> String {
    format!("{:02}:{:02}", seg / 3600, (seg % 3600) / 60)
}

pub fn fmt_hms(seg: u32) -> String {
    format!("{:02}:{:02}:{:02}", seg / 3600, (seg % 3600) / 60, seg % 60)
}

/// Nome curto do dia da semana em português (0 = segunda).
pub fn nome_dia(i: u8) -> &'static str {
    ["Seg", "Ter", "Qua", "Qui", "Sex", "Sáb", "Dom"][(i % 7) as usize]
}
