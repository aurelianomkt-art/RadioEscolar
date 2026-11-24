# Roteiro de aceite no notebook da escola

Faça estes testes **no notebook que ficará na escola**, com a caixa Bluetooth real. Marque cada item. Só considere Bluetooth, bateria e instalador “concluídos” depois disso.

Dica: para testar horários sem esperar, crie eventos de teste para daqui a 2 ou 3 minutos em **Programação** (depois, apague-os).

## A. Instalação
- [ ] O instalador `…-setup.exe` instala sem pedir internet (o WebView2 já existe no Windows 10/11 atualizado).
- [ ] O ícone aparece no menu Iniciar e o app abre em português.
- [ ] A versão portátil (.zip) abre a partir de um pendrive, e a pasta `dados\` é criada ao lado do .exe.
- [ ] Tirar o cabo de rede e desligar o Wi-Fi: tudo continua funcionando.

## B. Áudio e Bluetooth
- [ ] Assistente: a caixa Bluetooth aparece na lista de saídas.
- [ ] O som de teste toca **na caixa**, baixo e suave.
- [ ] Desligar a caixa com música tocando: a rádio **pausa**, aparece o alerta vermelho e o som **não** sai pelos alto-falantes do notebook.
- [ ] Religar a caixa, ir em Diagnóstico e escolher a saída: o alerta some e a música continua pausada até alguém tocar Play.
- [ ] Se o nome da saída tiver “Hands-Free”, o Diagnóstico mostra a dica de qualidade.

## C. Programação
- [ ] Sinalização: toca **uma** música completa e para.
- [ ] Recreio de teste (ex.: agora+2 até agora+6): músicas em sequência; no fim, fade-out.
- [ ] Sinal e recreio no mesmo minuto: toca **só** o recreio, e o Histórico registra “suprimido”.
- [ ] Recepção com “deixar a música terminar”: no horário final, termina a faixa atual e não começa outra.
- [ ] Recado agendado com vinheta: vinheta, depois recado, depois (se configurado) música.

## D. Rádio Livre
- [ ] Ao ativar com música tocando, aparece a confirmação.
- [ ] A faixa laranja “Programação automática suspensa” fica visível.
- [ ] Arrastar itens na fila muda a ordem; repetir e aleatório funcionam.
- [ ] Um horário programado que passa durante o Rádio Livre **não** toca (Histórico: “suspenso”).
- [ ] Ao voltar ao Rádio Escolar, nenhum evento vencido toca.

## E. Recados e microfone
- [ ] Recado imediato com música tocando: a música pausa suavemente, toca beep e recado, e a música volta do ponto em que parou.
- [ ] Microfone: abrir só mostra o nível (sem som na caixa); “Falar ao vivo” pausa a música; ao parar, a música volta.
- [ ] Microfone perto da caixa com ganho alto: o app desliga sozinho e alerta “possível microfonia”.

## F. Segurança e confiabilidade
- [ ] “Silenciar agora” corta o som na hora (inclusive microfone) e nada toca até clicar em Retomar.
- [ ] Com PIN definido, salvar a Programação pede o PIN; Silenciar **não** pede.
- [ ] Volume máximo: o controle não passa do teto.
- [ ] Suspender o notebook antes de um horário e acordá-lo depois: o evento perdido **não** toca, e o Histórico registra a reavaliação.
- [ ] Fechar o app pelo ícone da bandeja (Sair) antes de um horário e reabrir depois: nada retroativo toca.
- [ ] Reabrir o app no meio do recreio: retoma a playlist do recreio, sem tocar a sinalização antiga.
- [ ] Mudar a hora do Windows para trás e para frente: nenhum evento toca duas vezes.
- [ ] Renomear um arquivo de música e reindexar: aparece como “Ausente” e é pulado.
- [ ] Bateria: com o carregador desligado, o topo mostra a porcentagem; abaixo do limite, aparece o aviso.
- [ ] Iniciar com o Windows (se ativado): após reiniciar o notebook, o app abre sozinho (na bandeja, se configurado).
- [ ] Abrir o app duas vezes: só existe uma janela/rádio.
- [ ] Interface em 1366×768 e com zoom de 130%: tudo acessível com rolagem.
