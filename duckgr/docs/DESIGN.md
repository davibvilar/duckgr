# DuckGR: documento de design

Rodrigo Vilar e Davi Vilar · exportado do documento vivo em 27 de outubro de 2026

## Visão em uma página

A proposta é um jogo de automação aberto, leve e jogável no navegador, em que um time constrói junto uma rede produtiva para cumprir missões. Do Factorio vem o loop viciante: extrair, transformar, transportar e pesquisar. O que muda é o propósito: o jogo é um instrumento de facilitação para cursos de liderança e dinâmicas terapêuticas de grupo.

Três escolhas sustentam o resto do documento:

- **Indústria real com um mundo que se cura.** Minérios, cadeias, poluição e besouros como no Factorio, mais reflorestamento, agricultura e eras desde o início; investir em ecologia rende vantagens concretas (seção Proposta temática).
- **Núcleo de simulação em Rust, isolado da tela.** O mesmo crate roda no navegador (WebAssembly), no desktop e, depois, no servidor multiplayer (seção Arquitetura técnica).
- **Tudo é comando registrado.** Cada ação do jogador vira um evento com autor e tick. Isso dá replay da sessão e métricas para o facilitador conduzir o debriefing — o diferencial pedagógico.

A primeira versão é para um jogador, com uma cadeia de produção curta. Cada fase seguinte acrescenta uma mecânica e só avança quando a anterior está divertida sozinha.

## Plano de lançamento

O projeto é liderado por Rodrigo e Davi e nasce aberto: primeiro a chamada e as ideias, depois o código, devagar e em público, para que a comunidade cresça junto com o jogo.

### Fase A — Preparar a casa (antes de anunciar)

- [ ] **Conferir os contratos de trabalho** quanto a propriedade intelectual de projetos pessoais, já que há vínculo com mais de uma instituição. Não sou advogado; vale uma leitura cuidadosa ou uma consulta rápida.
- [ ] **Escolher o nome** e checar se estão livres: domínio, organização no GitHub, conta Nostr e registro de marca no INPI.
- [ ] **Decidir a licença** do código (MIT/Apache-2.0 ou AGPL, ver Decisões em aberto) e da arte (CC BY-SA), e a forma de aceitar contribuições: DCO (uma linha de assinatura em cada commit) é mais leve que um contrato de cessão.
- [ ] **Criar as contas do projeto:** organização no GitHub, domínio, chave Nostr do projeto e um e-mail de contato.
- [ ] **Escrever os documentos-base do repositório:** README (o que é, estado atual, como ajudar), VISION (este documento, revisado), CONTRIBUTING, CODE\_OF\_CONDUCT (o Contributor Covenant é o padrão), GOVERNANCE (quem decide: Rodrigo e Davi, e como isso pode evoluir) e LICENSE.

### Fase B — Chamada inicial (o anúncio)

- [ ] **Landing page** com quatro blocos: a ideia em uma frase, por que importa (liderança, colaboração, um mundo que se cura, tudo aberto), em que estado está (ideia, procurando colaboradores) e como participar. Inscrição para novidades por Nostr e por e-mail.
- [ ] **Repositório público** só com a documentação, sem código ainda. Ativar GitHub Discussions, criar etiquetas (`design`, `bom-primeiro-issue`, `pesquisa`, `arte`) e um quadro público de roadmap no GitHub Projects.
- [ ] **Decisões em aberto viram discussões públicas** (RFCs): nome, licença, formato de dados, tema. A comunidade entra decidindo, não só assistindo.
- [ ] **Escolher um único canal de conversa** além do GitHub, para não dispersar: Nostr, Matrix ou Discord.
- [ ] **Divulgar** em comunidades que têm afinidade com cada pedaço do projeto: Rust e desenvolvimento de jogos, Nostr e Bitcoin, universidades (começando pela UFCG), facilitadores, RH e quem trabalha com terapia de grupo.

### Fase C — Fundação técnica, em público (fase 1 do roadmap)

- [ ] **Esqueleto do código:** workspace Rust com `vale-sim`, `vale-data` e `vale-wasm` (nomes provisórios).
- [ ] **CI desde o primeiro commit:** testes de determinismo e benchmark de desempenho, como descrito no Orçamento de desempenho.
- [ ] **Demo jogável publicada no navegador a cada merge**, por exemplo no GitHub Pages: o progresso fica visível para quem não lê código.
- [ ] **Diário de desenvolvimento quinzenal**, no espírito das Friday Facts do Factorio: o que foi feito, uma decisão explicada, um pedido de ajuda.
- [ ] **Primeiras issues para novos contribuidores**, pequenas e bem descritas, em código, arte, tradução e design de missões.

### Fase D — Primeiros jogáveis e primeiros grupos (fases 2 a 4 do roadmap)

- [ ] **Playtests abertos mensais**, com formulário curto de impressões.
- [ ] **Primeira oficina piloto com um grupo de liderança real**, conduzida por vocês, e um relato público do que funcionou.
- [ ] **Convidar facilitadores** para testar o painel e contribuir com missões.
- [ ] **Reconhecer quem contribui:** lista de contribuidores no site e menções no diário.

### Fase E — Comunidade e ecossistema (fase 5 do roadmap em diante)

- [ ] Multiplayer, editor de missões e o SDK de mundos, cada um anunciado como marco.
- [ ] Mercado em sats e mentorias, quando houver base de usuários que justifique.

**Rituais que mantêm a comunidade viva:** o diário quinzenal, o playtest mensal, uma conversa aberta por mês sobre o roadmap e responder toda contribuição em poucos dias. Projetos abertos crescem menos pelo código publicado e mais pela sensação de que há gente do outro lado.

## Como o Factorio funciona

