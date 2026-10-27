# Primeira sessão no Claude Code

Cole o texto abaixo como primeira mensagem na sessão.

---

Leia o `CLAUDE.md` e o `docs/DESIGN.md` inteiros antes de começar, e abra o protótipo `docs/prototipos/paz-ou-guerra.html` para ver as regras do Paz ou Guerra funcionando.

Hoje é a semana 1: arrumar a casa. Faça, num pull request só:

1. `CONTRIBUTING.md` curto: como abrir issue, como abrir PR, os comandos de checagem do `CLAUDE.md`, Conventional Commits e DCO (`git commit -s`).
2. `CODE_OF_CONDUCT.md` baseado no Contributor Covenant 2.1, em português.
3. `GOVERNANCE.md`: Rodrigo e Davi são os mantenedores; decisões de design passam por PR no `docs/DESIGN.md`; decisões grandes viram uma RFC em `docs/rfcs/`.
4. Templates em `.github/`: issue de bug, issue de ideia de jogo (pensado para o Davi preencher) e pull request.
5. `.gitignore` para Rust e Node.
6. Não escolha a licença: deixe um `LICENSE` pendente e abra uma issue "Escolher a licença" com as opções do design (MIT/Apache-2.0 ou AGPL; arte em CC BY-SA).

Depois, abra issues para as semanas seguintes, cada uma pequena o bastante para uma sessão de duas horas:

- Workspace Rust com `duckgr-sim` e `duckgr-data`, e CI no GitHub Actions (fmt, clippy, test).
- Grade de células com camadas e o comando "colocar bloco" com autor e tick.
- Comando "quebrar bloco" e a regra "só quebra o que colocou", vinda de configuração do mundo.
- Teste de determinismo por hash de estado.
- Regras do mundo de Paz (aviso e expulsão) e do mundo de Guerra (guerra automática), como dados.
- Núcleo em WebAssembly e cliente web mínimo, com demo no GitHub Pages a cada merge.

Me mostre o plano antes de escrever os arquivos.
