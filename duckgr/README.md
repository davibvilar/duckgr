# DuckGR

> Pronuncia-se "Duck Gê-Erre".

Uma plataforma aberta para criar e jogar mundos em grade, onde cada pessoa constrói, inventa as regras do próprio mundo e hospeda o próprio servidor. O primeiro jogo é o **Paz ou Guerra**: todo mundo pode colocar qualquer bloco, mas só quem colocou pode quebrar.

O mascote é um pato que vai na frente mostrando o caminho.

## Estado

Versão jogável: **Terra Viva** (nome provisório), a primeira era do mundo industrial e ecológico inspirado no Factorio, só para um jogador. Você é um pato que anda pelo mapa: corte árvores, minere, monte mineradoras e fornalhas, e cumpra a missão sem deixar a floresta cair.

Controles: passe o mouse numa célula para ver as ações possíveis, cada uma com uma tecla (C corta, M minera, R recolhe, A abastece, F fornece, P planta, D desmonta, I vai até lá, 1–9 constrói). Clicar faz a ação destacada; com Shift, a ação entra na fila. E abre o inventário, Esc para as tarefas, Espaço pausa.

Câmera: arraste para mover o mapa e use a roda do mouse (ou + e −) para o zoom; no celular, arraste com um dedo e faça pinça com dois. O botão ◎ (tecla H) volta a seguir o pato, e ⛶ (tecla T) põe o jogo em tela cheia. No celular, tocar numa célula abre uma folha com as ações em botões grandes, e "+ fila" faz o papel do Shift. A cada merge na `main`, o jogo é publicado no GitHub Pages.

O rascunho das regras do Paz ou Guerra continua em [`docs/prototipos/paz-ou-guerra.html`](docs/prototipos/paz-ou-guerra.html).

## Como rodar

Precisa do Rust (com o alvo WebAssembly) e do Node 22.

```sh
rustup target add wasm32-unknown-unknown   # uma vez só
npm test          # compila o núcleo para WebAssembly e roda os testes
npm run servir    # compila e abre um servidor em http://localhost:8000
cargo test        # testes do núcleo em Rust
```

## Como o código está dividido

- `crates/duckgr-sim/`: o núcleo da simulação, em Rust puro. Não sabe que existe árvore, fornalha ou poluição.
  - `pacote.rs`: valida o pacote do mundo e o transforma em tabelas numeradas.
  - `estado.rs`: o estado do mundo (células em camadas, elementos, campos, jogador) e o JSON dele, que é o checkpoint do save e a base do hash.
  - `gerador.rs`: gera o mapa a partir das regras do pacote.
  - `lib.rs`: o tick (elementos feitos de componentes, campos por chunk, crescimento, missão), os comandos e o replay.
  - `jogador.rs`: o jogador como agente (posição, velocidade, alcance), caminho até o alvo, fila de ordens e as ações genéricas (colher, minerar, plantar, construir, recolher, abastecer, fornecer, desmontar).
  - `tests/`: regras reproduzidas a partir de comandos, saves antigos e o orçamento de desempenho.
- `crates/duckgr-wasm/`: o núcleo compilado para WebAssembly, com uma interface mínima em JSON.
- `web/plataforma/`: a plataforma no navegador.
  - `nucleo.js`: a ponte para o núcleo em WebAssembly (`duckgr.wasm`, gerado por `npm run wasm`).
  - `persistencia.js`: o save (checkpoint do estado + histórico de comandos, em JSON com gzip), exportar e importar.
  - `desenho.js`: desenha o mundo num canvas, com a aparência que o pacote define.
  - `app.js`: telas (jogo, menu, configurações, montagem do mundo), laço de ticks e salvamento.
- `web/jogos/`: os mundos. Cada um é um pacote de dados.
  - `terra-viva/pacote.js`: recursos, coberturas, receitas, máquinas, campos, eras, tecnologias e missão.
  - `catalogo.js`: a lista de mundos oferecidos pela plataforma.
- `testes/`: testes em JavaScript que jogam pelo núcleo em WebAssembly. Em `testes/dados/` ficam o pacote exportado para os testes em Rust (`npm run pacotes`) e um save gravado pelo antigo núcleo em JavaScript, que precisa continuar abrindo com o mesmo hash.

O núcleo começou em JavaScript, para fixar o modelo (camadas, componentes, comandos, determinismo), e foi migrado para Rust sem mudar o resultado: o mesmo pacote, a mesma configuração e os mesmos comandos dão exatamente o mesmo mundo e o mesmo hash nos dois. Por isso os saves de antes da migração continuam abrindo.

## Documentos

- [`docs/DESIGN.md`](docs/DESIGN.md): visão, arquitetura, modelo do mundo, roadmap e backlog.
- [`CLAUDE.md`](CLAUDE.md): convenções do repositório para quem desenvolve com o Claude Code.

## Quem lidera

Rodrigo Vilar e Davi Vilar ([@davibvilar](https://github.com/davibvilar)).

## Licença

A definir (ver "Decisões em aberto" no documento de design).
