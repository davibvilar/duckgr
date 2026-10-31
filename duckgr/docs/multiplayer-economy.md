# Multiplayer, economia e Lightning: proposta de design

Rodrigo Vilar · rascunho de 31 de outubro de 2026 · proposta, ainda não implementada

Este documento responde a uma pergunta do Rodrigo: começar pelo multiplayer ou pela integração com carteiras Lightning? A resposta curta é **multiplayer primeiro**. O objetivo do projeto é testar teoria dos jogos, e teoria dos jogos precisa de pelo menos dois agentes cujas escolhas afetam um ao outro. Num jogo solo, uma carteira só serviria para vender o jogo, que é um problema pequeno e separado.

O documento cobre quatro partes: (1) como os jogadores se encontram e como a partida trafega, (2) o modelo econômico com criadores, mentores e jogadores, (3) as opções de carteira Lightning e (4) um roteiro em passos pequenos, com o primeiro passo recomendado.

Ele parte do que já está decidido no `docs/DESIGN.md` (Nostr só em volta da partida, modelo Mastodon, núcleo que não conhece dinheiro) e propõe revisar uma decisão: a do multiplayer "autoritativo, não lockstep". A seção 1.4 explica por quê.

## 0. O que o código permite hoje

Conferido no `crates/duckgr-sim`:

- O núcleo é determinístico e o save já é checkpoint + histórico de comandos com tick (`advance`, `replay`, `State::hash`). Refazer duas horas de jogo leva cerca de 2 s.
- O estado tem **um** jogador (`State.player`) e **um** inventário (`State.inv`). Os comandos (`ordem`, `cancelar`, `fabricar`, `pesquisar`) não usam o autor. Entidades não guardam dono.
- O núcleo roda inteiro no navegador por WebAssembly. Não existe `duckgr-server`.

Consequência: antes de qualquer rede, o estado precisa conhecer vários jogadores, cada um com o seu inventário, e cada entidade precisa de um dono. Sem isso não há troca, disputa nem "cada um quebra o que colocou", e portanto nada de teoria dos jogos para testar. Esse é o primeiro passo recomendado (seção 4).

## 1. Encontro e transporte da partida

### 1.1 Duas camadas

| Camada | O que passa | Meio |
| --- | --- | --- |
| Encontro | Identidade, anúncio de sala, pedido para entrar, aceite, sinalização WebRTC, chat do lobby | Relays Nostr |
| Partida | Comandos com tick, confirmações, hash de verificação, checkpoint para quem entra | Canal direto WebRTC (DataChannel), com plano B por WebSocket |

Relays Nostr não servem para o tick: a latência típica fica entre centenas de milissegundos e segundos, há limites de taxa, e tudo o que vai para um relay pode ser guardado e lido por terceiros. Já para o encontro eles servem bem, sem servidor nosso.

### 1.2 Encontro pelo Nostr

**Identidade.** Cada pessoa é uma chave pública. Quem tem extensão (Alby, nos2x) assina pelo NIP-07; no celular, um assinador remoto pelo NIP-46; quem não tem nada recebe uma chave gerada na hora, guardada no navegador. Identidade continua opcional, como no `DESIGN.md`.

**Tipos de evento (provisórios).** Os números de `kind` abaixo são reservas nossas até virarem uma proposta de NIP, se o modelo der certo.

| Evento | Kind | Faixa Nostr | Conteúdo |
| --- | --- | --- | --- |
| Anúncio de sala | 31950 (provisório) | Endereçável (substituível por `d`) com `expiration` (NIP-40) | Mundo e hash do pacote, versão do formato, vagas, modo (aberto, convite, oficina), idioma, relays de sinalização |
| Presença no lobby | 21950 (provisório) | Efêmero | "estou aqui", a cada 30 s |
| Pedido para entrar e aceite | 21951 (provisório) | Efêmero, cifrado com NIP-44 para o host | Chave do jogador, apelido, versão do cliente |
| Sinalização WebRTC | 21952 (provisório) | Efêmero, cifrado com NIP-44 | Oferta, resposta e candidatos ICE |

**Três jeitos de entrar numa sala:**

1. **Link ou QR de convite.** Leva a chave do host, a lista de relays e um segredo curto da sala. É o caso da oficina e dos amigos, e o primeiro a implementar.
2. **Lista pública.** O cliente busca anúncios de sala do mesmo mundo e da mesma versão nos relays configurados.
3. **Pareamento automático (depois).** Um bot de matchmaking, que é só mais uma chave Nostr, junta pedidos parecidos (mundo, idioma, tamanho) e cria a sala. Qualquer pessoa pode rodar um, coerente com o modelo Mastodon.

