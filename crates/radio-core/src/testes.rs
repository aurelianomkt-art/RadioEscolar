//! Testes automatizados do agendador (critérios de aceite do documento).

use crate::*;

// 2026-11-16 é segunda-feira.
fn seg() -> Data {
    Data::new(2026, 11, 16).unwrap()
}
fn m(d: Data, hms: &str) -> Momento {
    Momento::new(d, parse_hora(hms).unwrap())
}
fn iniciados(a: &[Acao]) -> Vec<(String, bool, bool)> {
    a.iter()
        .filter_map(|x| match x {
            Acao::Iniciar { evento, retomada, interromper_atual } => Some((evento.id.clone(), *retomada, *interromper_atual)),
            _ => None,
        })
        .collect()
}
fn registros(a: &[Acao]) -> Vec<String> {
    a.iter()
        .filter_map(|x| match x {
            Acao::Registro { mensagem, .. } => Some(mensagem.clone()),
            _ => None,
        })
        .collect()
}
/// Liga o agendador às `ini` e avança segundo a segundo até `fim`, acumulando ações.
fn rodar(ag: &mut Agendador, agenda: &Agenda, d: Data, ini: &str, fim: &str, modo: Modo) -> Vec<Acao> {
    let mut out = Vec::new();
    let (a, b) = (parse_hora(ini).unwrap(), parse_hora(fim).unwrap());
    for s in a..=b {
        out.extend(ag.tick(agenda, Momento::new(d, s), modo, false));
    }
    out
}

#[test]
fn calendario_basico() {
    assert_eq!(seg().dia_semana(), 0);
    assert_eq!(Data::new(2026, 11, 17).unwrap().dia_semana(), 1);
    assert_eq!(Data::new(2026, 11, 22).unwrap().dia_semana(), 6);
    assert_eq!(Data::new(2028, 2, 28).unwrap().somar_dias(1), Data::new(2028, 2, 29).unwrap());
    assert_eq!(Data::new(2026, 12, 31).unwrap().somar_dias(1), Data::new(2027, 1, 1).unwrap());
    assert!(Data::new(2026, 2, 29).is_none());
    assert_eq!(Data::parse("2026-11-16"), Some(seg()));
    assert_eq!(parse_hora("09:47"), Some(9 * 3600 + 47 * 60));
    assert_eq!(parse_hora("24:00"), None);
    let x = Momento::new(seg(), 86_399).somar(2);
    assert_eq!(x.data, Data::new(2026, 11, 17).unwrap());
    assert_eq!(x.seg, 1);
}

#[test]
fn agenda_padrao_valida_e_informa_coincidencias() {
    let a = agenda_padrao();
    let av = validar(&a);
    assert!(!tem_erro(&av), "agenda padrão não pode ter erros: {:?}", av);
    let coinc: Vec<&Aviso> = av.iter().filter(|x| x.mensagem.contains("começam juntos")).collect();
    assert_eq!(coinc.len(), 2, "{:?}", coinc);
    assert!(coinc.iter().all(|x| x.mensagem.contains("Recreio")));
    assert_eq!(a.eventos_do_dia(seg()).len(), 15);
}

#[test]
fn troca_de_professor_toca_uma_vez() {
    let a = agenda_padrao();
    let mut ag = Agendador::new();
    let acoes = rodar(&mut ag, &a, seg(), "07:45:00", "07:52:00", Modo::Escolar);
    assert_eq!(iniciados(&acoes), vec![("manha-sinal-0750".to_string(), false, false)]);
    // Ao terminar a música única, para (não emenda outra).
    assert_eq!(ag.faixa_terminou(m(seg(), "07:53:30")), Proximo::Parar);
}

#[test]
fn recreio_prevalece_sobre_sinal_no_mesmo_minuto() {
    let a = agenda_padrao();
    let mut ag = Agendador::new();
    let acoes = rodar(&mut ag, &a, seg(), "09:29:00", "09:31:00", Modo::Escolar);
    assert_eq!(iniciados(&acoes), vec![("manha-recreio".to_string(), false, false)]);
    assert!(registros(&acoes).iter().any(|r| r.contains("Troca 09:30 suprimido")));
    // Tarde também.
    let mut ag = Agendador::new();
    let acoes = rodar(&mut ag, &a, seg(), "15:29:30", "15:30:30", Modo::Escolar);
    assert_eq!(iniciados(&acoes), vec![("tarde-recreio".to_string(), false, false)]);
}

