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

Não precisa instalar nada além do Node 22 (só para os testes).

```sh
npm test          # testes do núcleo e das regras da Terra Viva
npm run servir    # abre um servidor em http://localhost:8000
```

## Como o código está dividido

- `web/plataforma/`: a plataforma. Não sabe que existe árvore, fornalha ou poluição.
  - `nucleo.js`: células em camadas, elementos feitos de componentes, campos por chunk, comandos, ticks e hash do estado.
  - `persistencia.js`: o save (checkpoint do estado + histórico de comandos, em JSON com gzip), exportar e importar.
  - `jogador.js`: o jogador como agente (posição, velocidade, alcance), caminho até o alvo, fila de ordens e as ações genéricas (colher, minerar, plantar, construir, recolher, abastecer, fornecer, desmontar).
  - `gerador.js`: gera o mapa a partir das regras do pacote.
  - `desenho.js`: desenha o mundo num canvas, com a aparência que o pacote define.
  - `app.js`: telas (jogo, menu, configurações, montagem do mundo), laço de ticks e salvamento.
- `web/jogos/`: os mundos. Cada um é um pacote de dados.
  - `terra-viva/pacote.js`: recursos, coberturas, receitas, máquinas, campos, eras, tecnologias e missão.
  - `catalogo.js`: a lista de mundos oferecidos pela plataforma.
- `testes/`: testes que reproduzem cada regra a partir de uma sequência de comandos.

Este núcleo em JavaScript é um degrau: ele fixa o modelo (camadas, componentes, comandos, determinismo) que depois vai para o `duckgr-sim` em Rust, rodando em WebAssembly no lugar dele.

## Documentos

- [`docs/DESIGN.md`](docs/DESIGN.md): visão, arquitetura, modelo do mundo, roadmap e backlog.
- [`CLAUDE.md`](CLAUDE.md): convenções do repositório para quem desenvolve com o Claude Code.

## Quem lidera

Rodrigo Vilar e Davi Vilar ([@davibvilar](https://github.com/davibvilar)).

## Licença

A definir (ver "Decisões em aberto" no documento de design).