**Relays.** O projeto não mantém relay (decisão do `DESIGN.md`). O cliente vem com uma lista editável de relays públicos, e o pacote de hospedagem pode trazer um relay local para oficinas sem internet.

**STUN e TURN.** STUN público por padrão; TURN da comunidade. Redes de empresa e de universidade às vezes bloqueiam conexão direta, e aí entra o plano B: um retransmissor WebSocket simples, que só repassa bytes.

### 1.3 Quatro opções de transporte

| Opção | Como funciona | A favor | Contra |
| --- | --- | --- | --- |
| A. Jogar pelos relays Nostr | Cada comando vira um evento | Zero infraestrutura | Latência alta, limites de taxa, partida pública. Descartada |
| B. Lockstep P2P em malha | Todos falam com todos; cada tick espera os comandos de todos | Sem host; banda mínima | Um jogador lento ou uma aba em segundo plano para todo mundo; N² conexões; difícil passar de 3 jogadores |
| C. Host sequenciador, replicação determinística | Estrela: o navegador de um jogador é o host. Ele recebe os comandos, dá a cada um o tick em que vale e a ordem, e repassa a todos. Cada cliente roda o mesmo núcleo e chega ao mesmo estado | Usa o que já existe (núcleo no navegador, log de comandos, hash). Banda de poucos bytes por comando. Ninguém espera o mais lento. Entrar no meio é mandar o checkpoint + comandos. Sem servidor para manter | Todos veem o estado inteiro (sem informação escondida). O host precisa ficar com a aba visível. Mapa grande custa CPU em todos |
| D. Servidor autoritativo com estado | `duckgr-server` em Rust nativo simula; clientes recebem diffs só da área que veem (o plano do `DESIGN.md`) | Esconde informação, escala para mapas e grupos grandes, roda numa oficina sem internet | Precisa construir o servidor e o protocolo de diffs; alguém precisa hospedar |

### 1.4 Recomendação: C agora, D quando precisar

Recomendo a opção **C**, com a mesma interface de transporte que a D usaria depois.

Isso revisa a decisão "multiplayer autoritativo, não lockstep" do `DESIGN.md`, mas preserva o motivo dela. O medo era o lockstep estrito, em que o cliente mais lento segura todo mundo. Na opção C o host continua sendo a autoridade (decide a ordem e o tick de cada comando), e ninguém espera ninguém: um cliente atrasado só fica para trás e se recupera em alta velocidade, porque refazer comandos é rápido. A diferença para o plano original é que o host manda comandos em vez de diffs de estado, o que hoje é muito mais simples e mais barato.

Como a partida corre na opção C:

1. O cliente assina o comando com a sua chave e o envia ao host, com o tick em que o jogador o deu.
2. O host confere a assinatura e a permissão, marca o comando para o tick `atual + atraso` (atraso de entrada de uns 3 a 5 ticks, 100 a 170 ms) e o põe na fila.
3. A cada janela de 3 ticks (100 ms) o host envia a todos um lote: "comandos dos ticks T a T+2, em ordem; podem avançar até T+2". Lote vazio também é enviado: é ele que marca o ritmo.
4. Cada cliente avança o seu núcleo até o tick liberado. A tela mostra uma prévia local da ação do jogador enquanto a confirmação não chega.
5. A cada 5 s, cada cliente manda o hash do estado. Hash diferente do host é dessincronização: o cliente recebe o checkpoint do host e refaz os comandos desde ele, e o caso fica registrado para depuração.
6. Quem entra no meio recebe o último checkpoint e os comandos posteriores, exatamente como ao abrir um save.
7. Se o host cai, todos têm o estado inteiro: o jogador seguinte da lista combinada no início vira host e a partida continua do último tick confirmado.

**Riscos a medir no primeiro protótipo:**

- **Aba do host em segundo plano.** Navegadores reduzem a frequência de timers em abas escondidas. O relógio do host deve rodar no Web Worker, e o protótipo precisa medir se isso basta; se não bastar, o cliente pede para manter a aba visível ou passa a vez de host.
- **Informação escondida.** Na opção C todos têm o estado inteiro, então um cliente modificado vê tudo. Para experimentos que dependem de informação privada (lances fechados, inventário secreto) há duas saídas: compromisso e revelação (o jogador publica o hash do lance e só depois o lance), ou passar para a opção D.
- **Trapaça do host.** O host pode recusar ou atrasar comandos de alguém, mas não pode forjá-los, porque cada comando é assinado. O log assinado permite auditar a partida depois.

