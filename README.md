# Rádio Escolar Mestre João Silvério

Aplicativo desktop para Windows, offline, em português, que substitui o sinal sonoro da escola por músicas programadas, recados e vinhetas, com transições suaves e volume controlado. Feito para funcionários sem conhecimento técnico, num notebook ligado por Bluetooth ao som da escola.

> **Situação desta entrega (leia antes de instalar).**
> - **Testado aqui:** o núcleo do agendador (Rust, 18 testes automatizados passando), a interface (empacotada e percorrida tela a tela num navegador automatizado, sem erros) e o gerador de áudios de demonstração.
> - **Escrito, mas ainda não compilado:** a camada nativa do Windows (Tauri, áudio, SQLite, bandeja, bateria/Wi-Fi). O ambiente onde o código foi escrito não tinha internet para baixar as bibliotecas Rust. Ela é compilada no Windows ou no GitHub Actions (passo a passo abaixo). Se algo não compilar, o erro aparece no log e normalmente se corrige em poucas linhas.
> - **Não declarado como pronto:** Bluetooth, leitura de bateria e instalador só valem como concluídos depois do roteiro de testes em `docs/TESTES-NO-WINDOWS.md`, feito no notebook da escola.

O checklist completo de requisitos (o que está feito, parcial ou pendente) está em [`REQUISITOS.md`](REQUISITOS.md).

---

## 1. Como gerar o instalador (sem instalar nada no seu PC): GitHub Actions

