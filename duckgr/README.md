# DuckGR

> Pronuncia-se "Duck Gê-Erre".

Uma plataforma aberta para criar e jogar mundos em grade, onde cada pessoa constrói, inventa as regras do próprio mundo e hospeda o próprio servidor. O primeiro jogo é o **Paz ou Guerra**: todo mundo pode colocar qualquer bloco, mas só quem colocou pode quebrar.

O mascote é um pato que vai na frente mostrando o caminho.

## Estado

Versão jogável: **Terra Viva** (nome provisório), a primeira era do mundo industrial e ecológico inspirado no Factorio, só para um jogador. Você é um pato que anda pelo mapa: corte árvores, minere, monte mineradoras e fornalhas, e cumpra a missão sem deixar a floresta cair.

Controles: passe o mouse numa célula para ver as ações possíveis, cada uma com uma tecla (C corta, M minera, R recolhe, A abastece, F fornece, P planta, D desmonta, I vai até lá, 1–9 constrói). Clicar faz a ação destacada; com Shift, a ação entra na fila. E abre o inventário, Esc para as tarefas, Espaço pausa.

Idioma: o jogo abre no idioma do navegador (português ou inglês) e dá para trocar em Configurações. Os atalhos de teclado acima são os do português; em inglês as letras mudam (a tela de atalhos mostra todas).

Câmera: arraste para mover o mapa e use a roda do mouse (ou + e −) para o zoom; no celular, arraste com um dedo e faça pinça com dois. O botão ◎ (tecla H) volta a seguir o pato, e ⛶ (tecla T) põe o jogo em tela cheia. No celular, tocar numa célula abre uma folha com as ações em botões grandes, e "+ fila" faz o papel do Shift. A cada merge na `main`, o jogo é publicado no GitHub Pages.

Sem internet: depois da primeira visita, o jogo fica guardado no aparelho (service worker em `web/sw.js`) e abre offline, com novo jogo, salvar e carregar. Ele também pode ser instalado como aplicativo (PWA), no computador ou pelo "Adicionar à tela inicial" do celular. Tudo é servido junto com o jogo, inclusive as fontes (`web/fonts/`).

O rascunho das regras do Paz ou Guerra continua em [`docs/prototipos/paz-ou-guerra.html`](docs/prototipos/paz-ou-guerra.html).

## Como rodar

Precisa do Rust (com o alvo WebAssembly) e do Node 22.

```sh
rustup target add wasm32-unknown-unknown   # uma vez só
npm test          # compila o núcleo para WebAssembly e roda os testes
npm run serve     # compila e abre um servidor em http://localhost:8000
npm run serve:site # monta o site como no Pages (_site/, com cache offline) e serve em :8000
cargo test        # testes do núcleo em Rust
```

## Como o código está dividido

- `crates/duckgr-sim/`: o núcleo da simulação, em Rust puro. Não sabe que existe árvore, fornalha ou poluição.
  - `package.rs`: valida o pacote do mundo e o transforma em tabelas numeradas.
  - `state.rs`: o estado do mundo (células em camadas, elementos, campos, jogador) e o JSON dele, que é o checkpoint do save e a base do hash.
  - `generator.rs`: gera o mapa a partir das regras do pacote.
  - `lib.rs`: o tick (elementos feitos de componentes, campos por chunk, crescimento, missão), os comandos e o replay.
  - `player.rs`: o jogador como agente (posição, velocidade, alcance), caminho até o alvo, fila de ordens e as ações genéricas (colher, minerar, plantar, construir, recolher, abastecer, fornecer, desmontar).
  - `message.rs`: as mensagens para o jogador. O núcleo não escreve frases: diz o que aconteceu com uma chave e argumentos (`{"key": "log.built", "args": {...}}`), e a interface monta o texto no idioma do jogador.
  - `tests/`: regras reproduzidas a partir de comandos, saves antigos e o orçamento de desempenho.
- `crates/duckgr-wasm/`: o núcleo compilado para WebAssembly, com uma interface mínima em JSON.
- `web/platform/`: a plataforma no navegador.
  - `core.js`: a ponte para o núcleo em WebAssembly (`duckgr.wasm`, gerado por `npm run wasm`).
  - `persistence.js`: o save (checkpoint do estado + histórico de comandos, em JSON com gzip), exportar e importar.
  - `render.js`: desenha o mundo num canvas, com a aparência que o pacote define.
  - `i18n.js` e `locales/`: os idiomas. Todo texto da interface fica em `locales/pt-BR.js` e `locales/en.js`, com as mesmas chaves; um teste confere que nenhum idioma ficou sem alguma chave.
  - `app.js`: telas (jogo, menu, configurações, montagem do mundo), laço de ticks e salvamento.
- `web/games/`: os mundos. Cada um é um pacote de dados.
  - `terra-viva/package.js`: recursos, coberturas, receitas, máquinas, campos, eras, tecnologias e missão, com os textos em português.
  - `terra-viva/locales/en.js`: os textos do pacote em inglês, nos mesmos lugares do `package.js`.
  - `catalog.js`: a lista de mundos oferecidos pela plataforma e as traduções de cada um.
- `tests/`: testes em JavaScript que jogam pelo núcleo em WebAssembly. Em `tests/data/` ficam o pacote exportado para os testes em Rust (`npm run packages`) e um save gravado pelo antigo núcleo em JavaScript, que precisa continuar abrindo com o mesmo hash.
- `tools/`: scripts do `npm` (compilar o wasm, exportar os pacotes).

O código (nomes, arquivos, comentários) é escrito em inglês. Ficam em português, por serem formatos já gravados em saves e pacotes, as chaves do JSON do estado e do save, os campos dos comandos e as chaves e ids dos pacotes (`"substratos"`, `"madeira"`...); mudá-los exige uma versão nova do formato.

O núcleo começou em JavaScript, para fixar o modelo (camadas, componentes, comandos, determinismo), e foi migrado para Rust sem mudar o resultado: o mesmo pacote, a mesma configuração e os mesmos comandos dão exatamente o mesmo mundo e o mesmo hash nos dois. Por isso os saves de antes da migração continuam abrindo.

## Documentos

- [`docs/DESIGN.md`](docs/DESIGN.md): visão, arquitetura, modelo do mundo, roadmap e backlog.
- [`CLAUDE.md`](CLAUDE.md): convenções do repositório para quem desenvolve com o Claude Code.

## Quem lidera

Rodrigo Vilar e Davi Vilar ([@davibvilar](https://github.com/davibvilar)).

## Licença

A definir (ver "Decisões em aberto" no documento de design).