**Quando ir para a D.** Mapas de missão grandes (256×256), grupos de mais de 8 pessoas, experimentos com informação privada de verdade, ou oficinas em que o facilitador quer um servidor no próprio notebook. O protocolo de comandos assinados é o mesmo; muda quem simula e o que volta para o cliente.

### 1.5 O que muda no núcleo

- `State.players`: lista ordenada de jogadores (por chave pública ou por índice), cada um com posição, fila de ordens e inventário próprio. O `State.inv` global vira o inventário do jogador 0 na migração.
- Todo comando ganha `autor`. O núcleo usa o autor para saber de quem é a fila, o inventário e a permissão; nunca para assinatura, que fica fora do núcleo.
- Toda entidade ganha `dono`. As regras de quem pode quebrar o quê vêm do pacote do mundo, como já previsto para o Paz ou Guerra.
- Ordem de aplicação dentro do mesmo tick: a ordem do log, decidida pelo host. Já é assim no `advance`.
- Formato do save versão 2, com migração do save versão 1 (um jogador). O hash de partidas antigas continua igual depois da migração.

## 2. Modelo econômico

A economia tem três relações, como o Rodrigo descreveu, mais a relação de cada um com o mundo.

| Relação | Quem paga quem, por quê | Precisa de multiplayer? | Exemplos |
| --- | --- | --- | --- |
| Criador ↔ jogador | O jogador paga o criador por um mundo, um pacote de conteúdo, uma missão ou um mod | Não | Comprar um mundo novo; apoiar o autor com um zap; comprar uma missão para a oficina |
| Mentor ↔ jogador | O jogador (ou a empresa dele) paga o mentor pelo tempo: conduzir uma partida, observar e fazer o debriefing | Sim, o mentor está na partida | Sessão de 45 minutos; mentor de plantão num servidor aberto |
| Jogador ↔ jogador | Troca, contrato, serviço, aluguel, recompensa | Sim | Vender ferro por comida; pagar alguém para limpar a poluição; alugar um bot trabalhador |
| Todos ↔ o mundo | Bens comuns: poluição, floresta, jazidas finitas, água | Sim | O rio poluído por um prejudica todos a jusante |

### 2.1 O que a teoria dos jogos pode testar aqui

O jogo já tem os ingredientes de vários jogos clássicos. Cada experimento abaixo usa mecânicas que existem ou estão no design, e se mede pelo log de comandos com autor.

| Jogo clássico | Onde aparece no DuckGR | O que medir |
| --- | --- | --- |
| Bens públicos | A poluição é um campo compartilhado; cada um ganha produzindo e todos pagam a poluição. Reflorestar custa a quem planta e beneficia todos | Quanto cada jogador investe em ecologia versus produção; como isso muda com punição, com reputação, com comunicação |
| Tragédia dos comuns | Jazidas e nascentes finitas, compartilhadas | Velocidade de esgotamento com e sem regras de cota |
| Dilema do prisioneiro repetido | Paz ou Guerra: quebrar o bloco do outro inicia guerra | Taxa de cooperação ao longo da partida; estratégias do tipo olho por olho |
| Jogo da confiança | Troca com entrega em dois tempos: um paga, o outro entrega depois | Quanto se confia num desconhecido; efeito de uma reputação visível |
| Ultimato e barganha | Ofertas de troca que o outro aceita ou recusa | Divisões propostas e recusadas |
| Leilões | Direito de explorar uma jazida rica | Lances, maldição do vencedor, conluio |
| Coordenação | Escolher a mesma bitola, o mesmo padrão de estrada, a mesma era de pesquisa | Tempo até convergir |

Dois cuidados: experimento com pessoas reais pede consentimento e dados anônimos (LGPD, já no `DESIGN.md`), e o log de experimento nunca vai para relay público.

### 2.2 Duas moedas, duas fases

1. **Moeda do jogo (sem dinheiro real).** A moeda é só mais um item do pacote do mundo, e trocar é um comando (`oferta`, `aceitar`, `transferir`). Fica dentro do núcleo, determinística, e não quebra a regra "o núcleo não conhece dinheiro", porque não é dinheiro: é conteúdo. Dá para rodar todos os experimentos da tabela acima assim, de graça e sem risco regulatório.
2. **Sats (dinheiro real).** Só por fora do núcleo, e só para conceder ou retirar permissões (regra do `CLAUDE.md`). Exemplos: um pagamento libera um pacote; um pagamento por período mantém a permissão de um mentor ou de um bot. Um experimento de teoria dos jogos com dinheiro real de verdade (pagar aos jogadores conforme o desempenho, como se faz em economia experimental) é uma recompensa paga pelo organizador ao fim da sessão, fora da simulação.

