# Checklist de requisitos: Rádio Escolar Mestre João Silvério

Legenda:
- ✅ implementado e **testado neste ambiente**
- 🟡 implementado, **precisa ser validado no Windows** (camada nativa ainda não compilada aqui; ver README)
- ⚠️ parcial (explicado na linha)
- ⏳ fora desta versão (explicado na linha)

Prioridade: **P1** crítico para o uso diário · **P2** importante · **P3** desejável.

## Plataforma e entrega
| P | Requisito | Status | Onde |
|---|---|---|---|
| P1 | App desktop Windows, PT-BR, offline, sem navegador nem Store | 🟡 | Tauri 2 (`src-tauri`) |
| P1 | Instalador .exe/.msi | 🟡 | `tauri.conf.json` (NSIS + MSI) e CI `.github/workflows/build-windows.yml` |
| P2 | Versão portátil | 🟡 | `portatil.txt` ao lado do .exe → dados em `dados\` (CI gera o .zip) |
| P1 | Código-fonte, instruções de build, testes | ✅ | README, `crates/radio-core/src/testes.rs` |
| P1 | Sem dependência de internet (fontes, ícones, scripts) | ✅ | fontes do sistema, ícones empacotados, CSP local |
| P2 | Iniciar com o Windows, bandeja, ícone de status | 🟡 | plugin autostart, `criar_bandeja`, dica da bandeja atualizada |
| P1 | Sem telemetria, contas ou nuvem | ✅ | não há rede no código |

## Identidade e UX
| P | Requisito | Status | Onde |
|---|---|---|---|
| P2 | Fundo branco, linhas finas, cantos discretos, azul-marinho/azul-vivo, laranja/amarelo para ações | ✅ | `src/styles.css` |
| P2 | Logo da rádio (sem fundo) e troca da logo por arquivo local | ✅ / 🟡 | `public/emblema.png`; Configurações → Trocar logo |
| P1 | Botões grandes e “Voltar” nas telas secundárias | ✅ | `Cabecalho` em `src/ui.tsx` |
| P1 | Adaptável a 1366×768, zoom e escala de fonte, alto contraste | ✅ | testado em 1366×768; Configurações → Aparência |
| P2 | Uso pelo teclado, rótulos claros | ✅ | foco visível, aria-labels, Alt+S, Espaço, ↑/↓ na fila |

## Tela principal
| P | Requisito | Status | Onde |
|---|---|---|---|
| P1 | Logo/nome, relógio grande, data | ✅ | `App.tsx` |
| P1 | Estado da saída de áudio (Bluetooth/dispositivo), Wi-Fi, bateria com dados reais e mensagens de falha | 🟡 | `sistema.rs`, `audio.rs` (“indisponível” sem leitura confiável) |
| P1 | Botões “Rádio Escolar” (azul) e “Rádio Livre” (laranja) com “MODO ATIVO” | ✅ | barra de modos |
| P1 | Música atual, artista, progresso, tempo decorrido/restante, play/pause, anterior, próxima, volume | ✅ / 🟡 | `Player.tsx` |
| P3 | Capa opcional | 🟡 | capa embutida no arquivo (ID3/FLAC/Vorbis); sem capa mostra ícone |
| P3 | Forma de onda ou visualizador de níveis | ✅ / 🟡 | visualizador de níveis (pico medido no motor de áudio) |
| P1 | “Silenciar agora” sempre visível | ✅ | topo, bandeja e Alt+S |
| P1 | Rodapé: próximo evento, contagem regressiva, status do agendador, navegação | ✅ | `App.tsx` |
| P1 | Avisos de configuração ausente | ✅ | `painel().avisos_config` |

## Rádio Escolar
| P | Requisito | Status | Onde |
|---|---|---|---|
| P1 | Agenda inicial manhã/tarde (todos os horários do pedido) | ✅ | `padrao.rs`; teste `agenda_padrao_valida…` |
| P1 | 09:30/15:30: recreio prevalece, sem duas faixas; vinheta de abertura opcional | ✅ | teste `recreio_prevalece…` |
| P1 | Parar a recepção às 07:05/13:05 conforme configurado | ✅ | `FimJanela` + teste `recepcao_terminar_faixa_com_teto` |
| P1 | 13:50 como sinalização solicitada | ✅ | evento `tarde-sinal-1350` |
| P1 | Trocas: UMA música completa, sem sobreposição; limite/trecho opcional | ✅ / 🟡 | teste `troca_de_professor_toca_uma_vez`; limite no `audio.rs` |
| P1 | Recepção/recreio: sequência até o fim; finalizar faixa ou fade (padrão: fade) | ✅ | testes de recreio e recepção |
| P1 | Política de prioridade sem mistura acidental | ✅ | teste `nunca_duas_faixas_simultaneas` |
| P2 | Aleatório sem repetição recente ou música/playlist escolhida | ✅ | `selecao.rs` + teste |
| P2 | Volumes por tipo de evento, fade-in/out | ✅ / 🟡 | evento e Configurações |

## Rádio Livre
| P | Requisito | Status | Onde |
|---|---|---|---|
| P1 | Fila visual reordenável por arrastar e soltar (e por teclado) | ✅ | `FilaResumo.tsx` |
| P2 | Playlists, vinhetas, recados, tocar/pausar/pular, repetição, aleatório, volume | ✅ / 🟡 | `RadioLivre.tsx`, `motor.rs` |
| P1 | Suspende os eventos sem perdê-los + aviso persistente | ✅ | teste `radio_livre_suspende…`; faixa laranja |
| P1 | Ao voltar, sem retroativo; retoma só o que ainda vale | ✅ | `reavaliar` + teste |
| P1 | Confirmação ao trocar de modo com áudio tocando | ✅ | `definir_modo` → “CONFIRMAR” |
| P2 | Rádio Livre por tempo indeterminado (persiste após reiniciar) | 🟡 | modo salvo no banco |

## Biblioteca
| P | Requisito | Status | Onde |
|---|---|---|---|
| P1 | “Adicionar pasta” com seletor nativo, subpastas configuráveis | 🟡 | plugin dialog, `biblioteca.rs` |
| P2 | Nome, duração, formato, localização, metadados | 🟡 | lofty |
| P1 | Categorias Músicas/Vinhetas/Recados/Playlists sem mover os originais | ✅ / 🟡 | só leitura dos arquivos |
| P1 | Detectar removidos, inválidos ou inacessíveis; pular com registro | 🟡 | status `ausente`/`invalido`; erro de áudio marca e pula |
| P2 | Várias pastas, reindexação manual e automática, busca e filtro | 🟡 | `reindexar`, laço de reindexação |
| P2 | Histórico de reprodução para evitar repetição | 🟡 | tabela `reproducoes` |

## Recados e vinhetas
| P | Requisito | Status | Onde |
|---|---|---|---|
| P1 | Cadastro (nome, categoria, duração), agendamento ou disparo manual | ⚠️ / 🟡 | o nome exibido é o do arquivo (renomeie o arquivo para mudar); categoria e volume editáveis; agendar e tocar na tela Recados |
| P1 | Beep/vinheta suave antes do recado (substituível) | 🟡 | beep sintetizado interno ou vinheta escolhida |
| P2 | Sequência vinheta → recado → música | 🟡 | `iniciar_evento` |
| P2 | Volume individual | 🟡 | controle por arquivo + teto de vinheta |
| P1 | Recado imediato pausa/abaixa suavemente e retoma | 🟡 | canal de anúncio (`audio.rs`) |
| P3 | Microfone ao vivo opcional (dispositivo, nível, anti-microfonia, nunca automático) | 🟡 | `mic_*`; corte por saturação |
| P3 | Síntese de voz por IA | ⏳ | módulo futuro opcional, conforme o pedido |

## Programação e calendário
| P | Requisito | Status | Onde |
|---|---|---|---|
| P1 | Editor por dia da semana, tipo, horários, playlist/pasta, seleção, duração, volume, vinheta, recado | ✅ | `Programacao.tsx` (testado no protótipo) |
| P2 | Perfis: normal, prova, evento, reunião, feriado, sábado letivo, reduzido | ✅ | teste `fim_de_semana_feriado…` |
| P1 | Calendário com exceções e suspensão do dia | ✅ | idem |
| P1 | Agenda do dia e próxima execução | ✅ | `agenda_do_dia`, `proximos` |
| P1 | Validação de conflitos com sugestões antes de salvar | ✅ | `validacao.rs` + teste |
| P2 | Backup/exportação/importação local | 🟡 | `exportar_config` / `importar_config` (o PIN não é exportado) |

## Segurança, confiabilidade e acessibilidade
| P | Requisito | Status | Onde |
|---|---|---|---|
| P1 | PIN para configurações críticas; nunca para silenciar | 🟡 | `exigir_pin` |
| P1 | Silenciar = corte imediato; retomada explícita | ✅ / 🟡 | teste `silenciar_bloqueia…`; `Cmd::Silenciar` |
| P1 | Volume máximo e limite para vinhetas | 🟡 | `volume_maximo`, `limite_vinheta` |
| P1 | Fade suave, sem picos | 🟡 | rampa em cosseno |
| P1 | Não afirmar que as músicas são universalmente adequadas a pessoas autistas | ✅ | texto em Configurações |
| P1 | Alertar perda de saída; nunca redirecionar sem confirmação | 🟡 | `verificar_dispositivo` + Diagnóstico |
| P2 | Alertar bateria baixa e falta de pasta | 🟡 | `avisos_config`, histórico |
| P1 | Suspensão/hibernação/relógio/reinício: reavaliar sem retroativo | ✅ | testes de reinício, suspensão e relógio |
| P1 | Histórico local auditável (execução, falha, ação manual) | 🟡 | tabela `historico`, exportação CSV |

## Telas
Início/Painel ✅ · Rádio Escolar ✅ · Rádio Livre ✅ · Programação e Calendário ✅ · Biblioteca ✅ · Recados/Vinhetas ✅ · Histórico ✅ · Configurações ✅ · Diagnóstico do áudio ✅ · Assistente de primeira configuração ✅ (todas navegáveis no protótipo; as funções reais dependem da validação no Windows 🟡)

## Pendências conhecidas
1. **Compilar a camada nativa no Windows/CI** e corrigir eventuais erros de compilação (o código foi escrito sem acesso às bibliotecas).
2. **Roteiro de aceite no notebook da escola** (`docs/TESTES-NO-WINDOWS.md`): Bluetooth, bateria, instalador, suspensão real e início com o Windows.
3. Campos de horário usam o seletor nativo do sistema. Em Windows configurado em português, aparecem em 24 h.
4. Síntese de voz por IA: futura e opcional.