O Factorio é um laço de retroalimentação: você automatiza a produção das próprias ferramentas que permitem automatizar mais. O loop começa em minerar ferro, fundir em placas, montar engrenagens e usar essas peças para fabricar mais máquinas ([Summer Engine](https://www.summerengine.com/templates/strategy/4x-strategy/factorio-style)). A pesquisa fecha o ciclo: os pacotes de ciência verdes exigem esteiras e insersores, então o jogador é forçado a automatizar a infraestrutura antes de avançar ([Supercraft](https://supercraft.host/article/factorio-automation-constraints/)).

### Os sistemas, do essencial ao avançado

| Sistema | O que faz | Por que importa para nós |
| --- | --- | --- |
| Recursos e extratores | Jazidas finitas no mapa; mineradoras extraem itens brutos | Escassez e localização criam decisões de onde construir |
| Receitas e máquinas | Fornalhas e montadoras transformam N entradas em M saídas com um tempo de ciclo | É onde nascem as proporções (quantas máquinas de A alimentam uma de B) |
| Esteiras | Transportam itens em duas faixas, com velocidade e densidade fixas | O coração do jogo; o gargalo visível |
| Insersores | Braços que pegam de uma esteira ou baú e colocam numa máquina | Ligam o transporte à produção; sem eles nada se conecta |
| Energia | Rede elétrica que alimenta máquinas | Uma dependência transversal: se cai, tudo para |
| Pesquisa | Laboratórios consomem pacotes de ciência e liberam tecnologias | Dá direção e ritmo de progressão |
| Ameaça | Poluição atrai inimigos que evoluem conforme a fábrica cresce | Pressão externa; crescer tem custo |
| Logística avançada | Trens, robôs e circuitos lógicos | Muda o jogo de construir para projetar em escala |

### Três lições de engenharia do próprio estúdio

1. **Simulação determinística com lockstep.** O multiplayer do Factorio sincroniza os clientes enviando apenas as entradas dos jogadores, não o estado do mundo ([FFF #188](https://factorio.com/blog/post/fff-188)). Isso gasta pouquíssima banda, mas qualquer divergência gera dessincronização, e um cliente lento atrasa todos ([FFF #76](https://factorio.com/blog/post/fff-76)).
2. **Esteiras como segmentos, não peças.** Atualizar cada tile de esteira isoladamente não escala; a solução foi fundir trechos contínuos em linhas de transporte tratadas como uma unidade ([FFF #148](https://factorio.com/blog/post/fff-148)).
3. **Números inteiros e fixos.** Na física das esteiras, um tile tem 256 unidades de comprimento, cada item ocupa 64, e a esteira básica avança 8 por tick, o que dá 7,5 itens por segundo por faixa ([Wiki](https://wiki.factorio.com/Transport_belts/Physics)). Aritmética inteira é o que torna o determinismo viável.

Para o nosso caso, o mínimo jogável é esteira + insersor + máquina + receita + pesquisa. Energia, ameaça e logística avançada entram depois, cada uma como uma fase.

## O que aprender com Mindustry e shapez

Os dois jogos abertos mais conhecidos do gênero provam que dá para simplificar o Factorio sem perder o vício — e cada um simplificou numa direção diferente.

| Jogo | Licença e plataforma | O que simplificou | O que levar |
| --- | --- | --- | --- |
| [Mindustry](https://www.gamingonlinux.com/2018/12/mindustry-an-open-source-sandbox-tower-defense-game-thats-a-little-like-factorio/) | GPL; desktop e mobile | Mistura fábrica com defesa de torres; partidas curtas em vez de bases infinitas ([AlternativeTo](https://www.alternativeto.net/software/factorio/)) | Sessões com começo, meio e fim — ideal para caber num encontro de 90 minutos |
| [shapez](https://github.com/tobspr/shapez.io) | GPL; roda no navegador | Sem combate nem minérios variados: o jogador corta, gira e pinta formas | Um único material abstrato reduz a curva de aprendizado; o desafio fica todo na logística |

Duas observações práticas. Primeiro, o repositório do shapez original não é mais mantido ativamente, então serve de referência de design, não de base de código. Segundo, ambos são GPL: se reaproveitarmos código deles, nosso projeto herda a licença. Escrever do zero nos deixa livres para escolher (sugestão em Decisões em aberto).

## Proposta temática

Mantemos o coração do Factorio — minérios reais, cadeias industriais, poluição e a fauna que ela atrai — e acrescentamos desde a primeira era um segundo sistema: o mundo se cura, e quem investe nele colhe benefícios. O jogador atravessa eras, da fornalha a carvão à corrida espacial, conforme pesquisa, e cada missão mede produção e equilíbrio ao mesmo tempo.

### O que mantemos do Factorio

- **Minérios e cadeias do mundo real.** Ferro, cobre, carvão e pedra; petróleo na era química. Placa vira engrenagem, engrenagem vira máquina — a ligação com a indústria de verdade é parte do valor didático.
- **Logística.** Insersores, montadoras, proporções e gargalos, como no original; o transporte, porém, evolui com as eras (abaixo).
- **Poluição que se espalha.** Máquinas emitem, a poluição avança pelo mapa chunk a chunk.
- **Fauna que reage.** Besouros e ninhos que evoluem e atacam as fontes de poluição.
- **Pesquisa com pacotes.** A árvore tecnológica continua guiando a progressão.

### O que acrescentamos: o mundo que se cura

No Factorio base as árvores absorvem poluição, mas não podem ser replantadas; a expansão Space Age passou a permitir plantá-las com a torre agrícola, principalmente como fonte de madeira ([Wiki](https://wiki.factorio.com/Tree)). Aqui a ecologia é um sistema de primeira classe, com ramo próprio na árvore de pesquisa.

| Mecanismo | Como funciona | O que o jogador ganha |
| --- | --- | --- |
| Absorção natural | Floresta, solo saudável e áreas úmidas absorvem poluição do chunk a cada tick | Área limpa não atrai besouros |
| Viveiro e reflorestamento | Mudas são produzidas numa máquina e plantadas; levam minutos para crescer e precisam de água | Absorção crescente e madeira renovável |
| Agricultura | Fazendas produzem alimento e biomassa | Combustível, fertilizante e, mais tarde, bioplástico |
| Monocultura × policultura | Monocultura rende mais no início, mas esgota o solo e atrai pragas; policultura rende menos por tile e mantém o solo fértil | Um trade-off explícito entre curto e longo prazo, decidido na pesquisa |
| Saúde do solo e da água | Cada chunk tem índices que caem com poluição e mineração e sobem com cuidado | Bônus no rendimento das fazendas e das caldeiras |
| Fauna que recua | Com a poluição baixa, os ninhos param de evoluir e se afastam; combater continua possível, mas não é a única saída | Defesa mais barata que torres |
| Recuperação de áreas | Minas esgotadas podem ser recuperadas com aterro, solo e mudas | O terreno volta a produzir |

### Logística que evolui com as eras

As esteiras do Factorio aparecem cedo demais e não gastam energia, o que soa estranho. Aqui o transporte acompanha a história: começa com o próprio jogador carregando, passa por estradas e força animal, e a esteira só chega quando já existe indústria para movê-la.

| Era | Transporte | Custo | O que muda no jogo |
| --- | --- | --- | --- |
| 1. Carvão e vapor | O jogador carrega; depois trilhas e estradas de terra com carroças puxadas por animais | Animais comem ração das fazendas; estrada de terra vira lama com chuva e uso | Logística é rota, não layout: de onde para onde, quantos animais, quanta ração |
| 2. Eletricidade | Estrada de pedra, carroças maiores | Manutenção das estradas | Mais carga por viagem, menos perda |
| 3. Aço e automação | Ferrovia entre polos; esteiras elétricas, dentro e fora das fábricas | Esteiras consomem energia elétrica | Começa o quebra-cabeça de layout do Factorio, agora como conquista |
| 4. Petróleo e química | Estrada de asfalto e caminhões | Combustível e poluição por viagem | Rápido, mas poluente: caminhão a diesel × ferrovia |
| 5 em diante | Robôs logísticos, ferrovia elétrica | Energia e manutenção | Logística vira projeto em escala |

Três consequências boas. Primeiro, a agricultura deixa de ser opcional: animal sem ração para, então a primeira fazenda já tem função industrial (e o esterco pode virar fertilizante). Segundo, cada modo de transporte tem um custo visível em energia, comida ou poluição — nada é de graça. Terceiro, o jogador sente a evolução: carregar à mão no início faz a carroça parecer uma revolução, e a esteira, depois, outra.

O risco é perder o quebra-cabeça que torna o Factorio tão didático, o de proporções e gargalos visíveis na esteira. Por isso as esteiras não somem: entram na era 3, quando já há aço e eletricidade para construí-las e movê-las, e podem cruzar o mapa ou ficar dentro das fábricas, e as rotas de animais e veículos também mostram fluxo e gargalo (fila na estação de carga, animal parado sem ração).

### Água: do aqueduto ao encanamento

No Factorio a água sai de uma bomba na margem e corre por canos sem regra física. Aqui ela é um recurso com geografia: precisa ser levada de onde existe até caldeiras, viveiros, fazendas e animais, e a solução evolui com as eras — começando pelo aqueduto, que existe desde a Antiguidade.

| Era | Como a água chega | Regra e custo |
| --- | --- | --- |
| 1. Carvão e vapor | Baldes à mão, depois canais de terra e aquedutos de pedra | Funciona como um canal da fonte ao destino; o de terra perde água por vazamento e evaporação, o aqueduto de pedra perde menos |
| 2. Eletricidade | Canos de ferro com bombas elétricas, cisternas | Mais vazão e alcance, mas a bomba consome energia |
| 3–4. Aço, petróleo e química | Rede industrial, tratamento de efluentes | Efluente sem tratamento contamina o rio a jusante |
| 5 em diante | Irrigação inteligente, reuso, monitoramento | Menos água por colheita, com dados por chunk |

A ligação com a ecologia é direta: nascentes e aquíferos são finitos e se esgotam com uso excessivo, e um rio poluído derruba a saúde dos chunks por onde passa. Floresta preservada em volta de uma nascente aumenta a vazão dela.

**Impacto técnico.** Aquedutos e canos são redes conectadas, como os canais da logística: cada rede calcula a vazão por tick, das fontes aos consumidores, sem simular o fluido trecho a trecho. Altitude e gravidade ficam fora por enquanto, como possível extensão futura.

**Impacto técnico.** Animais e veículos não andam livres pelo mapa: o jogador define rotas entre estações de carga e descarga, como os horários de trem do Factorio. A simulação calcula o caminho uma vez sobre o grafo de estradas e só recalcula quando a estrada muda, o que mantém o custo baixo e o determinismo.

### Eras

As eras nascem da pesquisa, como no Factorio: cada uma é aberta por uma tecnologia-marco e traz seu próprio pacote de ciência. A diferença é que a virada fica explícita — o jogo anuncia "Estamos entrando na era da eletricidade", mostra o que muda no mundo e libera as missões daquela era.

| Era | Pesquisa que abre a era | Indústria e energia | Ecologia que entra | Tensão principal |
| --- | --- | --- | --- | --- |
| 1. Carvão e vapor | Ponto de partida | Mineração e fundição a carvão, caldeira e máquina a vapor | Plantio manual de mudas, horta | Crescer rápido sem devastar a floresta inicial |
| 2. Eletricidade | Eletricidade | Rede elétrica, montadoras, insersores elétricos | Viveiro automatizado, primeiras fazendas | A poluição dispara: defender ou compensar |
| 3. Aço e automação | Siderurgia | Aço, primeiras esteiras, circuitos, ferrovia | Monocultura × policultura, rotação de culturas | Escala industrial sobre um solo que cansa |
| 4. Petróleo e química | Refino de petróleo | Plástico, enxofre, combustíveis | Filtros nas chaminés, tratamento de efluentes, bioplástico | Produtos mais valiosos, rejeitos mais pesados |
| 5. Informação e robótica | Robótica | Robôs logísticos, circuitos lógicos, sensores | Monitoramento ambiental por chunk, irrigação inteligente | Ver o sistema inteiro e decidir com dados |
| 6. Nuclear | Fissão nuclear | Reator, enriquecimento de urânio | Gestão de rejeito radioativo, zonas de exclusão | Muita energia limpa no ar, risco concentrado no chão |
| 7. Renovável | Energia solar e eólica | Painéis, turbinas, baterias | Restauração de solo, corredores ecológicos, retorno da fauna | Trocar a matriz sem parar a produção |
| 8. Espacial | Ciência espacial | Foguete e satélite | Satélite de observação da Terra | O projeto final |

Nuclear e renovável podem ser pesquisados em qualquer ordem, ou em paralelo; a era espacial exige as duas. É uma decisão estratégica de verdade para o time, e boa de discutir no debriefing.

Quando o time entra numa nova era, três coisas mudam juntas: as máquinas novas poluem mais (ou menos), os besouros evoluem um degrau e o ramo ecológico ganha ferramentas à altura. Cada era, portanto, é uma nova versão do mesmo equilíbrio.

O equivalente ao lançamento do foguete é colocar em órbita o satélite de observação, e ele só pode ser lançado com produção industrial alta **e** saldo ecológico positivo — nenhum dos dois sozinho basta.

### Missões no centro

O Factorio é aberto; aqui cada sessão tem uma missão com metas verificáveis em duas dimensões, produção e equilíbrio. O jogo mostra o progresso das metas a todo momento, e o facilitador acompanha ao vivo.

| Missão | Meta de produção | Meta ecológica | O que exercita |
| --- | --- | --- | --- |
| Primeiro aço | Atingir um ritmo de aço por minuto | A floresta inicial não pode cair abaixo da metade | Planejamento sob restrição |
| Transição | Manter a produção enquanto troca carvão por solar | Poluição total em queda contínua | Gestão de mudança |
| A praga | Recuperar uma fazenda de monocultura infestada | Solo de volta ao saudável | Visão de longo prazo |
| Cerco | Sobreviver a ataques crescentes | Escolher entre construir torres e reduzir a poluição | Decisão sob pressão |

Missões são dados: objetivo, tempo, recursos iniciais, restrições e eventos programados. O nome do jogo fica em aberto.

### Primeira versão jogável: Terra Viva, era 1

O primeiro pedaço jogável (pasta `web/`) cobre só a era do carvão e do vapor, para um jogador. A ideia é crescer aos pedaços: cada pedaço novo de jogo entra como dado no pacote, e só quando um dado não basta é que o núcleo ganha uma peça nova.

| Peça do modelo | O que já existe na Terra Viva |
| --- | --- |
| Substrato | Solo e água (rio gerado pela semente) |
| Depósito | Ferro, cobre, carvão e pedra, com quantidade finita |
| Cobertura | Grama, muda, árvore jovem, árvore adulta e toco; mudas crescem e crescem mais rápido ao lado da água |
| Ocupante (elementos) | Mineradora a carvão, fornalha de pedra, baú e viveiro, todos 1×1 |
| Componentes | Energia (combustível), extração, processo (receitas), entrega ao vizinho, emissão de campo, inventário |
| Campo | Poluição por trecho de 8×8: máquinas emitem, coberturas absorvem, acima de um limiar se espalha; poluída demais, impede o crescimento e faz as árvores regredirem |
| Logística | O jogador carrega; a única automação é a entrega direta entre máquinas vizinhas (mineradora → fornalha → baú) |
| Pesquisa e eras | Pacotes de ciência pesquisam o viveiro e depois a eletricidade, que abre a era 2 (ainda vazia) |
| Missão | Primeira fundição: produzir 50 placas de ferro sem a floresta cair abaixo de 50% da inicial |

Ficaram de fora de propósito, para os próximos pedaços: fauna, água em rede, estradas e carroças, horta e fazendas, saúde do solo, pegadas maiores que 1×1, insersores e esteiras. A tela de montagem já deixa escolher semente, tamanho do mapa, floresta, recursos e metas, e a partida é salva como lista de comandos e refeita no carregamento, o que testa o determinismo a cada vez que alguém abre o jogo.

## Indo além do Factorio: lições da história e da ficção

O Factorio começa numa fábrica e só tem uma relação com o mundo: extrair e poluir. A história humana e a boa ficção científica mostram um percurso muito mais rico — fogo, agricultura, cidades, crises e reconstruções — e cada etapa sugere uma mecânica. Esta seção é um banco de ideias candidatas; as que forem aprovadas sobem para a Proposta temática.

### O que a história humana ensina

| Marco | O que aconteceu | Mecânica possível |
| --- | --- | --- |
| Fogo e lenha | Por milênios a energia foi madeira; na Inglaterra dos séculos XVI–XVII, o carvão vegetal para fundir ferro desmatou tanto que empurrou a troca para o carvão mineral | Uma era zero de lenha e carvão vegetal: o primeiro desmatamento força a primeira transição energética |
| Revolução agrícola | Domesticar plantas e animais gerou excedente de comida, e o excedente liberou gente para outros ofícios | Comida como base de tudo: excedente libera colonos para virar especialistas |
| Bronze | Cobre mais estanho, que é raro e fica longe: o bronze criou as primeiras rotas de comércio de longa distância | Ligas que exigem um recurso que só existe em outra região do mapa — ou na base de outro time |
| Moinhos de água e de vento | Energia renovável movia moinhos e serrarias séculos antes do vapor | Renováveis já na era 1, presas à geografia; o vapor vence por poder ser instalado em qualquer lugar |
| Escrita e bibliotecas | Registrar conhecimento permitiu acumulá-lo entre gerações; perder uma biblioteca era perder séculos | Pesquisa guardada num lugar físico: se o laboratório é destruído sem cópia, o saber se perde |
| Rotação de culturas | Do sistema de três campos medieval à rotação de quatro anos de Norfolk, a terra deixou de cansar | A base do ramo mono × policultura |
| Fome da batata na Irlanda (1845–1852) | Uma única variedade plantada em toda parte foi destruída por uma praga | Monocultura rende muito até o dia em que uma praga leva tudo |
| Cólera em Londres (1854) | John Snow ligou a epidemia a uma bomba d'água contaminada; nasceu o saneamento moderno | Água contaminada adoece colonos; saneamento vira pesquisa |
| Revolução industrial | Carvão e vapor multiplicaram a produção e a fumaça | A era 1 do jogo, como já está |
| Grande Smog de Londres (1952) | Milhares de mortes levaram à Lei do Ar Limpo de 1956 | Leis e regulações como pesquisa, com custo e benefício |
| Rio Cuyahoga em chamas (1969) | Um rio tão poluído que pegou fogo virou símbolo do movimento ambiental nos EUA | Eventos de crise que mudam a disposição do time |
| Haber-Bosch e Revolução Verde | Fertilizante sintético alimentou bilhões e trouxe excesso de nutrientes em rios e mares | Toda solução poderosa tem um efeito colateral a gerenciar |
| Protocolo de Montreal (1987) | Países coordenados recuperaram a camada de ozônio | Missão de crise global que só se resolve com todos os times cooperando |
| Tragédia dos comuns × Ostrom | Hardin previu que recursos comuns se esgotam; Elinor Ostrom, Nobel de 2009, mostrou comunidades que criam regras e os preservam | Terras e águas comuns, governadas por regras que o próprio time combina |

### O que a ficção científica imagina

| Obra | A ideia | Mecânica possível |
| --- | --- | --- |
| *Perdido em Marte* (livro e filme) | A sobrevivência começa plantando em solo estéril, reaproveitando cada resíduo | Ciclos fechados: todo resíduo pode virar insumo, e fechar ciclos é um ramo de pesquisa |
| *Nausicaä do Vale do Vento* (Miyazaki) | A floresta tóxica que todos temem está, na verdade, purificando o solo; os insetos gigantes a protegem | Os besouros guardam as florestas: atacam quem as destrói, e entendê-los rende mais que combatê-los |
| Trilogia de Marte (Kim Stanley Robinson) | Terraformar leva gerações e divide os colonos entre preservar o planeta como é e transformá-lo | Facções internas com valores diferentes, que o time precisa conciliar |
| *Duna* (Frank Herbert) | Num planeta deserto, a água é poder e o plano de reverdecer leva séculos | Água como recurso estratégico, e projetos de longo prazo cujo retorno vem tarde |
| *Avatar* | Colonização extrativista contra um povo e um ecossistema interligados | O custo de tratar o mundo como estoque |
| *Interstellar* | Pragas que destroem uma cultura após outra até sobrar só uma | A fragilidade da monocultura, levada ao limite |
| *Fundação* (Asimov) | Preservar o conhecimento para encurtar a idade das trevas após o colapso | Bibliotecas e cópias de pesquisa como seguro |

### O que outros jogos já testaram

- **Frostpunk:** carvão como centro da sobrevivência e leis votadas que mudam a moral da sociedade — mostra que decisões políticas podem ser mecânica.
- **Oxygen Not Included:** ciclos fechados de água, gás e comida com colonos que têm necessidades.
- **Anno:** a população sobe de nível quando suas necessidades são atendidas, e cada nível pede novos produtos.
- **Alpha Centauri:** o planeta reage como um organismo à exploração.

### Propostas que saem daqui

1. **Era zero: fogo, lenha e pedra.** O jogo começa antes do carvão; o primeiro desmatamento é a primeira lição de sustentabilidade.
2. **Colonos com necessidades.** Comida, água limpa e moradia; o excedente libera especialistas. É a maior mudança desta lista: transforma o jogo em fábrica mais sociedade, e precisa de decisão consciente.
3. **Recursos distribuídos de forma desigual.** Cada região ou time tem o que os outros não têm; o comércio passa a ser necessário, não opcional — combustível perfeito para colaboração.
4. **Bens comuns com regras do time.** Florestas, rios e pastos compartilhados, com regras de uso que o grupo combina e pode quebrar.
5. **Leis como pesquisa.** Regulação de emissões, saneamento, reserva florestal: cada lei tem custo e benefício.
6. **Fauna guardiã.** Os besouros protegem florestas e nascentes; convivência e pacificação passam a ser caminhos possíveis.
7. **Ciclos fechados.** Resíduos como insumo; economia circular como ramo de pesquisa.
8. **Conhecimento que pode ser perdido.** Pesquisa guardada em bibliotecas físicas, com cópias como proteção.
9. **Crises globais.** Eventos que nenhum time resolve sozinho, inspirados no Protocolo de Montreal.
10. **Clima de longo prazo.** Emissões acumuladas mudam chuvas e secas devagar: o efeito de hoje aparece três eras depois.

Uma nota sobre a palavra "colonização": na história ela inclui exploração de povos e de terras. No próprio Factorio, os inimigos são, na prática, os habitantes nativos do planeta. Tratar a fauna como guardiã, e não só como praga, é uma forma de o jogo carregar essa reflexão sem sermão — e rende boas conversas no debriefing.

## Design para grupos: missões, papéis e facilitação

O jogo só serve à dinâmica se a colaboração for necessária, não opcional. A regra de design é: nenhum jogador consegue cumprir a missão sozinho, e o gargalo do time deve ficar visível para todos.

### Alavancas de colaboração

- **Papéis assimétricos.** Cada jogador tem uma habilidade ou visão que os outros não têm: um vê o mapa inteiro, outro é o único que constrói oficinas, outro controla o estoque comum.
- **Recurso compartilhado e escasso.** Um estoque único de materiais obriga o time a negociar prioridades.
- **Informação parcial.** O objetivo completo da missão é dividido entre os jogadores; só juntando as partes se entende o que construir.
- **Comunicação restrita (opcional).** O facilitador pode limitar o chat ou tornar um jogador o único que fala com os outros, para trabalhar escuta e delegação.
- **Eventos de crise.** Uma onda de besouros ou uma pane de energia no meio da missão testa adaptação e liderança sob pressão.

### Mecânicas e o que elas exercitam

| Mecânica | Competência exercitada | Pergunta para o debriefing |
| --- | --- | --- |
| Gargalos e proporções | Pensamento sistêmico | Onde estava o gargalo, e quem percebeu primeiro? |
| Árvore de conhecimento | Priorização e visão de longo prazo | O que vocês escolheram pesquisar e por quê? |
| Estoque comum | Negociação e confiança | Alguém guardou recurso só para si? |
| Papéis assimétricos | Delegação e interdependência | Como vocês decidiram quem fazia o quê? |
| Eventos de crise | Adaptação e comunicação sob pressão | O que mudou na forma de falar quando o ataque chegou? |

### O papel do facilitador

O facilitador não joga: ele monta a sessão e conduz a conversa depois. Para isso precisa de um painel próprio com quatro funções:

1. Escolher a missão, os papéis e as restrições antes de começar.
2. Disparar eventos de crise durante a sessão.
3. Ver métricas ao vivo: produção por minuto, tempo parado por gargalo, quem construiu o quê, volume de mensagens por pessoa.
4. Rever a sessão em replay acelerado no debriefing, pausando nos momentos decisivos.

O item 4 é o que um jogo comercial não oferece, e é viabilizado pela arquitetura de comandos registrados da próxima seção. Como as métricas descrevem pessoas reais em contexto terapêutico, o registro deve ser anônimo por padrão e apagado ao fim da sessão, a menos que o grupo concorde em guardá-lo.

## Arquitetura técnica

O centro é um crate de simulação em Rust puro, sem nenhuma dependência de gráficos ou rede. Navegador, desktop e servidor são cascas finas em volta dele; a única forma de mudar o mundo é enviar um comando.

&#91;embedded content: arquitetura · núcleo, adaptadores e clientes\]

No jogo solo o cliente web conversa com a simulação rodando no próprio navegador; na fase 5 a mesma conversa passa a ir para o servidor (linha tracejada), sem mudar o núcleo.

### Componentes

| Componente | Responsabilidade | Tecnologia sugerida |
| --- | --- | --- |
| `vale-sim` | Mapa, entidades, receitas, esteiras, pesquisa, poluição, ecologia; avança o mundo em ticks fixos | Rust, aritmética inteira, sem `std::time` nem threads |
| `vale-data` | Itens, receitas, árvore de conhecimento e missões como dados | Arquivos RON ou TOML, validados por um teste no build |
| `vale-wasm` | Expõe a simulação ao navegador, rodando num Web Worker para não travar a tela | `wasm-bindgen` |
| Cliente web | Desenha o mapa e a interface; traduz cliques em comandos | TypeScript + PixiJS para o mapa, HTML para menus |
| Cliente desktop | O mesmo cliente web empacotado, com a simulação nativa | Tauri |
| `vale-server` (fase multiplayer) | Roda a simulação autoritativa e distribui diffs aos jogadores | Rust, axum, WebSocket |
| Painel do facilitador | Configura a sessão, dispara crises, mostra métricas e replay | Mesmo stack do cliente web |

### Decisões de projeto

- **Tick fixo e determinismo.** A simulação avança num passo fixo (sugiro 30 ticks/s; o Factorio usa 60), com inteiros e um gerador aleatório semeado. Mesmo estado inicial + mesmos comandos = mesmo resultado, sempre. É o que viabiliza replay, testes e save confiável.
- **Comandos como única entrada.** `Construir``Maquina, GirarEsteira, PlantarMuda`, `Pesquisar`… cada um com autor e tick. O registro de comandos é ao mesmo tempo o save (snapshot + log), o replay do debriefing e a fonte das métricas.
- **Esteiras como segmentos.** Trechos contínuos de esteira viram uma linha de transporte única, atualizada de uma vez, como no Factorio. Poluição e índices ecológicos ficam por chunk, não por tile. Mapa dividido em chunks (por exemplo 32×32 tiles) para que só as áreas ativas e visíveis custem processamento.
- **Multiplayer autoritativo, não lockstep.** O lockstep do Factorio economiza banda, mas um cliente lento atrasa todos e qualquer divergência dessincroniza. No navegador isso é frágil: abas em segundo plano são pausadas e notebooks de alunos variam muito. Com servidor autoritativo, para 4 a 8 jogadores e mapas pequenos, o tráfego de diffs é aceitável. O determinismo continua valendo para replay e testes.
- **Roteiro de missões depois, em dados primeiro.** Missões começam como dados declarativos (objetivo, recursos, papéis). Quando precisarem de lógica, uma linguagem embutida em Rust puro, como Rhai, roda também no WebAssembly.

### Onde o mundo roda

O mundo inteiro é simulado o tempo todo, esteja alguém olhando ou não, como no Factorio: a mina que você deixou do outro lado do mapa continua produzindo. "Fora da tela" nunca significa "fora da simulação"; a área de interesse só decide o que é desenhado e o que trafega pela rede. Por isso o orçamento de simulação é medido no mapa inteiro, e por isso máquinas que dormem e produção agendada são essenciais — uma fábrica que ninguém vê não pode custar o mesmo que uma em construção.

O que muda entre os modos é quem hospeda essa simulação:

| Modo | Onde a simulação roda | Quando usar | Cuidado |
| --- | --- | --- | --- |
| Solo no navegador | Web Worker na própria aba | Jogo individual, demonstração | Navegadores podem desacelerar abas em segundo plano; ao voltar, o jogo recupera os ticks perdidos em alta velocidade (o determinismo permite) ou simplesmente pausa |
| Solo no desktop | Processo nativo no app Tauri | Mapas maiores, máquinas fracas | — |
| Servidor local | `vale-server` no notebook do facilitador; jogadores entram pelo navegador na mesma rede Wi-Fi | Oficinas presenciais, sem depender de internet | O notebook do facilitador precisa ficar ligado e na mesma rede |
| Servidor na nuvem | O mesmo `vale-server` numa máquina remota | Sessões remotas, turmas grandes | Custo de hospedagem e latência |

Servidor local e na nuvem são o mesmo binário, que também serve o painel do facilitador. Nos dois, a simulação continua rodando mesmo que todos os jogadores fechem a aba por um instante; ao fim da sessão, o mundo é salvo (snapshot + registro de comandos) e pausado até o próximo encontro.

### Nostr: identidade, descoberta e sinalização

Usamos Nostr para tudo o que acontece em volta da partida — quem é quem, como as pessoas se encontram e como abrem a conexão — e nunca para o tick. Relays não foram feitos para dezenas de mensagens por segundo com latência baixa; depois do encontro, jogadores e host conversam direto por WebRTC. O padrão já foi demonstrado num protótipo de jogo que usa mensagens cifradas do Nostr só para negociar o canal de dados e depois dispensa o relay ([vmsplice](https://blog.vmsplice.net/2023/09/how-nostr-could-enable-peer-to-peer-apps.html)), e há bibliotecas em Rust que fazem a sinalização WebRTC com eventos efêmeros embrulhados para o destinatário ([hashtree](https://docs.rs/hashtree-cli/latest/src/hashtree_cli/webrtc/signaling.rs.html)).

| Uso | Como funciona | Fica no relay? |
| --- | --- | --- |
| Identidade | Cada pessoa tem um par de chaves; quem já usa Nostr entra com a extensão do navegador, quem não usa recebe uma chave gerada na hora | Só a chave pública |
| Descoberta de sessão | O facilitador publica o anúncio da sessão com prazo de expiração; os jogadores entram por um código curto ou QR | Até expirar |
| Sinalização WebRTC | Oferta, resposta e candidatos de rede trocados como eventos efêmeros, cifrados para o destinatário | Não — efêmeros não são guardados |
| Lobby | Presença e chat antes de a partida começar | Efêmero ou com expiração |
| Biblioteca de missões | Missões publicadas como eventos assinados pelo autor; facilitadores seguem autores em quem confiam | Sim, é público por design |

**O que nunca vai para o Nostr:** comandos, ticks e diffs da partida (vão pelo canal direto), e métricas e replays de participantes. Em contexto terapêutico, dados de comportamento de pessoas reais não podem ir para relays públicos, nem cifrados.

**A conexão depois do encontro** é em estrela, com o host autoritativo no centro — o servidor local, o da nuvem ou, num jogo casual, o navegador de um jogador. Isso combina com o modelo de servidor autoritativo e exige uma pilha WebRTC em Rust no `vale-server`. Duas ressalvas práticas: redes corporativas e universitárias às vezes bloqueiam conexão direta, então é preciso um retransmissor (TURN) ou WebSocket como plano B; e numa oficina sem internet não há relay público, então o `vale-server` pode embutir um relay Nostr local e anunciar a sessão na própria rede.

### Motor e visualização: o que trafega entre eles

O cliente não executa o jogo: é um visualizador que mantém um espelho da área que está olhando. O motor, num servidor parrudo ou no Worker do próprio navegador, simula o mundo inteiro e manda a cada cliente só o necessário para desenhar aquela área naquele nível de zoom. O protocolo é o mesmo nos dois casos — muda só o transporte (mensagens internas do navegador no jogo solo, WebRTC ou WebSocket no multiplayer) —, então o cliente é um só.

&#91;embedded content: protocolo motor ↔ cliente · 6 canais e os assets\]

A resposta para "eventos ou dados das células?" é: os dois, cada um para o que faz melhor, mais um terceiro tipo que é o que deixa tudo fluido.

| Tipo de mensagem | Quando vai | O que leva | Por quê |
| --- | --- | --- | --- |
| Snapshot de chunk | Uma vez, quando o chunk entra na área do cliente | As camadas da célula comprimidas e a lista de elementos, só no detalhe do nível de zoom pedido | Terreno muda pouco; mandar tudo de uma vez é mais barato que mil eventos |
| Evento de mudança | Quando algo muda de estado | Elemento construído ou removido, cobertura que cresceu, máquina que parou por falta de insumo | Mudanças discretas são raras e pequenas |
| Movimento por parâmetros | Quando um movimento começa ou muda | Trecho de esteira com velocidade e espaçamento; carroça com rota, tick de saída e velocidade; máquina com tick de início e duração da receita | O cliente calcula sozinho onde cada coisa está a cada quadro; o motor só avisa quando algo muda (a esteira parou, a carroça desviou) |
| Resumo de mapa e campos | Em baixa frequência, por chunk | Cor dominante de cada célula, poluição, floresta, rotas | Basta para o modo mapa, que não precisa de nenhum detalhe fino |

O terceiro tipo é a chave: o motor descreve movimentos como função do tempo em vez de mandar a posição de cada item a cada tick. Uma esteira cheia correndo por uma hora gera uma mensagem quando começa e outra quando para. Toda mensagem leva o número do tick, e o cliente desenha um pouco atrás do tempo real para interpolar sem engasgos.

**Assinatura de área com folga.** O cliente informa o retângulo que vê e o nível de zoom; o motor responde com assinaturas e cancelamentos de chunks. A área assinada tem margem e histerese: um pequeno vaivém da câmera não fica assinando e cancelando o mesmo chunk.

**Zoom contínuo, sem tecla de mapa.** Três níveis, com transição suave entre eles conforme o zoom:

1. **Perto:** sprites completos, itens nas esteiras, animais e pessoas animados.
2. **Médio:** elementos como ícones, sem itens nas esteiras; agentes agrupados.
3. **Mapa:** só o resumo por chunk — cores, poluição, floresta, rotas. Nenhum sprite é necessário.

Como o nível vai na assinatura, ao afastar o zoom o motor para de mandar movimento e passa a mandar só resumos: área maior, dados muito menores.

**Painéis sob demanda.** Detalhes que não aparecem no mapa (o inventário de uma máquina, o progresso de uma pesquisa) só trafegam quando o jogador abre o painel, como uma assinatura daquele elemento; fecha o painel, cancela.

**Custo nas duas pontas.** No motor, o que muda num chunk é codificado uma vez e enviado a todos os clientes que o assinam, em vez de uma vez por cliente. No cliente, só a área visível existe na memória. As mensagens vão em binário, com o formato definido num crate compartilhado (`vale-protocol`) compilado também para o navegador, para que motor e cliente nunca discordem do formato.

### Assets sob demanda e qualidade automática

- **Manifesto com hash.** Cada pacote de conteúdo publica um manifesto: cada grupo de sprites, em cada nível de qualidade, identificado pelo hash do conteúdo. Arquivo com hash nunca muda, então pode ficar em cache para sempre — no navegador, num servidor de arquivos ou num armazenamento compatível com Nostr.
- **Carregamento preguiçoso.** Um grupo de sprites só é baixado quando algo dele entra na tela pela primeira vez, com pré-carga dos vizinhos da área visível. O modo mapa não precisa de nenhum.
- **Nunca travar o desenho.** Enquanto um sprite não chega, desenha-se um marcador simples na cor do elemento.
- **Níveis de qualidade.** Por exemplo alto, médio e leve, com atlas de resoluções diferentes. O cliente escolhe sozinho medindo o tempo de quadro e a velocidade da rede, ajusta durante o jogo e respeita a escolha do jogador quando ele fixa uma.

### O que a literatura diz, e o que adotamos

A literatura de jogos em rede organiza a sincronização em três famílias: lockstep determinístico, interpolação de snapshots e sincronização de estado ([tese sobre lockstep](https://hdms.bsz-bw.de/files/7107/DeterministicLockstepInNetworkedGamesPaper.pdf)). Um estudo quantitativo dessa tese mostrou que o lockstep escala bem em número de entidades mas sofre com o número de jogadores, e propôs um híbrido que usa snapshots para ressincronizar e para quem entra no meio da partida.

| Técnica | Quem usa e como | O que levamos |
| --- | --- | --- |
| Lockstep determinístico | O Factorio: todos os clientes simulam o mundo e trocam só as entradas dos jogadores ([FFF #302](https://cf-www.factorio.com/blog/post/fff-302)) | O determinismo, para replay, auditoria e testes — mas não como protocolo de rede no navegador |
| Ocultação de latência | O Factorio mantém um "estado de latência", cópia do estado de jogo refeita a cada tick com as ações locais ainda não confirmadas; só para ações simples, como andar, abrir ou girar ([FFF #83](https://factorio.com/blog/post/fff-83)) | A prévia imediata da construção no cliente, confirmada depois pelo motor |
| Interpolação com atraso | O motor Source desenha cerca de 100 ms atrás do tempo real, para sempre ter dois estados entre os quais interpolar mesmo se um pacote se perder ([Valve](https://developer.valvesoftware.com/wiki/Source_Multiplayer_Networking)) | O cliente desenha um pouco atrás do tick, como já previsto |
| Sincronização de estado | Servidor autoritativo manda o estado relevante a cada cliente | A base do nosso protocolo, com a variante de mandar movimento como função do tempo |
| Gestão de interesse | Pesquisa consolidada sobre mandar a cada jogador só o que acontece na sua área de interesse ([survey ACM, 2014](https://researcher.ibm.com/publications/interest-management-for-distributed-virtual-environments-a-survey)) | A assinatura de área por chunks com histerese |
| Mundo persistente que roda sozinho | O Screeps roda o mundo em ticks no servidor, executando o código dos jogadores mesmo com eles desconectados; quando cresceu demais, dividiu o mundo em shards independentes ligados por portais ([Screeps](https://docs.screeps.com/architecture.html), [shards](https://blog.screeps.com/2017/08/shards/)) | Mundos persistentes com bots vigias; e, no futuro, mundos de hosts diferentes ligados por portais |
| Dilatação do tempo | EVE Online desacelera o relógio do jogo sob carga extrema em vez de travar, e volta ao normal quando a carga passa ([CCP](https://eveonline.com/article/introducing-time-dilation-tidi)) | Válvula de segurança: se o motor não fechar o tick no orçamento, o mundo anda mais devagar para todos, sem engasgos |

**Síntese para os dois modos.** No jogo solo, o motor roda num Web Worker e o protocolo é só troca de mensagens interna: não há rede, só a disciplina de separar motor e tela. No multiplayer, servidor autoritativo com sincronização de estado, gestão de interesse por chunk, prévia local das ações do jogador e interpolação no cliente. O determinismo não é usado para sincronizar a rede, e sim para replay, auditoria por amostragem e testes — o que evita a fragilidade do lockstep com clientes lentos ou abas em segundo plano. A dilatação do tempo entra como proteção contra picos.

### Por que não Bevy de ponta a ponta

Bevy é a opção natural em Rust e compila para WebAssembly com renderização WebGPU ou WebGL2 ([Bevy](https://bevy.org/news/bevy-webgpu/)). O motivo para preferir núcleo Rust + cliente TypeScript é outro: o painel do facilitador e os menus são interfaces web comuns, e é mais fácil atrair contribuidores web para um projeto aberto. Se o jogo exigir efeitos visuais que o PixiJS não dê conta, o núcleo isolado permite trocar o cliente por Bevy sem reescrever a simulação.

### Testes

Testes de ouro de determinismo (rodar uma missão gravada por N ticks e comparar o hash do estado), testes de propriedade nos canais (itens nunca somem nem duplicam) e um benchmark de ticks por segundo num mapa de referência, rodado no CI.

## Orçamento de desempenho

Desempenho é requisito de projeto, não otimização para depois: cada camada tem um orçamento, medido no CI desde a fase 1, e uma mecânica nova só entra se couber nele. O aparelho-alvo proposto é um notebook comum de sala de aula, com GPU integrada, rodando no navegador.

| Camada | Orçamento proposto | Princípio |
| --- | --- | --- |
| Simulação | Até 10 ms dos 33 ms de cada tick (30 ticks/s), com o mapa de referência inteiro ativo, na era mais pesada | Só processa o que mudou |
| Desenho | 60 quadros/s com GPU integrada; nunca abaixo de 30 | Só desenha o que está na tela, no nível de detalhe do zoom |
| Rede | Poucos KB/s por jogador em regime normal | Só envia o que o jogador vê, em binário |
| Memória | Cabe confortavelmente numa aba de navegador | Dados compactos, sem objetos por item |

Os números são pontos de partida para calibrar com o primeiro protótipo, não medições.

### Simulação

- **Dados compactos por tipo.** Cada tipo de entidade vive em vetores contíguos (structure of arrays), com IDs inteiros e nenhuma alocação de memória durante o tick.
- **Máquinas dormem.** Uma máquina sem insumo ou com a saída cheia sai da lista de atualização e só acorda por evento (chegou item, liberou espaço). Uma fábrica parada custa quase nada.
- **Agendamento em vez de contagem.** Máquina em regime estável não conta o progresso a cada tick: agenda o tick em que a próxima peça fica pronta.
- **Transporte por segmento ou rota.** Esteiras e canais por trecho contínuo; animais e veículos seguem rotas calculadas uma vez; água por rede.
- **Sistemas lentos em rodízio.** Poluição, solo, crescimento de árvores e evolução dos besouros rodam a cada N ticks, distribuindo os chunks entre ticks para não gerar picos.
- **Mapas do tamanho da missão.** Uma missão de 45 minutos não precisa de mapa infinito. Mapa limitado e teto de entidades por missão são a alavanca mais barata de todas.
- **Fora da tela principal.** No navegador a simulação roda num Web Worker, sem depender de múltiplas threads em WebAssembly, que exigem cabeçalhos especiais de servidor.

### Desenho

- **Estilo visual que não pesa.** 2D visto de cima, sprites simples num único atlas, cores chapadas. Sem iluminação dinâmica, sombras em tempo real ou partículas pesadas. A beleza vem da paleta e da legibilidade, não de efeitos.
- **Terreno em cache.** O chão de cada chunk é desenhado uma vez numa textura e reaproveitado; só entidades que se movem são redesenhadas.
- **Níveis de detalhe.** De perto: itens nas esteiras e animais animados. De longe: blocos de cor por chunk, sem itens individuais. No mapa geral: só poluição, floresta e rotas.
- **Desenho desacoplado do tick.** A tela interpola entre ticks; num aparelho fraco ela cai para 30 quadros/s sem afetar a simulação.
- **Ponte binária.** O Worker entrega ao desenho buffers binários só da região visível, transferidos sem cópia — nunca objetos JSON.
- **Base gráfica comum.** WebGL2 via PixiJS roda em qualquer GPU integrada, com poucas chamadas de desenho graças ao atlas e ao agrupamento de sprites.

### Rede (fase multiplayer)

- **Área de interesse.** O servidor simula o mundo inteiro, mas envia a cada jogador só os chunks que ele vê, mais uma margem.
- **Estado, não itens.** Um trecho de esteira trafega como estado do segmento (velocidade e espaçamento); o cliente anima os itens localmente entre uma atualização e outra.
- **Binário e com cadência própria.** Diffs compactos em formato binário, de 10 a 15 envios por segundo, com interpolação no cliente.
- **Resposta imediata.** Ao construir, o cliente mostra a peça na hora como prévia e o servidor confirma em seguida.
- **Entrada barata.** Quem entra recebe um snapshot comprimido e depois só diffs.

### Como garantir

Um mapa de referência por era, com a fábrica típica do fim daquela era, roda no CI em três medidas: ticks por segundo (nativo e WebAssembly), chamadas de desenho por quadro e bytes por segundo por jogador num replay gravado. Estourou o orçamento, o build falha — e a discussão de design acontece antes do código, não depois.

## Roadmap incremental

As fases 1 a 3 são jogo solo; a 4 transforma o registro de comandos em ferramenta de facilitação; a 5 abre para o grupo. Cada portão é um critério observável, não uma data.

&#91;embedded content: roadmap · 6 fases, cada uma com um portão\]

A fase 1 é a única em destaque porque é onde começar: um vertical slice com uma cadeia só (minério de ferro → placa) já valida o núcleo, o determinismo e o desempenho no navegador.

## Backlog futuro (fora do MVP)

Algumas extensões ficam fora do MVP, mas a arquitetura precisa deixar espaço para elas desde a fase 1. O custo de prever agora é baixo; o de encaixar depois, alto.

### 1. Bots vigias e bots trabalhadores

Quem deixa o mundo rodando sem estar presente pode ter medo de um ataque de besouros. Um bot vigia cuida do mundo na ausência do dono: aciona defesas, repara o que foi danificado, reabastece fornalhas e avisa o dono quando algo sai do controle — por exemplo, com uma mensagem Nostr. Num segundo passo, o bot faz parte do trabalho como se fosse um jogador: mantém uma linha de produção, cuida das rotas de animais, planta mudas.

O bot é só mais um cliente: usa o mesmo protocolo de comandos, tem a própria chave e age com permissões delegadas pelo dono. Pode começar com regras simples e evoluir para agentes inteligentes sem mudar o núcleo.

### 2. Mercado em Bitcoin com sats e zaps

Autores de missões, mods e bots podem receber zaps; facilitadores e jogadores podem comprar conteúdo pago. Como missões e conteúdo já são publicados como eventos Nostr assinados pelo autor, o zap vai direto para quem criou, sem intermediário.

Duas regras inegociáveis. Primeira: o núcleo da simulação não conhece dinheiro — pagamentos acontecem fora dela e, no máximo, concedem ou retiram permissões. Segunda: em sessões de curso e terapia, e em qualquer servidor que se declare competitivo, ninguém compra vantagem; quem pagou joga com as mesmas regras de quem não pagou. Servidores abertos podem escolher aceitar serviços pagos (ver item 5). Aspectos regulatórios e fiscais de pagamentos em Bitcoin no Brasil precisam de análise antes do lançamento.

### 3. Mods e criação de novos elementos

Comunidade criando itens, máquinas, receitas, eras e missões, que podem ser gratuitos ou vendidos no mercado. Um mod é um pacote de dados mais roteiros, assinado pelo autor, com o arquivo guardado num armazenamento compatível com Nostr e publicado com o hash do conteúdo. Os roteiros rodam numa caixa de areia: sem acesso a rede ou disco, determinísticos e com orçamento de CPU por tick, para não quebrar o orçamento de desempenho.

### 5. Mercado de ativos e serviços recorrentes

Alguns ativos não se compram uma vez: alugam-se enquanto trabalham. Um robô trabalhador, um bot vigia, um território arrendado, um mentor de plantão. Quem contrata paga sats periodicamente, e o ativo trabalha para ele num servidor multiplayer onde há outros participantes.

**Como funciona.** O dono de um bot publica uma oferta (o que faz, preço por período, reputação). O contratante aceita; ambos assinam o contrato; os sats fluem por período, por exemplo por minuto ou por hora. Enquanto o pagamento flui, o bot recebe permissões delegadas sobre o território ou as máquinas do contratante; quando o pagamento para, as permissões caem sozinhas e o bot para. Ninguém fica exposto a calote grande: no máximo um período.

**Onde o bot roda.** Dois modelos possíveis, e os dois cabem:

- **Na infraestrutura do dono do bot,** como um cliente qualquer, enviando comandos assinados. O host não precisa confiar no código do bot: só valida os comandos, como faz com qualquer jogador.
- **Dentro do servidor, em caixa de areia,** como no Screeps. O host cobra pelo processamento, e o bot funciona mesmo que o dono esteja desconectado.

**Auditoria.** Com ações assinadas e simulação determinística, o contratante pode auditar o que o bot de fato fez com o seu território.

**Riscos e salvaguardas.**

- **Abuso.** Um bot só age dentro das permissões delegadas; tudo o que faz fica registrado e assinado, e o host pode banir.
- **Pagar para vencer.** Onde houver competição ou avaliação, isso desequilibra. Por isso cada servidor declara se aceita serviços pagos, e sessões de curso e terapia nunca aceitam.
- **Microtransações em sats.** Exigem uma solução de pagamento por período confiável; Lightning e ecash (Cashu) são candidatos.

### 6. Mundos e visualização em 3D

O Davi imagina algo mais próximo de Roblox e Minecraft, com visualização em 3D. O MVP continua em 2D, porque roda em qualquer máquina (inclusive notebooks de oficina) e é mais barato de manter. O 3D fica para depois, em dois degraus:

- **Cliente 3D sobre o mesmo mundo.** Como a simulação não sabe nada de gráficos, um cliente 3D (por exemplo Bevy ou Three.js) pode desenhar o mesmo mundo em grade com volume, câmera livre e terreno em relevo. Custo moderado: só o cliente muda. Um passo intermediário barato é a visão isométrica (2,5D), que dá sensação de profundidade sem o peso do 3D.
- **Mundo 3D de verdade, em blocos (voxels).** Construir em altura, cavar, empilhar, como no Minecraft. Isso muda o modelo do mundo (células viram colunas ou chunks 3D), multiplica memória e tráfego e exige física e colisão em 3D. É um mundo plugável novo, não uma troca de tela.

Gancho para o MVP: não assumir em lugar nenhum fora do pacote do mundo que o mapa é plano. As coordenadas e os chunks no protocolo devem admitir uma terceira dimensão mais tarde, mesmo que o primeiro mundo use só uma camada.

## Ideias do Davi: um mundo de todos

O Davi achou o projeto parecido demais com o Factorio e propôs abri-lo na direção de Roblox e Minecraft: muita gente jogando, cada um criando os próprios mundos e jogos. O planeta industrial e ecológico vira só o primeiro mundo, não o projeto inteiro.

**Nome e mascote (ideia do Davi).** Candidato a nome: **DuckGR** (pronuncia-se "Duck Gê-Erre"), para dar graça ao jogo. Mascote: um pato que vai na frente mostrando o caminho; ao longo do jogo aparecem patos com adesivo de Bitcoin. A verificar: disponibilidade no GitHub, domínio e INPI, e distância do jogo existente "Duck Game". Para a plataforma, outros candidatos levantados: Mutirão, Quintal, Canteiro, Tabuleiro.

**O jogo em que todo mundo é ADM (ideia do Davi).** Todo mundo pode fazer quase tudo: colocar qualquer bloco, construir o que quiser. A exceção é destruir: cada pessoa só destrói os blocos que ela mesma colocou e não pode modificar os blocos dos outros. Esse jogo seria o primeiro a aparecer na tela inicial, numa lista com outros jogos e mundos.

**Modos de convivência.**

- **Meu mundo:** a pessoa cria um mundo só dela.
- **Pacífico:** a pessoa convida quem quiser; ninguém destrói o que é dos outros.
- **Convivência:** mundo online com todo mundo, onde as pessoas podem formar grupos.
- **Guerra:** grupos podem conquistar as cidades uns dos outros.

**Regras definidas na entrada.** A guerra não é um convite no meio da partida: é uma regra do mundo. Quem cria o mundo define se ali tem guerra, e quem entra vê as regras antes e aceita ao entrar. Quem não quer guerra escolhe um mundo pacífico.

**Menu Paz ou Guerra (ideia do Davi).** Inspirado no menu de Blox Fruits, em que o jogador escolhe entre marinheiro e pirata. Aqui o jogador escolhe Paz ou Guerra, e a escolha decide o mundo: todos que escolhem Paz ficam juntos no **mundo de Paz**; todos que escolhem Guerra ficam juntos no **mundo de Guerra**.

**Denúncia no mundo de paz (variação).** Num mundo de paz, quem destruir ou atrapalhar os outros pode ser denunciado e perder reputação. Como a regra de propriedade já impede destruir o bloco alheio, a denúncia serve sobretudo para o que as regras não bloqueiam: cercar a casa de alguém, fechar caminhos, encher o mapa de lixo, construções ofensivas, assédio no chat. A prova vem do log de comandos assinados: quem denuncia aponta o trecho, e qualquer um pode reexecutá-lo e verificar. Punições possíveis, da mais leve à mais grave: aviso, perda de reputação no papel de jogador, expulsão daquele mundo. Cuidado: denúncia também pode virar arma (grupos denunciando alguém em massa), então o peso de cada denúncia deve depender da reputação de quem denuncia e de evidência verificável, não do número de votos.

**Tentativa de quebrar o bloco alheio (regras do Davi).** No mundo de Paz: na primeira tentativa, aparece o aviso "você vai ser denunciado"; na segunda, a pessoa é expulsa do jogo, precisa entrar de novo, e a conta fica suspensa por um dia. No mundo de Guerra: tentar quebrar o bloco de outro jogador começa uma guerra automaticamente. Como os dois grupos ficam em mundos separados, ninguém da Guerra alcança quem escolheu Paz.

**Sem dono central.** Diferente do Roblox, o jogo não pertence a uma empresa nem roda num servidor único. Cada pessoa pode hospedar o próprio servidor, e os jogadores se conectam aos computadores uns dos outros. Quem cria um mundo escolhe quem vai hospedá-lo e combina as regras com esse host. Isso exige confiança: o servidor pode cair. A plataforma registra o histórico (se o host mantém o servidor no ar, se a pessoa é boa parceira de construção) e esse histórico vira reputação, ligando a ideia ao sistema de recibos e reputação por papel do backlog.

**Impacto no motor.** A regra de "cada um destrói só o que colocou" sai de graça do modelo de comandos com autor: todo bloco guarda quem o colocou, e o servidor valida a permissão. Os modos são configurações declaradas do mundo, do mesmo jeito que o servidor declara se aceita serviços pagos.

**Modos por idade (ideia do Davi).** Ao criar a conta, a pessoa informa a data de nascimento e entra automaticamente num modo, sem poder escolher: **Crianças** (3 a 11 anos), **Adolescentes** (12 a 17) e **Adultos** (18 ou mais). Só adultos mexem em dinheiro; para menores, compras e vendas dependem da conta de um responsável. A definir: o que muda em cada modo (chat, mundos de guerra, com quem pode jogar) e como verificar a idade de forma mais séria do que só digitar a data, sobretudo para pagamentos. Referências a respeitar: ECA, ECA Digital (Lei 15.211/2025) e LGPD para dados de crianças e adolescentes.

**Conquistas e badges (ideia do Davi).** Gamificação para motivar o engajamento: badges e recompensas por tempo de participação (um mês, um ano) e por desafios cumpridos. Como cada mundo define suas regras, as conquistas também podem ser declaradas pelo criador do mundo; badges podem ser publicados como eventos Nostr e servir de reputação leve. Cuidado de desenho: premiar o que a pessoa fez (desafios, construções, ajudar outros), e não sequências diárias obrigatórias que punem quem falta, sobretudo nos modos Crianças e Adolescentes.

**Selos de tempo de casa (Davi).** Contam o tempo desde que a pessoa entrou, sem exigir presença diária (pode faltar dias). Marcos: 1 mês (selo + item para a skin), 6 meses, 1 ano, 2 anos, 4 anos, e 10 anos: **selo de diamante** e uma skin completamente nova.

**Princípio: regras de recompensa são do mundo, não da plataforma (Rodrigo).** Se uma armadura dá força, se uma skin é só visual ou também protege, se prêmios podem ser vendidos ou transferidos, quanto tempo leva cada selo: tudo isso é regra do pacote de cada mundo, nunca fixada na plataforma. A plataforma oferece só os mecanismos (itens, componentes, prêmios assinados, permissões de venda ou transferência); cada criador decide como usá-los. As regras abaixo (selos do Davi, armaduras, intransferibilidade) são exemplos de regras de mundos específicos, não da plataforma.

**Recompensas cosméticas (Rodrigo e Davi).** A recompensa não é só o selo: itens de skin e skins inteiras também são prêmios. Regra: recompensas são cosméticas e não dão vantagem no jogo. Numa rede descentralizada, cada recompensa precisa ser assinada por quem a concede (o servidor ou o criador do mundo), para que outros servidores possam verificar que é legítima e exibi-la. Skins de recompensa podem ser intransferíveis, para que um selo de dez anos não possa ser comprado no mercado.

**Pendência: armadura do jogador (Rodrigo).** Como no Factorio, o personagem tem armadura que evolui com a pesquisa e as eras, tornando-o mais resistente em combate. Precisa ser tratada no desenho. No modelo do motor, a armadura é um elemento equipável com componentes (vida, resistência, encaixes para módulos), definido em dados pelo pacote do mundo. Diferente da skin: armadura dá vantagem, então é conquistada no jogo e nunca vendida no mercado.

**Armadura como prêmio de tempo de casa (ideia do Davi, em aberto).** Aos 4 anos, metade de uma armadura mediana; aos 10 anos, a outra metade; depois disso, uma armadura excelente, forte o bastante para proteger um país inteiro. Tensão com a regra de que prêmios não dão vantagem: veteranos ficariam fortes demais contra novatos, sobretudo no mundo de Guerra. Opções a discutir: (a) a armadura de prêmio tem aparência única, mas a mesma força da armadura que se pesquisa no jogo; (b) dá força real, mas só em mundos cujo criador permita prêmios com vantagem; (c) dá poderes de proteção que não servem para atacar (proteger a própria cidade, os amigos, o país).

**Regra do mundo do Davi: ataque ou defesa.** Aos 4 anos, o jogador ganha a primeira metade da armadura (mediana) e escolhe se ela é de ataque ou de defesa. Aos 10 anos, ao completar a armadura, escolhe de novo. As duas metades podem ser iguais (só ataque ou só defesa) ou diferentes (metade de cada).

**Regra do mundo do Davi: receber ou doar.** Ao ganhar uma recompensa, o jogador escolhe entre receber para si ou doar a outra pessoa. No mundo do Davi, prêmios podem ser doados (mas não vendidos). A definir: para quem se pode doar (qualquer jogador, um amigo, um novato) e se a doação aparece no perfil de quem doou.

**O que muda por modo (Davi).** Crianças: sem chat. Adolescentes: têm chat e outras funções que pedem mais responsabilidade. Adultos: tudo, incluindo dinheiro. A detalhar: quais outras funções ficam só para adolescentes e adultos.

**Comunicação das crianças (Davi aprovou).** Sem chat livre: só frases prontas, já formadas (por exemplo "vamos construir juntos?", "obrigado!"), escolhidas de uma lista fixa.

**Proteção de crianças: para estudar com calma (Rodrigo).** Problema: a rede não pede identidade, então a data de nascimento digitada não protege, porque um atacante pode se declarar criança. Caminhos levantados:

- **Servidores de comunidades conhecidas (principal ideia do Rodrigo).** A descentralização vira proteção: uma escola, uma família ou um grupo cria o próprio servidor e controla quem entra. O pai deixa o filho entrar porque conhece quem está do outro lado, e o abusador nem chega perto.
- **Servidores que exigem identidade.** Alguns servidores podem exigir identidade verificada e se declarar seguros para crianças. Proposta a avaliar: o modo Crianças só existe em servidores assim, com a conta da criança criada por um responsável verificado.
- **Mensagens guardadas para auditoria.** No esquema do gestor de portfólio, quem entra num servidor infantil aceita que as mensagens fiquem guardadas para auditoria posterior, como prova contra abusadores. Limites: age depois do dano, e o aliciador tende a levar a conversa para fora do jogo. Guardar mensagens de crianças exige minimização, prazo curto, criptografia e acesso controlado (LGPD).
- **Desenho que impede o contato.** Para crianças: sem mensagem privada, presentes, pagamentos ou links; só frases prontas.

**Modo seguro por servidor (Rodrigo).** O modo Criança pode ser uma configuração do servidor, e não só da conta: num servidor configurado como infantil, só funcionam mensagens pré-determinadas para todos, de qualquer idade. Isso protege sem depender de saber a idade de ninguém. A escola guarda os dados dos alunos no seu gestor de portfólio, criptografados, para segurança, e os apaga depois de um prazo.

**Responsabilidade numa rede descentralizada (posição do Rodrigo, a validar).** O projeto entrega a plataforma como código aberto e não opera os servidores nem o jogo; cada pessoa ou instituição que hospeda ou entra assume a responsabilidade pelo que faz. Pontos a validar com advogado: se o projeto mantiver relays, servidores padrão, app nas lojas ou mercado com taxa, pode passar a ser visto como provedor; e o ECA Digital pode alcançar produtos de acesso provável por crianças. Mesmo onde a lei não obriga, padrões seguros por omissão (servidor novo nasce no modo mais restrito) reduzem o risco.

**Decisão: modelo Mastodon (Rodrigo).** O projeto não mantém servidor padrão nem relay próprio. Quem quiser jogar usa um servidor e um relay mantidos pela comunidade, ou coloca o seu no ar. Rodrigo e Davi desenvolvem e publicam o código; não operam a rede.

**Caminho gradual (Rodrigo).** Primeiro, jogo solo: o servidor roda na própria máquina, sem Nostr. Depois, multiplayer no estilo Factorio: uma pessoa sobe o servidor e publica o endereço, as outras se conectam, e o host guarda os arquivos do mundo. Nostr e WebRTC entram só depois, para descoberta de sessões e para atravessar roteadores sem abrir portas.

**Pacote de hospedagem (Rodrigo).** Servidor do jogo, relay Nostr, STUN e TURN distribuídos juntos numa imagem pronta (VM ou contêiner). Quem quiser hospeda com poucos passos e pode cobrar pelo serviço, o que liga o pacote ao mercado de serviços recorrentes. STUN público (Google, Cloudflare) fica como padrão no cliente; TURN é sempre da comunidade.

A verificar com advogado: obrigações do ECA Digital (Lei 15.211/2025) quanto à verificação de idade e ao controle parental; quem responde legalmente numa rede descentralizada (a plataforma ou o host do servidor); obrigação de denunciar abuso às autoridades.

### Protótipo rascunho: "Paz ou Guerra"

Primeiro rascunho jogável, para testar as regras do Davi antes do motor de verdade. Um único arquivo HTML que abre no navegador, sem servidor, para ser feito no chat de texto ou no Claude Code.

- **Mapa:** grade 2D vista de cima, por exemplo 40×30 quadradinhos.
- **Jogadores:** dois ou três na mesma tela e no mesmo teclado (por exemplo WASD, setas e IJKL), cada um com uma cor.
- **Menu inicial:** cada jogador escolhe Paz ou Guerra.
- **Construir:** cada jogador coloca blocos da sua cor; todo bloco guarda quem o colocou.
- **Quebrar:** cada um quebra os próprios blocos livremente.
- **Regras de convivência:** jogador de Paz que tenta quebrar o bloco de outro recebe o aviso "você vai ser denunciado"; na segunda vez, é expulso da partida (a suspensão de um dia fica simulada com uma mensagem). Entre dois jogadores de Guerra, quebrar o bloco do outro inicia uma guerra, e a partir daí os dois podem quebrar os blocos um do outro.
- **Fora do rascunho:** rede, contas, reputação e 3D. No rascunho, o menu leva a um de dois mapas: o mapa de Paz ou o mapa de Guerra.

O que o rascunho deve responder: as regras são fáceis de entender? O aviso basta para conter quem atrapalha? A guerra automática fica divertida ou injusta?

### Ganchos que o MVP precisa deixar prontos

| Extensão | Gancho no MVP | Custo agora |
| --- | --- | --- |
| Bots | Comandos como única entrada, com autor; protocolo de cliente documentado e versionado; permissões por tipo de comando (dono, convidado, bot) | Baixo |
| Bots vigias | Servidor capaz de rodar o mundo sem ninguém conectado (já previsto em Onde o mundo roda) | Já está no plano |
| Mods | Todo conteúdo em dados, com IDs em namespace (`core:placa-ferro`, `autor:item`) e versão do formato; nada de item fixo no código | Baixo, mas caro de mudar depois |
| Mods e replay | Save e replay registram o hash de cada pacote de conteúdo usado, para que a partida possa ser reproduzida | Muito baixo |
| Mercado | Missões e conteúdo publicados como eventos assinados pela chave Nostr do autor | Quase zero, já previsto |
| Mercado | Pagamento fora do núcleo: o `vale-sim` não conhece dinheiro | Zero — é uma regra a manter |
| Mentoria | Papel "mentor" separado do facilitador, que vê o time e conduz o debriefing sem jogar; fim de sessão gera um registro que pode virar recibo assinado | Baixo |
| Motor de mundos | Poluição, água e ecologia definidas como campos e regras de conteúdo, não como código fixo do núcleo; transporte só pelas três primitivas (segmento, rota, rede de fluxo) | Médio, mas evita reescrita |
| Serviços recorrentes | Permissões delegadas com prazo, revogáveis por um serviço externo ao núcleo; servidor declara se aceita serviços pagos | Baixo |

## Visão de longo prazo: de jogo a motor de mundos

O destino possível é um motor de jogos de automação em que materiais, regras, máquinas e a própria física do mundo são plugáveis — e vendáveis no mercado. Um deserto onde a água é tudo, um planeta gasoso sem chão, um mundo submarino com pressão e correntes. A estratégia é não começar pelo motor: fazer um jogo real, depois um segundo mundo bem diferente, e só então extrair o que os dois têm em comum. Motor genérico desenhado antes do primeiro jogo costuma generalizar as coisas erradas.

Há precedente forte: o próprio Factorio separa a definição de itens, máquinas e receitas (dados) dos roteiros que rodam durante a partida, e essa separação sustenta uma das maiores comunidades de mods do gênero.

### O vocabulário do motor

O que já desenhamos para o nosso jogo se generaliza em poucas peças. Um mundo novo é uma combinação delas, não código novo no núcleo.

| Peça | O que é | No nosso jogo | Em outro mundo |
| --- | --- | --- | --- |
| Entidade | Algo no mapa com inventário e estado | Fornalha, árvore, ninho | Coletor de gás, recife artificial |
| Receita | Entradas, saídas, tempo e condições do ambiente | Minério vira placa | Algas viram oxigênio, só com luz |
| Segmento | Transporte contínuo por trechos | Esteira | Corrente marinha natural |
| Rota | Agentes indo e voltando entre estações | Carroça, trem | Dirigível, cardume domesticado |
| Rede de fluxo | Vazão calculada por rede conectada | Água, aqueduto | Gás, pressão hidráulica |
| Campo | Valor por chunk que se espalha, com fontes e sumidouros | Poluição, saúde do solo | Oxigênio, pressão, radiação, umidade |
| Agente | Ser com comportamento próprio | Besouros, bots | Fauna abissal, colonos |
| Era e árvore | Progressão por pesquisa | Carvão até espacial | Qualquer linha do tempo |
| Missão | Metas e condições verificáveis | Primeiro aço | Sobreviver à primeira tempestade |
| Rede de sinais (futura) | Vetores de inteiros trocados entre elementos conectados, em duas fases por tick | Automação com lógica, como os circuitos do Factorio | Sensores de pressão que abrem válvulas |
| Despachante (futuro) | Agentes que atendem pedidos e ofertas dentro de uma área | Carroças de uma vila, robôs logísticos | Drones de carga entre plataformas |
| Superfície (futura) | Um mapa inteiro, acima dos chunks; várias por mundo, ligadas por portais | A Terra e a órbita | Camadas de nuvem, o mundo de outro host |
| Transferência e modificador (futuros) | Componentes: mover itens entre inventários vizinhos; alterar um processo, local ou em área | Carregador; adubo que acelera a fazenda | Luz artificial que acelera as algas |

### Exemplos de mundos

- **Planeta desértico:** água como recurso central, umidade como campo, tempestades de areia que soterram estradas, e um plano de reverdecer que só rende muitas eras depois.
- **Planeta gasoso:** não há chão; tudo é construído em plataformas flutuantes, a coleta é atmosférica e a flutuação substitui o terreno.
- **Mundo submarino:** pressão cresce com a profundidade, correntes são esteiras naturais, oxigênio é produzido e não está no ar, e a luz some à medida que se desce.

Todos os mundos do projeto, nossos ou da comunidade, são originais, sem nomes, personagens ou marcas de terceiros. Como autores externos poderão publicar e vender pacotes, a plataforma precisa de uma política de denúncia e remoção para conteúdo que viole direitos de terceiros.

### Como um mundo vira um pacote

- **Dados primeiro.** Itens, receitas, campos, eras e missões em arquivos declarativos — o que cobre a maior parte de um mundo novo.
- **Comportamento em caixa de areia.** Quando dados não bastam (regras de difusão diferentes, fauna com comportamento próprio), o autor escreve módulos que rodam isolados, determinísticos e com limite de processamento por tick. Candidatos: uma linguagem de roteiro embutida, como Rhai, para o simples, e módulos WebAssembly interpretados com medição de consumo, para o pesado e para autores de outras linguagens.
- **Pacote assinado e com dependências.** Um mundo declara de que pacotes depende e em que versão; é assinado pelo autor, publicado com o hash do conteúdo e vendido pelo mercado descrito no backlog.

### Como chegar lá

1. **Jogo 1, a Terra.** O jogo deste documento, já escrito em cima das peças acima, mesmo sem expor nada a terceiros.
2. **Jogo 2, um mundo bem diferente.** Feito por nós — de preferência o submarino ou o gasoso, que quebram mais premissas (chão, ar, gravidade). Tudo o que precisar mudar no núcleo para acomodá-lo mostra onde a abstração estava errada.
3. **Extração do motor.** O que os dois jogos têm em comum vira o núcleo; o resto vira pacote.
4. **SDK público e mercado.** Documentação, ferramentas de autor e venda de mundos.

**A decisão que vale tomar já:** o núcleo não pode saber que existe Terra. Poluição, água e ecologia devem ser conteúdo (campos e regras registrados pelo pacote do jogo 1), não código fixo da simulação. Custa pouco agora e é exatamente o que faria o jogo 2 exigir uma reescrita, se esquecido.

## Modelo do mundo: células, camadas e elementos

A célula é a unidade mínima de espaço, mas não deve ser um objeto que "contém tudo". O modelo proposto é uma pilha de camadas fixas, cada uma com no máximo um valor por célula; o que se move fica fora da célula; e o que se espalha (poluição, oxigênio) fica numa grade mais grossa. O mundo — terrestre, gasoso ou submarino — define o que cada camada pode conter; o núcleo só sabe empilhar e combinar.

&#91;embedded content: célula · 6 camadas, agentes e campos fora dela\]

As três camadas de baixo vêm do mundo; as três de cima, do jogador. Agentes e campos não moram na célula: os agentes consultam a máscara para saber por onde podem ir.

### As camadas da célula

| Camada | Pergunta que responde | Quantos por célula | Exemplos na Terra |
| --- | --- | --- | --- |
| Depósito | O que há embaixo para extrair? | Zero ou um, com quantidade finita | Minério de ferro, carvão, aquífero, petróleo |
| Substrato | O que a célula é? | Exatamente um | Solo, rocha, areia, água rasa, água funda |
| Cobertura | O que cresce ou cobre a célula naturalmente? | Zero ou um, com estado de crescimento | Grama, floresta, plantação, mangue |
| Piso | Que base construída existe? | Zero ou um | Trilha, estrada de terra, asfalto, trilho, fundação |
| Ocupante | Que estrutura está aqui? | Zero ou uma entidade (que pode ocupar várias células) | Fornalha, esteira, muro, torre agrícola |
| Itens no chão | O que foi largado aqui? | Lista curta, rara | Minério jogado no mato |

Três regras simplificam muito:

- **Piso e ocupante convivem; ocupantes não.** Uma esteira fica sobre uma fundação, um trem sobre um trilho; mas duas máquinas nunca dividem a célula.
- **Árvores e plantações são cobertura, não entidades.** Uma floresta tem milhões de árvores; como cobertura, cada uma custa poucos bytes e cresce por rodízio. Só vira entidade o que tem inventário ou comportamento próprio.
- **Recurso no depósito não é item.** O depósito guarda uma quantidade; só quando uma mineradora extrai é que nasce um item, que passa a existir em inventários, esteiras e rotas.

### O que passa pela célula

Em vez de perguntar "esta célula é caminhável?", cada mundo declara suas **classes de passagem** — por exemplo pedestre, veículo terrestre, embarcação, flutuante, cada fluido — e cada camada diz o que permite, bloqueia ou modifica. A combinação vira uma **máscara de passagem** por célula, calculada só quando alguma camada muda, e é ela que a busca de rotas, a construção e os fluxos consultam.

| Na célula | Pedestre | Veículo | Embarcação | Água em rede |
| --- | --- | --- | --- | --- |
| Solo com grama | Passa | Passa devagar | — | — |
| Solo + estrada de asfalto | Passa | Passa rápido | — | — |
| Floresta densa | Passa devagar | Bloqueia | — | — |
| Água funda | Bloqueia | Bloqueia | Passa | Passa (é fonte) |
| Qualquer + fornalha | Bloqueia | Bloqueia | Bloqueia | — |
| Qualquer + aqueduto | Bloqueia | Bloqueia | — | Passa |

As mesmas camadas dizem também **o que se pode construir**: cada substrato e cada piso aceitam certas fundações (fornalha exige solo ou fundação; nada se constrói em água funda sem plataforma) e contribuem para os **campos** (floresta absorve poluição; asfalto não).

### O que fica fora da célula

- **Agentes móveis** — pessoas, animais, besouros, veículos — têm posição livre, mais fina que a célula, e são indexados por chunk. A célula não sabe quem está passando por ela; quem está passando consulta a máscara.
- **Campos** — poluição, umidade, saúde do solo, oxigênio, pressão — ficam numa grade por chunk, com resolução definida pelo mundo. Cada campo tem regra de difusão entre vizinhos, fontes e sumidouros. Um campo pode ser escalar (um valor) ou vetorial (direção e força, como uma corrente ou um vento).

### Elementos: protótipo mais componentes

Tudo o que é colocado no mundo (máquina, esteira, carroça, ninho) é um **elemento**, definido em dados como um protótipo que combina componentes:

| Componente | O que dá ao elemento |
| --- | --- |
| Pegada | Forma e tamanho em células, rotações possíveis, o que exige da célula |
| Inventário | Espaços para itens |
| Processo | Receitas que executa, com tempo e condições do ambiente |
| Energia | Consome ou produz, de que tipo (lenha, vapor, elétrica) |
| Porta de transporte | Conecta a um segmento, rota ou rede de fluxo |
| Emissor de campo | Emite ou absorve poluição, umidade, oxigênio |
| Vida | Pontos de dano, resistência a ataque |
| Comportamento | Para agentes: o que decide fazer e quando |

Uma fornalha a carvão é: pegada 2×2 em solo ou fundação + inventário + processo (minério vira placa) + energia de combustível + emissor de poluição + vida. Nenhuma linha de código fala em "fornalha" — o núcleo só executa componentes. É isso que torna mundos novos possíveis sem mexer no núcleo.

### O mesmo modelo em três mundos

| Camada | Terrestre | Gasoso | Submarino |
| --- | --- | --- | --- |
| Depósito | Minério, carvão, aquífero | Gases úteis por tipo de nuvem, renováveis devagar | Nódulos minerais, fontes hidrotermais |
| Substrato | Solo, rocha, água | Nuvem densa, nuvem leve, tempestade — nada sólido | Areia do fundo, rocha, fossa; água em toda parte |
| Cobertura | Grama, floresta, plantação | Colônias flutuantes de microrganismos | Algas, coral |
| Piso | Estrada, trilho, fundação | Plataforma flutuante, obrigatória para construir | Fundação ancorada |
| Passagem | Pedestre em terra, barco na água | Só flutuantes e voadores; gás flui livre | Todos nadam; correntes empurram |
| Campos | Poluição, solo, umidade | Pressão, temperatura, vento (vetorial) | Oxigênio, pressão, luz, corrente (vetorial) |

O teste passa sem mudar a estrutura: o que muda são os tipos que o mundo registra em cada camada, as classes de passagem e os campos. Dois pontos ficam em aberto, e os dois tocam na altitude, que deixamos de fora: mundos gasosos e submarinos pedem profundidade. A saída para começar é tratar a profundidade como propriedade do substrato ("nuvem alta", "fundo raso", "fossa") e deixar camadas verticais de verdade para quando o segundo mundo exigir.

### Como isso fica na memória

Por chunk de 32×32 células, cada camada é um vetor contíguo: substrato, cobertura e piso como identificadores de 16 bits, crescimento em 8 bits, ocupante como identificador de entidade de 32 bits, máscara de passagem em 16 ou 32 bits. Depósitos e itens no chão, que a maioria das células não tem, ficam em mapas esparsos. Isso dá na ordem de 15 KB por chunk — um mapa de missão de 256×256 células, com 64 chunks, cabe em cerca de 1 MB antes das entidades.

### Declarativo ou código?

A resposta curta: a maior parte pode ser declarativa, mas não tudo, e o projeto deve tratar isso como uma escada. Cada degrau acima dá mais poder ao autor e custa mais em desempenho, segurança e determinismo; só se sobe quando o degrau de baixo comprovadamente não dá conta.

| Degrau | O que cobre | Exemplo |
| --- | --- | --- |
| 1. Dados puros | Tipos de cada camada, protótipos de elementos, receitas, classes de passagem, regras de passagem em tabela, campos com difusão parametrizada | "Floresta: pedestre devagar, veículo bloqueia, absorve poluição 2 por tick" |
| 2. Expressões | Condições e fórmulas curtas, avaliadas pelo núcleo com aritmética inteira | "Rendimento da fazenda = base × saúde do solo"; "alga só cresce com luz acima de 30" |
| 3. Roteiros em caixa de areia | Comportamento de fauna e bots, eventos de missão, regras exóticas que não cabem em fórmula | "Ninho se afasta quando a poluição do chunk fica baixa por 5 minutos" |
| 4. Código do núcleo, em Rust | Apenas as primitivas: segmento, rota, rede de fluxo, campo, processo, agente, máscara | Uma primitiva nova só quando o segundo mundo provar que falta |

Um esboço de como a fornalha e a floresta poderiam ser escritas no degrau 1 (formato ilustrativo):

```ron
Elemento(
  id: "terra:fornalha-carvao",
  pegada: (2, 2), exige: ["solo", "fundacao"],
  inventario: (entrada: 2, saida: 1, combustivel: 1),
  processo: ["terra:placa-ferro"],
  energia: Combustivel(categoria: "quimico"),
  emite: { "terra:poluicao": 4 },
  vida: 200,
)

Cobertura(
  id: "terra:floresta",
  passagem: { "pedestre": Devagar, "veiculo": Bloqueia },
  absorve: { "terra:poluicao": 2 },
  crescimento: (estagios: 4, ticks_por_estagio: 18000),
)
```

Um ponto de arquitetura importante: o que o autor escreve não é o que roda. No carregamento, o pacote é validado contra um esquema e **compilado** para tabelas densas com identificadores numéricos e máscaras pré-calculadas; texto como `"terra:floresta"` só existe na autoria. Assim, ser declarativo não custa desempenho em tempo de jogo.

**Perguntas em aberto:**

- [ ] Formato dos dados: RON, TOML ou outro? RON casa bem com Rust; TOML é mais familiar para autores.
- [ ] Linguagem de expressões do degrau 2: criar uma mínima própria ou adotar uma existente que garanta determinismo?
- [ ] Roteiros do degrau 3: Rhai, WebAssembly interpretado, ou os dois?
- [ ] Validação de pacotes: esquema mais testes automáticos ("esta fornalha produz placa em N ticks") rodados antes de publicar.
- [ ] Recarga ao vivo para autores: editar o arquivo e ver o mundo mudar sem reiniciar.
- [ ] Quanto o jogo 1 precisa do degrau 3? A aposta é que quase nada além da fauna e das missões.

### O Factorio como teste de estresse do modelo

O teste: pegar os sistemas mais complexos do Factorio e ver se cabem nas nossas peças. O resultado é bom — a base aguenta, e em dois casos o próprio Factorio evoluiu na mesma direção que escolhemos —, mas faltam quatro peças, listadas no fim.

O wiki de que você lembra é um bom indicador de que o caminho é este. As páginas de tecnologia mostram tipo de protótipo, nome interno, tecnologias necessárias, o que ela libera e seus efeitos ([exemplo](https://wiki.factorio.com/Logistic_robotics_%28research%29)) — ou seja, o wiki espelha os dados dos protótipos. E o jogo inteiro é definido por protótipos de entidade, item, receita, tecnologia, tile, fluido, superfície e qualidade, entre outros ([API do Factorio](https://lua-api.factorio.com/latest/Classes.html)). Como no nosso modelo tudo também é dado, uma enciclopédia com entradas, saídas e a árvore de dependências pode ser **gerada automaticamente dos pacotes** — inclusive para os mods da comunidade.

| Sistema do Factorio | Como funciona | Cabe no nosso modelo? |
| --- | --- | --- |
| Tiles, chunks e poluição | Poluição é uma "nuvem" por chunk de 32×32, atualizada a cada 64 ticks; acima de 15 unidades se espalha para os quatro vizinhos; cada tile e cada árvore absorvem um pouco ([wiki](https://wiki.factorio.com/Pollution)) | Sim: substrato + cobertura + campo por chunk em rodízio — é exatamente o nosso desenho |
| Esteiras | Trechos contínuos fundidos em linhas de transporte | Sim: primitiva segmento |
| Fluidos | Na versão 2.0, canos e tanques foram fundidos em segmentos tratados como uma unidade, trocando realismo por previsibilidade ([FFF #416](https://cf-www.factorio.com/blog/post/fff-416)) | Sim: primitiva rede de fluxo — a mesma escolha que fizemos para a água |
| Pesquisa | Tecnologia com custo, pré-requisitos e efeitos | Sim: era e árvore |
| Blueprints | JSON com a lista de entidades (nome, posição, direção, configuração), comprimido e codificado em texto para compartilhar ([wiki](https://wiki.factorio.com/Blueprint_string_format)) | Sim, e de graça: protótipo + deslocamento + rotação + configuração. Pode virar evento Nostr assinado, compartilhável e vendável |
| Insersores | Braço que move itens de um inventário para outro | Quase: falta um componente de transferência entre inventários vizinhos |
| Rede de circuitos | Fios formam redes; cada rede tem um canal por item mais sinais virtuais; sinais se somam; combinadores calculam com um tick de atraso; esteiras, baús e insersores leem ou recebem condições ([wiki](https://wiki.factorio.com/Circuit_network), [combinadores](https://wiki.factorio.com/Tutorial:Combinator_Tutorial)) | Não ainda: é uma quarta rede, de sinais |
| Rede logística | Roboports definem a área coberta; baús ofertam, pedem ou guardam; robôs atendem os pedidos por prioridade ([wiki](https://wiki.factorio.com/Logistic_network)) | Não ainda: rota hoje é fixa; isto é um despachante que casa pedidos e ofertas dentro de uma área |
| Trens e sinais | Rotas sobre trilhos com trechos reservados por sinais | Quase: rota precisa de reserva de trechos |
| Superfícies | Vários mapas independentes no mesmo jogo | Não ainda: falta o nível acima do chunk |
| Módulos e beacons | Itens que modificam velocidade, consumo e produtividade de uma máquina, ou de todas numa área | Quase: falta um componente de modificador, local ou em área |

**O que acrescentar ao modelo, sem mudar o que já existe:**

1. **Rede de sinais** — quarta primitiva de rede, ao lado de segmento, rota e fluxo. Leva vetores de inteiros e é avaliada em duas fases por tick (ler tudo, depois escrever tudo), o que a mantém determinística. Elementos ganham um componente "porta de sinal"; um combinador é só um elemento cujo processo é lógico. É o que abre a automação avançada e, de quebra, é um ótimo instrumento didático de raciocínio lógico.
3. **Superfície** — o nível acima do chunk: vários mapas num mesmo mundo (outro planeta, o fundo do mar, o mundo de outro host), ligados por portais ou lançamentos.
4. **Componentes de transferência e de modificador** — o braço que move itens entre inventários vizinhos, e o efeito que altera um processo, numa máquina ou numa área.

Nenhuma delas precisa existir no MVP. O que importa agora é que o modelo tem lugar para elas sem reescrita.

## Decisões em aberto e riscos

### Decisões que dependem de você

- [ ] **Licença.** MIT/Apache-2.0 maximiza adoção (inclusive por consultorias); AGPL garante que versões hospedadas por terceiros continuem abertas. A arte pode ir em CC BY-SA em qualquer caso.
- [ ] **Sessões presenciais ou remotas?** Na mesma sala, a conversa acontece fora do jogo e o chat importa pouco; remoto, a comunicação restrita vira uma mecânica central.
- [ ] **Terapia e liderança na mesma ferramenta?** São públicos com cuidados diferentes. Sugiro começar por liderança e validar com um terapeuta parceiro antes de usar em contexto clínico.
- [ ] **Tema e nome.** Escolher o nome antes da fase 2 (os crates usam \`vale-\` como prefixo provisório).
- [ ] **Onde hospedar o multiplayer?** Servidor local no notebook do facilitador (presencial, sem internet), na nuvem (remoto) ou os dois desde o início. Recomendo começar pelo local: é o caso mais comum em oficina e não tem custo.
- [ ] **Identidade Nostr obrigatória ou opcional?** Recomendo opcional: participante de oficina ou terapia não deveria precisar criar conta nem gerenciar chave para jogar 45 minutos; a chave gerada na hora resolve, e quem já usa Nostr leva a sua.

### Riscos

| Risco | Sinal de alerta | Mitigação |
| --- | --- | --- |
| Virar um clone do Factorio e perder o foco pedagógico | Fases gastas em conteúdo sem nenhuma sessão de teste com grupo | Testar com um grupo real ao fim de cada fase a partir da 4 |
| Desempenho ruim no navegador ou em celular | Menos de 30 ticks/s no mapa de referência | Benchmark no CI desde a fase 1; mapas de missão pequenos |
| Determinismo quebrado sem ninguém notar | Replay diverge da sessão original | Teste de hash de estado em todo commit |
| Dados sensíveis de participantes | Métricas guardadas com nome em contexto terapêutico | Anônimo por padrão, apagar ao fim da sessão, consentimento explícito para guardar (LGPD) |
| Arte travar o projeto | Mecânicas prontas esperando sprites | Formas geométricas simples até a fase 4; arte depois |

## Fontes

- [Friday Facts #76 — multiplayer e lockstep](https://factorio.com/blog/post/fff-76)
- [Friday Facts #148 — otimização de esteiras e tubos](https://factorio.com/blog/post/fff-148)
- [Friday Facts #188 — dessincronização e lockstep determinístico](https://factorio.com/blog/post/fff-188)
- [Factorio Wiki — física das esteiras](https://wiki.factorio.com/Transport_belts/Physics)
- [Supercraft — restrições e quebra-cabeça de automação](https://supercraft.host/article/factorio-automation-constraints/)
- [Summer Engine — loop central do gênero](https://www.summerengine.com/templates/strategy/4x-strategy/factorio-style)
- [GamingOnLinux — Mindustry](https://www.gamingonlinux.com/2018/12/mindustry-an-open-source-sandbox-tower-defense-game-thats-a-little-like-factorio/)
- [shapez no GitHub](https://github.com/tobspr/shapez.io)
- [AlternativeTo — comparação de jogos do gênero](https://www.alternativeto.net/software/factorio/)
- [Bevy + WebGPU](https://bevy.org/news/bevy-webgpu/)
- [Factorio Wiki — árvores](https://wiki.factorio.com/Tree)
- [vmsplice — Nostr para apps peer-to-peer](https://blog.vmsplice.net/2023/09/how-nostr-could-enable-peer-to-peer-apps.html)
- [hashtree — sinalização WebRTC sobre Nostr em Rust](https://docs.rs/hashtree-cli/latest/src/hashtree_cli/webrtc/signaling.rs.html)
- [Castronova (2001) — Virtual Worlds: A First-Hand Account of Market and Society on the Cyberian Frontier](https://www.cesifo.org/DocDL/cesifo_wp618.pdf)
- [MCV — entrevista com o economista de EVE Online](https://mcvuk.com/?p=141487)
- [Deterministic Lockstep in Networked Games (artigo)](https://hdms.bsz-bw.de/files/7107/DeterministicLockstepInNetworkedGamesPaper.pdf)
- [Friday Facts #83 — ocultação de latência](https://factorio.com/blog/post/fff-83)
- [Friday Facts #302 — o megapacote do multiplayer](https://cf-www.factorio.com/blog/post/fff-302)
- [Friday Facts #416 — Fluids 2.0](https://cf-www.factorio.com/blog/post/fff-416)
- [Valve — Source Multiplayer Networking](https://developer.valvesoftware.com/wiki/Source_Multiplayer_Networking)
- [Interest Management for Distributed Virtual Environments: A Survey (ACM, 2014)](https://researcher.ibm.com/publications/interest-management-for-distributed-virtual-environments-a-survey)
- [Screeps — arquitetura do servidor](https://docs.screeps.com/architecture.html)
- [Screeps — world shards](https://blog.screeps.com/2017/08/shards/)
- [EVE Online — Time Dilation](https://eveonline.com/article/introducing-time-dilation-tidi)
- [Factorio API — classes e protótipos](https://lua-api.factorio.com/latest/Classes.html)
- [Factorio Wiki — poluição](https://wiki.factorio.com/Pollution)
- [Factorio Wiki — rede de circuitos](https://wiki.factorio.com/Circuit_network)
- [Factorio Wiki — combinadores](https://wiki.factorio.com/Tutorial:Combinator_Tutorial)
- [Factorio Wiki — formato de blueprint](https://wiki.factorio.com/Blueprint_string_format)
- [Factorio Wiki — rede logística](https://wiki.factorio.com/Logistic_network)