**O que não fazer com sats, por enquanto:** apostas entre jogadores. No Brasil, apostas com dinheiro têm regulação própria (Lei 14.790/2023), e guardar o dinheiro de terceiros como garantia nos tornaria custodiantes. Fica para depois de uma consulta jurídica, junto com os pontos fiscais já listados no `DESIGN.md`.

### 2.3 Regras que já valem

- Em sessão de curso, terapia ou servidor competitivo, ninguém compra vantagem. Cada servidor declara se aceita serviços pagos.
- O projeto nunca guarda dinheiro de ninguém. Todo pagamento vai de carteira para carteira.
- Pagar sempre dá permissão ou conteúdo; nunca altera a regra do mundo para quem pagou.

## 3. Integração Lightning

### 3.1 As peças

| Peça | O que é | A favor | Contra |
| --- | --- | --- | --- |
| Nostr Wallet Connect (NWC, NIP-47) | A carteira do usuário dá ao app uma conexão (`nostr+walletconnect://...`) com permissões e orçamento; o app pede pagamentos por eventos Nostr cifrados | Funciona no navegador, no celular e sem extensão; orçamento por app (por exemplo 5.000 sats por mês); várias carteiras suportam; combina com o Nostr que já usamos | O segredo da conexão fica no navegador do usuário; latência de alguns segundos por pagamento |
| Alby | Duas coisas: a extensão de navegador (WebLN e NIP-07) e o Alby Hub, um nó Lightning próprio que fala NWC | Extensão resolve identidade e pagamento juntos; Hub é de custódia própria | Extensão só no desktop; Hub exige rodar ou contratar um nó |
| Coinos | Carteira web, de custódia, com endereço Lightning e NWC | Muito fácil para quem nunca usou Bitcoin; boa para teste | Custodial: a Coinos guarda os sats |
| WebLN | API do navegador (`window.webln`) oferecida por extensões | Simples para pagar com um clique | Depende de extensão; pouco suporte no celular |
| Zaps (NIP-57) | Pagamento Lightning ligado a um evento ou pessoa Nostr; a carteira de quem recebe publica um recibo assinado (kind 9735) | O recibo é público e verificável sem servidor nosso; ideal para pagar criadores | Público por natureza: todo mundo vê quem pagou o quê; confia no servidor LNURL de quem recebe |
| Endereço Lightning (LNURL-pay) | `nome@dominio` para receber | O criador recebe sem rodar nada | Só recebe; não confirma pagamentos sozinho |
| Bitcoin Connect (Alby) | Biblioteca de componentes web que mostra um botão "conectar carteira" e cobre NWC, WebLN e outras | Uma integração só para várias carteiras; código aberto | Mais uma dependência na interface |
| Cashu (ecash) | Fichas digitais emitidas por uma "casa da moeda", trocáveis offline | Pagamentos minúsculos e frequentes, privados, instantâneos | A casa da moeda guarda os sats; ecossistema mais novo |
| Faturas retidas (hold invoices) | O pagamento fica preso até quem recebe liberar ou cancelar | Garantia sem custódia, útil para contratos e entregas em dois tempos | Suporte por NWC existe só em parte das implementações; a verificar |

### 3.2 Recomendação

- **Um caminho só na interface:** Bitcoin Connect, que entrega NWC no celular e WebLN no desktop. Testar primeiro com Coinos (mais fácil) e Alby (extensão e Hub).
- **Criador ↔ jogador por zap.** O pacote de um mundo já é (ou será) um evento Nostr assinado pelo criador. Comprar é dar um zap nesse evento; o recibo de zap é a licença, e qualquer cliente confere o recibo nos relays, sem servidor nosso. Como o código e os pacotes são abertos, a compra não impede cópia: ela é apoio, acesso antecipado ou um selo de "comprado". Isso é coerente com projeto aberto e evita DRM.
- **Mentor ↔ jogador por período.** O jogador autoriza pelo NWC um orçamento para a sessão; o cliente paga o endereço Lightning do mentor a cada 5 minutos enquanto ele está na partida. Se o pagamento para, a permissão de mentor cai (o padrão de serviços recorrentes do `DESIGN.md`). Calote máximo: um período.
- **Jogador ↔ jogador.** Primeiro só moeda do jogo. Sats depois, em servidores que declaram aceitar, começando por gorjetas e recompensas (zap para quem ajudou), sem garantias nem apostas.