#[test]
fn recreio_toca_sequencia_ate_0947_e_faz_fade() {
    let a = agenda_padrao();
    let mut ag = Agendador::new();
    rodar(&mut ag, &a, seg(), "09:29:50", "09:30:05", Modo::Escolar);
    assert!(matches!(ag.faixa_terminou(m(seg(), "09:33:40")), Proximo::TocarMais(e) if e.id == "manha-recreio"));
    assert!(matches!(ag.faixa_terminou(m(seg(), "09:44:59")), Proximo::TocarMais(_)));
    // Às 09:45 a aula retorna, mas a música segue até 09:47 (configurado).
    let mut fim = Vec::new();
    for s in parse_hora("09:30:06").unwrap()..=parse_hora("09:47:10").unwrap() {
        fim.extend(ag.tick(&a, Momento::new(seg(), s), Modo::Escolar, false));
    }
    let enc: Vec<&Acao> = fim.iter().filter(|x| matches!(x, Acao::EncerrarJanela { .. })).collect();
    assert_eq!(enc.len(), 1);
    assert!(matches!(enc[0], Acao::EncerrarJanela { modo: FimJanela::FadeOut, .. }));
    assert!(ag.ativo.is_none());
    assert_eq!(ag.faixa_terminou(m(seg(), "09:47:12")), Proximo::Parar);
}

#[test]
fn recepcao_terminar_faixa_com_teto() {
    let mut a = agenda_padrao();
    for e in a.eventos.iter_mut() {
        if e.id == "manha-recepcao" {
            e.fim_janela = FimJanela::TerminarFaixa;
            e.extra_max_segundos = 120;
        }
    }
    let mut ag = Agendador::new();
    let acoes = rodar(&mut ag, &a, seg(), "06:44:00", "07:05:00", Modo::Escolar);
    assert_eq!(iniciados(&acoes)[0].0, "manha-recepcao");
    assert!(acoes.iter().any(|x| matches!(x, Acao::EncerrarJanela { modo: FimJanela::TerminarFaixa, .. })));
    // Ainda ativa, mas não continua após a faixa atual.
    assert!(ag.ativo.is_some());
    let mut ag2 = ag.clone();
    assert_eq!(ag2.faixa_terminou(m(seg(), "07:06:00")), Proximo::Parar);
    // Se a faixa for longa demais, teto força fade às 07:07.
    let mais = rodar(&mut ag, &a, seg(), "07:05:01", "07:07:05", Modo::Escolar);
    assert!(mais.iter().any(|x| matches!(x, Acao::EncerrarJanela { modo: FimJanela::FadeOut, .. })));
    assert!(ag.ativo.is_none());
}

#[test]
fn reinicio_sem_execucao_retroativa() {
    let a = agenda_padrao();
    let mut ag = Agendador::new();
    // PC ligado 5 s depois do sinal das 07:50: não toca.
    let acoes = rodar(&mut ag, &a, seg(), "07:50:05", "07:55:00", Modo::Escolar);
    assert!(iniciados(&acoes).is_empty(), "{:?}", acoes);
    // O próximo sinal toca normalmente.
    let acoes = rodar(&mut ag, &a, seg(), "07:55:01", "08:40:30", Modo::Escolar);
    assert_eq!(iniciados(&acoes), vec![("manha-sinal-0840".to_string(), false, false)]);
}

#[test]
fn reinicio_dentro_do_recreio_retoma_playlist_sem_sinal_antigo() {
    let a = agenda_padrao();
    let mut ag = Agendador::new();
    let acoes = ag.tick(&a, m(seg(), "09:35:00"), Modo::Escolar, false);
    assert_eq!(iniciados(&acoes), vec![("manha-recreio".to_string(), true, false)]);

    let mut a2 = agenda_padrao();
    a2.retomar_janela_apos_reinicio = false;
    let mut ag2 = Agendador::new();
    assert!(iniciados(&ag2.tick(&a2, m(seg(), "09:35:00"), Modo::Escolar, false)).is_empty());
}

#[test]
fn suspensao_hibernacao_nao_toca_eventos_perdidos() {
    let a = agenda_padrao();
    let mut ag = Agendador::new();
    rodar(&mut ag, &a, seg(), "07:40:00", "07:40:10", Modo::Escolar);
    // Notebook "dormiu" de 07:40:10 até 08:45:00.
    let acoes = ag.tick(&a, m(seg(), "08:45:00"), Modo::Escolar, false);
    assert!(iniciados(&acoes).is_empty());
    assert!(registros(&acoes).iter().any(|r| r.contains("NÃO serão tocados")));
    let acoes = rodar(&mut ag, &a, seg(), "08:45:01", "09:00:00", Modo::Escolar);
    assert!(iniciados(&acoes).is_empty());
}

