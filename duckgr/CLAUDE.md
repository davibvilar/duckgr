# CLAUDE.md

Guia para o Claude Code neste repositório. Leia antes de qualquer tarefa.

## O projeto

DuckGR é uma plataforma aberta de mundos em grade 2D. O primeiro jogo é o **Paz ou Guerra** (regras do Davi). Depois vem um mundo industrial e ecológico inspirado no Factorio, para oficinas de liderança. Tudo está em `docs/DESIGN.md`, que é a fonte de verdade do design. Mudanças de design entram por pull request nesse arquivo.

Líderes: Rodrigo (arquitetura, revisão) e Davi (ideias de jogo, regras, testes). O Davi está aprendendo: explique as decisões em linguagem simples nas descrições de PR.

## Princípios que não se negociam

1. **O núcleo não sabe que existe Terra, fornalha ou guerra.** O motor conhece células, camadas, elementos (protótipo + componentes), comandos e regras. Todo conteúdo e toda regra de jogo vêm de pacotes de dados. A regra "cada um só quebra o que colocou" é configuração do mundo Paz ou Guerra, não código fixo.
2. **Comandos são a única entrada.** Todo efeito no mundo nasce de um comando com autor e tick. Isso dá save, replay, auditoria e testes.
3. **Simulação determinística.** Tick fixo de 30/s, aritmética inteira, sem relógio de parede, sem `HashMap` com ordem aleatória no estado, sem float no estado do mundo. Mesmo log de comandos, mesmo hash de estado.
4. **O núcleo não conhece gráficos, rede nem dinheiro.** Pagamentos, se um dia existirem, só concedem ou retiram permissões, por fora.
5. **Orçamento de desempenho é requisito.** Simulação até 10 ms por tick no mapa de referência, medido no CI.
6. **Regras de recompensa, idade e convivência são do mundo, não da plataforma.** A plataforma oferece mecanismos; o pacote do mundo decide.

## Arquitetura (alvo)

Workspace Rust (nomes provisórios, prefixo `duckgr-`):

- `duckgr-sim`: núcleo da simulação, Rust puro, sem dependências de I/O.
- `duckgr-data`: formato dos pacotes de mundo (RON ou TOML, a decidir) e compilação para tabelas densas.
- `duckgr-protocol`: mensagens motor↔cliente (snapshot de chunk, eventos, movimento por parâmetros, resumos).
- `duckgr-wasm`: o núcleo rodando num Web Worker.
- `web/`: cliente TypeScript + PixiJS.
- Depois: `duckgr-server` (axum/WebSocket), Tauri para desktop.

Não comece pelo que é "depois". Ordem: `duckgr-sim` com testes, depois `duckgr-data`, depois wasm + cliente web.

**Hoje:** o núcleo está em Rust (`crates/duckgr-sim`, com pacote, gerador, simulação, jogador e comandos) e roda no navegador por WebAssembly (`crates/duckgr-wasm`, ponte em `web/platform/core.js`). A interface continua em JavaScript puro em `web/` (mundos em `web/games/`, idiomas em `web/platform/locales/`), com testes em `tests/` (`npm test`, que compila o wasm antes) e publicação no GitHub Pages a cada merge na `main`. Os pacotes ainda são escritos em JavaScript e chegam ao núcleo como JSON; o `duckgr-data` (RON ou TOML) ainda não existe. O estado em JSON e o hash são idênticos aos do protótipo em JavaScript, para que os saves antigos continuem abrindo: qualquer mudança que altere o hash de uma partida existente precisa de versão nova do pacote ou do formato do save.

## Como trabalhar

- Toda mudança entra por pull request, com descrição curta do quê e do porquê. Nada direto na `main`.
- Commits no estilo Conventional Commits (`feat:`, `fix:`, `docs:`, `test:`, `chore:`).
- Antes de abrir PR: `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all` e `npm test`.
- Cada regra de jogo nova vem com teste que reproduz a regra a partir de uma sequência de comandos.
- Teste de determinismo: rodar o mesmo log duas vezes e comparar o hash do estado.
- Mantenha as tarefas pequenas: algo que o Davi consiga testar no fim de uma sessão de duas horas.
- **Código em inglês** (regra do Rodrigo): identificadores, nomes de arquivos e pastas, comentários, nomes de testes e mensagens de diagnóstico. Exceção: formatos já gravados em saves e pacotes (chaves do JSON do estado e do save, campos dos comandos, chaves e ids dos pacotes, chaves do `localStorage`) ficam como estão; mudá-los exige versão nova do formato, com migração.
- **Textos para o usuário internacionalizados** (regra do Rodrigo): nenhum texto de interface fixo no código. Todo texto vai para os arquivos de idioma (`web/platform/locales/pt-BR.js` e `en.js`, no mínimo), com as mesmas chaves em todos; o teste `tests/i18n.test.js` confere. O núcleo não escreve frases: devolve mensagens com chave e argumentos (`crates/duckgr-sim/src/message.rs`), e a interface monta o texto. Textos do conteúdo de um mundo ficam no pacote, no idioma base, com traduções em `web/games/<mundo>/locales/`. O idioma vem do navegador e pode ser trocado em Configurações.
- Documentação, descrições de PR e conversa com a equipe: em português.

## Fora de escopo por enquanto

Multiplayer pela internet, Nostr, WebRTC, mercado em sats, contas, reputação, 3D, mods em roteiro. Estão no design para que o modelo deixe espaço, não para implementar agora.

## Cuidados

- Não publique nada sobre o projeto de pesquisa ISOP neste repositório. Essas partes do design ficam fora até haver permissão por escrito.
- Nunca guarde dados pessoais de jogadores, nem em testes.