## 4. Roteiro em passos pequenos

Cada passo cabe numa sessão de duas horas e termina com algo que o Davi consegue testar.

| # | Passo | Como o Davi testa |
| --- | --- | --- |
| 1 | **Vários jogadores no núcleo, sem rede.** `State.players`, inventário por jogador, `autor` em todo comando, `dono` em toda entidade, save versão 2 com migração. Na interface, troca de jogador ativo na mesma tela (como o rascunho do Paz ou Guerra) | Dois patos na mesma tela, cada um com o seu inventário; o save antigo abre igual |
| 2 | **Host sequenciador entre duas abas, sem rede.** Interface de transporte e o laço do host (lotes, tick liberado, hash), ligados por `BroadcastChannel` | Abrir duas abas no mesmo computador e jogar junto |
| 3 | **WebRTC com convite manual.** O mesmo transporte por DataChannel; a sinalização é um código copiado e colado (ou QR) | Jogar com alguém em outra casa trocando um código pelo WhatsApp |
| 4 | **Encontro pelo Nostr.** Chave (NIP-07 ou gerada), sinalização cifrada pelos relays, link de convite | Mandar um link; o outro clica e entra |
| 5 | **Ressincronia e entrada no meio.** Checkpoint para quem entra, hash periódico, troca de host | Fechar e reabrir uma aba no meio da partida |
| 6 | **Moeda e troca.** Moeda como item do pacote; comandos de oferta e aceite | Vender ferro ao outro jogador |
| 7 | **Primeiro experimento: bens públicos.** Missão com poluição compartilhada; painel de quanto cada um investiu em ecologia, a partir do log | Jogar a missão e ver o painel no fim |
| 8 | **Lobby público.** Anúncio de sala e lista de salas abertas | Achar uma sala sem receber link |
| 9 | **Carteira.** Bitcoin Connect; zap no criador de um mundo, sem desbloquear nada | Dar 21 sats de zap ao autor da Terra Viva |
| 10 | **Conteúdo pago.** Recibo de zap libera um pacote | Comprar uma missão e vê-la aparecer |
| 11 | **Mentor pago por período.** Papel de mentor na partida e pagamento a cada 5 minutos pelo NWC | Um mentor entra, o pagamento corre, e para quando ele sai |

**O primeiro passo é o 1.** Tudo o que vem depois (rede, troca, experimentos, pagamentos) depende de o mundo saber que há mais de uma pessoa nele e de quem é cada coisa. Ele não precisa de rede nem de carteira, é testável na mesma tela e mexe só no núcleo e no save, que é onde o projeto já tem testes de determinismo.

A venda de um jogo ou pacote (passos 9 e 10) não depende do multiplayer e pode subir na fila se houver motivo comercial, mas não ajuda a testar teoria dos jogos.

## 5. Decisões em aberto

- [ ] **Inventário por jogador ou do time?** A proposta é por jogador, com a opção de o pacote do mundo declarar um baú comum. Teoria dos jogos precisa de propriedade individual; oficinas de liderança talvez prefiram o time.
- [ ] **Quem pode ser host:** qualquer jogador, ou só quem criou a sala?
- [ ] **Quantos jogadores no primeiro alvo:** 2 a 4 é o recomendado para a opção C.
- [ ] **Números de `kind` Nostr:** manter provisórios até o passo 8 e, se funcionar, propor um NIP.
- [ ] **Consulta jurídica antes do passo 9:** pagamentos em Bitcoin, nota fiscal de venda de conteúdo, apostas.

## Fontes

- [NIP-01, NIP-07, NIP-40, NIP-44, NIP-46, NIP-47 e NIP-57](https://github.com/nostr-protocol/nips)
- [Bitcoin Connect (Alby)](https://github.com/getalby/bitcoin-connect)
- [Nostr Wallet Connect: como funciona](https://www.spark.money/research/bitcoin-nostr-wallet-connect-remote-signing)
- [vmsplice: Nostr para apps peer-to-peer](https://blog.vmsplice.net/2023/09/how-nostr-could-enable-peer-to-peer-apps.html)
- [Friday Facts #188: lockstep determinístico](https://factorio.com/blog/post/fff-188)
- [Deterministic Lockstep in Networked Games](https://hdms.bsz-bw.de/files/7107/DeterministicLockstepInNetworkedGamesPaper.pdf)