#[test]
fn relogio_voltando_nao_repete_evento() {
    let a = agenda_padrao();
    let mut ag = Agendador::new();
    let acoes = rodar(&mut ag, &a, seg(), "07:49:50", "07:51:00", Modo::Escolar);
    assert_eq!(iniciados(&acoes).len(), 1);
    // Ajuste de relógio para 07:49:00 e passa de novo pelas 07:50.
    let acoes = ag.tick(&a, m(seg(), "07:49:00"), Modo::Escolar, false);
    assert!(registros(&acoes).iter().any(|r| r.contains("Relógio do Windows voltou")));
    let acoes = rodar(&mut ag, &a, seg(), "07:49:01", "07:51:00", Modo::Escolar);
    assert!(iniciados(&acoes).is_empty());
}

#[test]
fn radio_livre_suspende_e_volta_sem_retroativo() {
    let a = agenda_padrao();
    let mut ag = Agendador::new();
    rodar(&mut ag, &a, seg(), "07:40:00", "07:40:05", Modo::Escolar);
    ag.entrar_livre();
    let acoes = rodar(&mut ag, &a, seg(), "07:40:06", "09:35:00", Modo::Livre);
    assert!(iniciados(&acoes).is_empty());
    assert!(registros(&acoes).iter().any(|r| r.contains("Troca 07:50") && r.contains("Rádio Livre")));
    // Volta ao Rádio Escolar às 09:35:01: retoma a janela do recreio, sem sinais antigos.
    let acoes = ag.reavaliar(&a, m(seg(), "09:35:01"), Modo::Escolar, false, "Voltou ao Rádio Escolar");
    assert_eq!(iniciados(&acoes), vec![("manha-recreio".to_string(), true, false)]);
    let acoes = rodar(&mut ag, &a, seg(), "09:35:02", "10:35:05", Modo::Escolar);
    assert_eq!(iniciados(&acoes), vec![("manha-sinal-1035".to_string(), false, false)]);
}

#[test]
fn silenciar_bloqueia_ate_retomada_explicita() {
    let a = agenda_padrao();
    let mut ag = Agendador::new();
    rodar(&mut ag, &a, seg(), "09:29:58", "09:31:00", Modo::Escolar);
    ag.silenciar();
    let mut acoes = Vec::new();
    for s in parse_hora("09:31:01").unwrap()..=parse_hora("10:36:00").unwrap() {
        acoes.extend(ag.tick(&a, Momento::new(seg(), s), Modo::Escolar, true));
    }
    assert!(iniciados(&acoes).is_empty());
    assert!(registros(&acoes).iter().any(|r| r.contains("silenciada")));
    // Retomada explícita às 10:40: nada de surpresa; próximo é 11:25.
    let acoes = ag.reavaliar(&a, m(seg(), "10:40:00"), Modo::Escolar, false, "Retomada pelo operador");
    assert!(iniciados(&acoes).is_empty());
    let acoes = rodar(&mut ag, &a, seg(), "10:40:01", "11:25:01", Modo::Escolar);
    assert_eq!(iniciados(&acoes), vec![("manha-sinal-1125".to_string(), false, false)]);
}

#[test]
fn fim_de_semana_feriado_sabado_letivo_e_dia_suspenso() {
    let mut a = agenda_padrao();
    let sab = Data::new(2026, 11, 21).unwrap();
    assert!(a.eventos_do_dia(sab).is_empty());
    a.excecoes.push(Excecao { data: sab.to_string(), perfil: Some("sabado_letivo".into()), descricao: "Sábado letivo".into() });
    assert_eq!(a.eventos_do_dia(sab).len(), 15);
    let ter = Data::new(2026, 11, 17).unwrap();
    a.excecoes.push(Excecao { data: ter.to_string(), perfil: Some("feriado".into()), descricao: "Feriado municipal".into() });
    assert!(a.eventos_do_dia(ter).is_empty());
    let qua = Data::new(2026, 11, 18).unwrap();
    a.excecoes.push(Excecao { data: qua.to_string(), perfil: None, descricao: "Dia suspenso".into() });
    assert!(a.eventos_do_dia(qua).is_empty());
    assert!(matches!(a.perfil_do_dia(qua), PerfilDia::Suspenso(_)));
    let mut ag = Agendador::new();
    let acoes = rodar(&mut ag, &a, ter, "06:40:00", "10:00:00", Modo::Escolar);
    assert!(iniciados(&acoes).is_empty());
}

