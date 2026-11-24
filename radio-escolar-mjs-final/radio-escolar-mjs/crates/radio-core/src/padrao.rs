//! Agenda inicial da E.E. Mestre João Silvério (editável na tela Programação).

use crate::modelo::*;

pub fn perfis_padrao() -> Vec<Perfil> {
    [
        ("normal", "Dia normal"),
        ("prova", "Dia de prova"),
        ("evento", "Dia de evento"),
        ("reuniao", "Reunião"),
        ("feriado", "Feriado"),
        ("sabado_letivo", "Sábado letivo"),
        ("reduzido", "Dia reduzido"),
    ]
    .iter()
    .map(|(id, n)| Perfil { id: id.to_string(), nome: n.to_string() })
    .collect()
}

fn base(id: &str, nome: &str, tipo: TipoEvento, inicio: &str, fim: Option<&str>) -> Evento {
    let janela = tipo.eh_janela();
    Evento {
        id: id.into(),
        nome: nome.into(),
        tipo,
        inicio: inicio.into(),
        fim: fim.map(|s| s.to_string()),
        // Perfis em que a programação normal vale. "evento", "reuniao", "feriado"
        // e "reduzido" começam vazios para o funcionário configurar.
        perfis: vec!["normal".into(), "prova".into(), "sabado_letivo".into()],
        volume: if janela { 60 } else { 55 },
        fade_in_ms: if janela { 3000 } else { 1500 },
        fade_out_ms: if janela { 6000 } else { 3000 },
        prioridade: tipo.prioridade_padrao(),
        ..Evento::default()
    }
}

pub fn agenda_padrao() -> Agenda {
    use TipoEvento::*;
    let mut ev = vec![
        base("manha-recepcao", "Recepção da manhã", Recepcao, "06:45", Some("07:05")),
        base("manha-sinal-0750", "Troca 07:50", Sinal, "07:50", None),
        base("manha-sinal-0840", "Troca 08:40", Sinal, "08:40", None),
        base("manha-sinal-0930", "Troca 09:30", Sinal, "09:30", None),
        base("manha-recreio", "Recreio da manhã", Recreio, "09:30", Some("09:47")),
        base("manha-sinal-1035", "Troca 10:35", Sinal, "10:35", None),
        base("manha-sinal-1125", "Troca 11:25", Sinal, "11:25", None),
        base("manha-saida-1215", "Saída 12:15", Saida, "12:15", None),
        base("tarde-recepcao", "Recepção da tarde", Recepcao, "12:45", Some("13:05")),
        base("tarde-sinal-1350", "Troca 13:50", Sinal, "13:50", None),
        base("tarde-sinal-1440", "Troca 14:40", Sinal, "14:40", None),
        base("tarde-sinal-1530", "Troca 15:30", Sinal, "15:30", None),
        base("tarde-recreio", "Recreio da tarde", Recreio, "15:30", Some("15:47")),
        base("tarde-sinal-1635", "Troca 16:35", Sinal, "16:35", None),
        base("tarde-saida-1725", "Saída 17:25", Saida, "17:25", None),
    ];
    for e in ev.iter_mut() {
        match e.id.as_str() {
            "manha-recreio" | "tarde-recreio" => {
                e.observacao = "Aula retorna às :45; música segue até :47. Prevalece sobre a sinalização do mesmo minuto.".into()
            }
            "manha-sinal-0930" | "tarde-sinal-1530" => {
                e.observacao = "Coincide com o recreio: por padrão não toca (o recreio prevalece).".into()
            }
            "tarde-sinal-1350" => e.observacao = "Sinalização solicitada pela escola.".into(),
            _ => {}
        }
    }
    Agenda {
        eventos: ev,
        perfis: perfis_padrao(),
        perfil_por_dia_semana: vec![
            Some("normal".into()),
            Some("normal".into()),
            Some("normal".into()),
            Some("normal".into()),
            Some("normal".into()),
            None,
            None,
        ],
        ..Agenda::default()
    }
}