1. Crie um repositório **privado** no GitHub e envie esta pasta inteira.
2. O workflow `.github/workflows/build-windows.yml` roda sozinho a cada envio (ou em *Actions → Build Windows → Run workflow*).
3. Ao terminar (cerca de 15 a 25 min na primeira vez), abra a execução e baixe os **Artifacts**:
   - `instalador-nsis`: `Radio Escolar MJS_1.0.0_x64-setup.exe` (instalador recomendado, sem precisar de administrador)
   - `instalador-msi`: `.msi` (para instalação gerenciada)
   - `versao-portatil`: `.zip` com o `.exe` e o arquivo `portatil.txt` (os dados ficam na pasta `dados\` ao lado do executável)
4. O mesmo workflow roda os testes do agendador antes de compilar. Se algum falhar, nada é gerado.

## 2. Como compilar no próprio Windows

Pré-requisitos (uma vez só, com internet):

1. **Microsoft C++ Build Tools**: instale o *Visual Studio Build Tools 2022* com a carga “Desenvolvimento para desktop com C++”.
2. **Rust**: <https://rustup.rs> (opção padrão `stable-x86_64-pc-windows-msvc`).
3. **Node.js 20 LTS** ou mais novo.
4. **WebView2**: já vem no Windows 10/11 atualizados.

Depois, no PowerShell, dentro da pasta do projeto:

```powershell
npm install
cargo test -p radio-core      # testes do agendador
npm run tauri dev             # abre o app em modo desenvolvimento
npm run tauri build           # gera os instaladores em src-tauri\target\release\bundle\
```

Ou rode `scripts\build-windows.ps1`, que faz tudo isso e ainda monta a versão portátil.

Depois de instalado, o aplicativo **não precisa de internet**: fontes, ícones e scripts vão dentro do instalador.

## 3. Primeiro uso

Na primeira abertura, um assistente guia cinco passos:

1. Escolher a pasta de músicas (ou criar **áudios DEMO** sintetizados para testar).
2. Escolher a saída de áudio (recomendado: a caixa Bluetooth como padrão do Windows).
3. Testar o volume com um som suave.
4. Conferir a agenda do dia.
5. Ativar, se quiser, a inicialização junto com o Windows.

Ao fechar a janela, a rádio continua funcionando na bandeja (perto do relógio). Para sair de verdade, use o ícone da bandeja e clique em **Sair**.

Atalhos: **Alt+S** = Silenciar agora; **Espaço** = tocar/pausar.

## 4. Agenda inicial (editável em Programação)

| Período | Recepção | Sinalizações (1 música cada) | Recreio |
|---|---|---|---|
| Manhã | 06:45–07:05 | 07:50, 08:40, 09:30\*, 10:35, 11:25, 12:15 | 09:30–09:47 |
| Tarde | 12:45–13:05 | 13:50, 14:40, 15:30\*, 16:35, 17:25 | 15:30–15:47 |

\* Às 09:30 e 15:30 o **recreio prevalece**: toca a playlist do recreio, não a sinalização (prioridade 80 contra 20). Há opção de vinheta de abertura no recreio. A aula volta às :45 e a música segue até as :47 (configurável).

Perfis de dia: normal, prova, evento, reunião, feriado, sábado letivo e dia reduzido. Os eventos padrão valem nos perfis “normal”, “prova” e “sábado letivo”. O calendário permite trocar o perfil de uma data ou suspender o dia.

## 5. Regras de comportamento (todas cobertas por testes)

| Situação | Comportamento padrão |
|---|---|
| Troca de professor | Uma música completa. Opcional: limite de duração com fade |
| Recepção e recreio | Músicas em sequência até o horário final, depois **fade-out** (opção: deixar a faixa terminar, com teto de segurança) |
| Dois eventos no mesmo minuto | Vence a maior prioridade; o outro é registrado como suprimido |
| Evento novo com outro ainda tocando | Encerra o anterior com fade e inicia o novo (por evento: aguardar ou ignorar). Prioridade menor nunca interrompe maior |
| Silenciar agora | Corta tudo na hora, inclusive o microfone. Nada toca até alguém clicar em **Retomar** |
| Rádio Livre | Suspende a programação. Os eventos são registrados como “suspensos”, não perdidos |
| Volta ao Rádio Escolar | Não toca eventos vencidos. Se estiver dentro da recepção ou do recreio, retoma a playlist da janela (configurável) |
| PC desligado, suspenso, hibernado ou relógio alterado | Eventos perdidos **não** são tocados depois. A agenda é reavaliada e não repete eventos |
| Queda do Bluetooth | Pausa e alerta. **Nunca** troca sozinho para os alto-falantes do notebook. O operador confirma a saída no Diagnóstico e a música segue pausada até alguém tocar Play |
| Arquivo removido ou corrompido | É pulado e registrado. A biblioteca marca como ausente ou inválido |
| Sem internet | Tudo funciona |

## 6. Decisões técnicas

- **Tauri 2 + React/TypeScript + Rust.** Executável pequeno, usa o WebView2 do Windows, sem navegador nem Microsoft Store.
- **O agendador fica no processo nativo, não na interface.** É uma thread Rust que verifica o relógio local do Windows a cada segundo e continua rodando com a janela minimizada ou escondida na bandeja. A interface só exibe o estado (eventos a cada 250 ms).
- **As regras ficam num crate separado (`crates/radio-core`), sem dependências.** O agendador recebe o horário e devolve ações (iniciar, encerrar com fade, registrar), sem tocar áudio. Por isso todas as regras (prioridade, conflitos, sem retroativo, suspensão, virada de dia, Rádio Livre, silenciar) são testáveis de forma determinística, sem som e sem relógio real.
- **Detecção de suspensão e de mudança de relógio:** se passarem mais de 90 s entre duas verificações, ou se o relógio voltar, o app faz uma *reavaliação*: marca como já passados todos os eventos até agora e só retoma a janela vigente.
- **Áudio:** `rodio` + `cpal` (WASAPI) com decodificação Symphonia de MP3, WAV, OGG/Vorbis e FLAC (licenças MIT, Apache-2.0 e MPL-2.0, compatíveis com uso escolar). Há um canal principal (músicas) e um de anúncio (recados e microfone). Antes do anúncio, a música é pausada ou abaixada: nunca há duas músicas misturadas. Os fades usam curva de cosseno, sem degraus. O volume é limitado por um teto global e por um teto próprio para vinhetas.
- **Dispositivo:** o app segue o padrão do Windows ou um dispositivo escolhido. A cada 2 s verifica se a saída ainda existe e se o Windows não trocou o padrão (sinal típico de queda do Bluetooth). Nesse caso, pausa e alerta.
- **Status do Bluetooth:** o Windows não oferece uma leitura simples e universal de “caixa Bluetooth conectada” para aplicativos. O app mostra o **dispositivo de saída ativo** e se ele está presente. Bateria vem de `GetSystemPowerStatus`. Wi-Fi vem de `netsh wlan show interfaces`. Sem leitura confiável, aparece “indisponível”.
- **SQLite local** (`radio.db` em `%APPDATA%\br.mg.escola.mestrejoaosilverio.radio\`, ou em `dados\` na versão portátil) guarda configuração, biblioteca, playlists, reproduções e o histórico auditável (180 dias).
- **Uma única instância:** abrir o app de novo só mostra a janela que já existe, evitando duas rádios tocando juntas.
- **PIN** com SHA-256 e sal. Protege programação, ajustes, remoção de pastas, importação e logo. **Silenciar nunca pede PIN.**
- **Sem telemetria, contas ou nuvem.**
- **Microfone ao vivo** (opcional, nunca liga sozinho): portão de ruído, limitador suave, ganho inicial baixo e corte automático se o nível ficar saturado por 1,5 s (possível microfonia).
- **Síntese de voz (IA):** não incluída. Fica como módulo futuro opcional; a operação offline nunca depende dela.

## 7. Estrutura

```
crates/radio-core/     regras do agendador + testes (std puro)
src-tauri/src/
  audio.rs             motor de áudio (fades, anúncio, microfone, perda de dispositivo)
  motor.rs             núcleo residente: agendador ⇄ biblioteca ⇄ áudio; laço de 1 s
  comandos.rs          comandos chamados pela interface
  db.rs                SQLite (config, faixas, playlists, histórico)
  biblioteca.rs        indexação de pastas (sem mover arquivos)
  sistema.rs           bateria e Wi-Fi do Windows
  demo.rs              gerador de áudios DEMO sintetizados
  lib.rs               janela, bandeja, inicialização, plugins
src/                   interface React (telas em src/screens)
src/demo.ts            backend FICTÍCIO usado só fora do app (protótipo no navegador)
docs/TESTES-NO-WINDOWS.md   roteiro de aceite no notebook da escola
```

## 8. Testes

```bash
cargo test -p radio-core
```

Os 18 testes cobrem: troca de professor com uma música, recreio prevalecendo às 09:30 e 15:30, playlist do recreio até 09:47 com fade, “terminar faixa” com teto, reinício sem tocar eventos vencidos, reinício dentro do recreio retomando a playlist, suspensão e hibernação, relógio voltando sem repetir evento, Rádio Livre suspendendo e voltando, silenciar até retomada explícita, fim de semana, feriado, sábado letivo e dia suspenso, virada de dia e próximo evento, as três políticas de conflito, nunca duas faixas ao mesmo tempo num dia inteiro, sorteio sem repetição recente e validação com sugestões.

O protótipo navegável (Etapa 1, dados fictícios) pode ser visto abrindo a interface fora do Tauri (`npm run dev` e depois <http://localhost:1420>). Ele mostra a faixa “PROTÓTIPO / DEMONSTRAÇÃO”.