#[test]
fn virada_de_dia_e_proximo_evento() {
    let a = agenda_padrao();
    let sex = Data::new(2026, 11, 20).unwrap();
    let p = proximos(&a, m(sex, "18:00:00"), 3);
    assert_eq!(p[0].data, "2026-11-23");
    assert_eq!(p[0].inicio, "06:45");
    assert_eq!(p[0].em_segundos, (6 + 24 * 2) * 3600 + 12 * 3600 + 45 * 60 - 6 * 3600);
    // Itens suprimidos aparecem marcados na agenda do dia.
    let dia = agenda_do_dia(&a, seg(), m(seg(), "00:00:00"));
    let s930 = dia.iter().find(|i| i.evento_id == "manha-sinal-0930").unwrap();
    assert_eq!(s930.suprimido_por.as_deref(), Some("Recreio da manhã"));
    // Atravessar meia-noite com ticks não dispara nada nem trava.
    let mut ag = Agendador::new();
    ag.tick(&a, m(seg(), "23:59:58"), Modo::Escolar, false);
    let acoes = ag.tick(&a, m(Data::new(2026, 11, 17).unwrap(), "00:00:01"), Modo::Escolar, false);
    assert!(iniciados(&acoes).is_empty());
}

#[test]
fn politicas_de_conflito() {
    let mut a = agenda_padrao();
    let mut rec = Evento {
        id: "recado-0751".into(),
        nome: "Recado da direção".into(),
        tipo: TipoEvento::Recado,
        inicio: "07:51".into(),
        recado_id: Some("r1".into()),
        prioridade: 40,
        perfis: vec!["normal".into()],
        ..Evento::default()
    };
    // Interromper (padrão): sinal das 07:50 ainda tocando às 07:51.
    a.eventos.push(rec.clone());
    let mut ag = Agendador::new();
    let acoes = rodar(&mut ag, &a, seg(), "07:49:00", "07:51:05", Modo::Escolar);
    assert_eq!(
        iniciados(&acoes),
        vec![("manha-sinal-0750".to_string(), false, false), ("recado-0751".to_string(), false, true)]
    );
    // Aguardar.
    rec.conflito = PoliticaConflito::Aguardar;
    a.eventos.pop();
    a.eventos.push(rec.clone());
    let mut ag = Agendador::new();
    let acoes = rodar(&mut ag, &a, seg(), "07:49:00", "07:51:05", Modo::Escolar);
    assert_eq!(iniciados(&acoes).len(), 1);
    assert!(matches!(ag.faixa_terminou(m(seg(), "07:53:00")), Proximo::IniciarPendente(e) if e.id == "recado-0751"));
    // Ignorar.
    rec.conflito = PoliticaConflito::Ignorar;
    a.eventos.pop();
    a.eventos.push(rec.clone());
    let mut ag = Agendador::new();
    let acoes = rodar(&mut ag, &a, seg(), "07:49:00", "07:51:05", Modo::Escolar);
    assert_eq!(iniciados(&acoes).len(), 1);
    assert_eq!(ag.faixa_terminou(m(seg(), "07:53:00")), Proximo::Parar);
    // Evento de prioridade menor não interrompe o recreio.
    let mut s = Evento { id: "sinal-0935".into(), nome: "Sinal extra".into(), inicio: "09:35".into(), ..Evento::default() };
    s.perfis = vec!["normal".into()];
    a.eventos.push(s);
    let mut ag = Agendador::new();
    let acoes = rodar(&mut ag, &a, seg(), "09:29:59", "09:36:00", Modo::Escolar);
    assert_eq!(iniciados(&acoes), vec![("manha-recreio".to_string(), false, false)]);
}

#[test]
fn nunca_duas_faixas_simultaneas() {
    // Em um dia inteiro, cada Iniciar com algo ativo precisa pedir interrupção.
    let a = agenda_padrao();
    let mut ag = Agendador::new();
    let mut tocando = false;
    for s in parse_hora("06:00:00").unwrap()..=parse_hora("18:00:00").unwrap() {
        let agora = Momento::new(seg(), s);
        for x in ag.tick(&a, agora, Modo::Escolar, false) {
            match x {
                Acao::Iniciar { interromper_atual, .. } => {
                    assert!(!tocando || interromper_atual, "iniciou por cima às {}", fmt_hms(s));
                    tocando = true;
                }
                Acao::EncerrarJanela { modo: FimJanela::FadeOut, .. } => tocando = false,
                _ => {}
            }
        }
        // Simula sinal de 3 min.
        if let Some(at) = &ag.ativo {
            if at.evento.tipo.eh_troca() {
                let ini = at.evento.inicio_seg().unwrap();
                if s >= ini + 180 {
                    ag.faixa_terminou(agora);
                    tocando = false;
                }
            }
        }
    }
}

#[test]
fn selecao_aleatoria_evita_recentes() {
    let c: Vec<String> = (1..=5).map(|i| format!("f{}", i)).collect();
    let rec: Vec<String> = vec!["f1".into(), "f2".into(), "f3".into()];
    let mut rng = Rng::new(42);
    let mut cur = 0;
    for _ in 0..200 {
        let f = escolher_faixa(&c, &rec, 3, ModoSelecao::Aleatorio, None, &mut cur, &mut rng).unwrap();
        assert!(f == "f4" || f == "f5");
    }
    // Lista pequena: sempre há opção.
    let c2: Vec<String> = vec!["a".into(), "b".into()];
    let r2: Vec<String> = vec!["a".into(), "b".into()];
    for _ in 0..50 {
        assert_eq!(escolher_faixa(&c2, &r2, 10, ModoSelecao::Aleatorio, None, &mut cur, &mut rng).as_deref(), Some("b"));
    }
    let mut cur = 0;
    let seq: Vec<String> = (0..7).map(|_| escolher_faixa(&c, &[], 0, ModoSelecao::Sequencial, None, &mut cur, &mut rng).unwrap()).collect();
    assert_eq!(seq, vec!["f1", "f2", "f3", "f4", "f5", "f1", "f2"]);
    assert_eq!(escolher_faixa(&[], &[], 0, ModoSelecao::Aleatorio, None, &mut cur, &mut rng), None);
    assert_eq!(escolher_faixa(&c, &[], 0, ModoSelecao::FaixaFixa, Some("f3"), &mut cur, &mut rng).as_deref(), Some("f3"));
}

#[test]
fn validacao_aponta_problemas_com_sugestao() {
    let mut a = agenda_padrao();
    a.eventos.push(Evento { id: "x1".into(), nome: "Quebrado".into(), inicio: "7h50".into(), ..Evento::default() });
    a.eventos.push(Evento {
        id: "x2".into(),
        nome: "Janela sem fim".into(),
        tipo: TipoEvento::Recreio,
        inicio: "10:00".into(),
        ..Evento::default()
    });
    a.eventos.push(Evento { id: "x3".into(), nome: "Sinal no recreio".into(), inicio: "09:40".into(), ..Evento::default() });
    let av = validar(&a);
    assert!(tem_erro(&av));
    assert!(av.iter().any(|x| x.mensagem.contains("horário inicial inválido") && x.sugestao.is_some()));
    assert!(av.iter().any(|x| x.mensagem.contains("precisa de horário final")));
    assert!(av.iter().any(|x| x.mensagem.contains("Sinal no recreio") && x.mensagem.contains("não tocará")));
}

#[test]
fn interruptor_desliga_evento_so_na_data_indicada() {
    let mut a = crate::padrao::agenda_padrao();
    let segunda = seg();
    let terca = segunda.somar_dias(1);
    let antes_seg = a.eventos_do_dia(segunda).len();
    let antes_ter = a.eventos_do_dia(terca).len();
    a.desligados.push(Desligado { data: segunda.to_string(), evento_id: "manha-sinal-0750".into() });
    // Some só na segunda, que toca normalmente na terça.
    assert_eq!(a.eventos_do_dia(segunda).len(), antes_seg - 1);
    assert_eq!(a.eventos_do_dia(terca).len(), antes_ter);
    // A lista do dia continua mostrando o evento, marcado como desligado.
    let lista = agenda_do_dia(&a, segunda, m(segunda, "06:00"));
    let item = lista.iter().find(|i| i.evento_id == "manha-sinal-0750").expect("evento na lista");
    assert_eq!(item.suprimido_por.as_deref(), Some("desligado neste dia"));
}

#[test]
fn avisos_padrao_das_trocas_e_saidas() {
    let a = crate::padrao::agenda_padrao();
    let troca = a.eventos.iter().find(|e| e.id == "manha-sinal-0750").unwrap();
    assert!(troca.aviso_troca && !troca.aviso_atraso);
    let saida = a.eventos.iter().find(|e| e.id == "tarde-saida-1725").unwrap();
    assert_eq!(saida.tipo, TipoEvento::Saida);
    assert!(saida.tipo.eh_troca() && saida.aviso_troca);
}
