// Pacote do mundo "Terra Viva" (nome provisório): a primeira era do jogo
// industrial e ecológico. Só dados. O núcleo da plataforma não conhece
// nada daqui: árvores, minérios, fornalhas e poluição existem só neste arquivo.
//
// Unidades: tempos em segundos (o núcleo converte para ticks); poluição em
// centésimos de unidade por segundo, para tudo ficar em números inteiros.

export default {
  id: "terra-viva",
  nome: "Terra Viva",
  versao: "0.2.0",
  descricao: "Primeira era: carvão e vapor. Produza sem devastar a floresta.",

  mapa: {
    chunk: 8,
    tamanhos: {
      pequeno: { nome: "Pequeno", largura: 32, altura: 20 },
      medio: { nome: "Médio", largura: 40, altura: 24 },
      grande: { nome: "Grande", largura: 56, altura: 32 },
    },
    geracao: {
      substrato: "solo",
      cobertura: "grama",
      rios: [{ substrato: "agua", vau: "vau", vaus: 2, posicao: 0.2, largura: 2 }],
      manchas: [
        // floresta: manchas de árvores adultas e jovens
        { camada: "cobertura", tipos: { adulta: 3, jovem: 1 }, porArea: 1 / 55, raio: [2, 4], preenche: 75, escalaPor: "floresta" },
        // depósitos em lugares parecidos em todo mapa, para a partida ser justa
        { camada: "deposito", tipo: "ferro", em: [0.55, 0.3], raio: 2, qtd: [60, 120] },
        { camada: "deposito", tipo: "carvao", em: [0.42, 0.7], raio: 2, qtd: [60, 120] },
        { camada: "deposito", tipo: "pedra", em: [0.33, 0.28], raio: 1, qtd: [40, 80] },
        { camada: "deposito", tipo: "cobre", em: [0.78, 0.68], raio: 2, qtd: [50, 100] },
      ],
    },
  },

  // valores usados quando o jogador não muda nada (o jogo padrão)
  padrao: { semente: 2026, largura: 40, altura: 24, floresta: 1, recursosIniciais: 1 },

  // O jogador anda a pé por enquanto. Veículos, no futuro, mudam a velocidade.
  jogador: {
    nome: "Pato",
    passagem: "pedestre",       // classe de passagem usada pelas camadas
    celulasPorSegundo: 4,       // a pé
    alcance: 1,                 // age nas 8 células em volta
    minerarVezes: 5,            // quantas vezes "minerar" repete por ordem
    duracoes: { colher: 1, minerar: 0.5, plantar: 0.4, construir: 0.5, desmontar: 0.5, recolher: 0.2, abastecer: 0.2, fornecer: 0.2 },
  },

  substratos: {
    solo: { nome: "Solo", construir: true, plantar: true, aparencia: { cor: "#7a6446" } },
    vau: { nome: "Água rasa", construir: false, plantar: false, marcas: ["agua"], passagem: { pedestre: 3 }, aparencia: { cor: "#4f8fb8", forma: "agua" } },
    agua: { nome: "Água", construir: false, plantar: false, marcas: ["agua"], passagem: { pedestre: "bloqueia" }, aparencia: { cor: "#2f6f9e", forma: "agua" } },
  },

  depositos: {
    ferro: { nome: "Minério de ferro", item: "minerio-ferro", aparencia: { cor: "#a8b8cc" } },
    cobre: { nome: "Minério de cobre", item: "minerio-cobre", aparencia: { cor: "#e08a4a" } },
    carvao: { nome: "Carvão", item: "carvao", aparencia: { cor: "#1e1e22" } },
    pedra: { nome: "Pedra", item: "pedra", aparencia: { cor: "#d8c9a3" } },
  },

  coberturas: {
    grama: { nome: "Grama", substituivel: true, absorve: { poluicao: 1 }, aparencia: { forma: "grama", cor: "#5a9a4a" } },
    muda: {
      nome: "Muda", absorve: { poluicao: 8 }, regride: "toco",
      cresce: { para: "jovem", segundos: 40, bonusVizinho: "agua" },
      aparencia: { forma: "broto", cor: "#8fd16a" },
    },
    jovem: {
      nome: "Árvore jovem", marcas: ["floresta"], passagem: { pedestre: 2 }, absorve: { poluicao: 25 }, regride: "muda",
      cresce: { para: "adulta", segundos: 60, bonusVizinho: "agua" },
      aparencia: { forma: "arvore", tamanho: 0.3, cor: "#4c9a3e" },
    },
    adulta: {
      nome: "Árvore adulta", marcas: ["floresta"], passagem: { pedestre: 2 }, absorve: { poluicao: 50 }, regride: "jovem",
      colheita: { verbo: "Cortar", da: { madeira: 4, muda: 1 }, vira: "toco" },
      aparencia: { forma: "arvore", tamanho: 0.46, cor: "#2f6e2f" },
    },
    toco: {
      nome: "Toco", substituivel: true,
      colheita: { verbo: "Arrancar", da: { madeira: 1 }, vira: null },
      cresce: { para: "grama", segundos: 90 },
      aparencia: { forma: "toco", cor: "#7b5532" },
    },
  },

  itens: {
    madeira: { nome: "Madeira", combustivel: 1200, cor: "#b07a3f" },
    carvao: { nome: "Carvão", combustivel: 2400, cor: "#2b2b30" },
    pedra: { nome: "Pedra", cor: "#d8c9a3" },
    "minerio-ferro": { nome: "Minério de ferro", cor: "#8e9db0" },
    "minerio-cobre": { nome: "Minério de cobre", cor: "#c9773e" },
    "placa-ferro": { nome: "Placa de ferro", cor: "#cfd8e3" },
    "placa-cobre": { nome: "Placa de cobre", cor: "#f0a46a" },
    engrenagem: { nome: "Engrenagem", cor: "#9aa7b5" },
    ciencia: { nome: "Pacote de ciência", cor: "#e5533d" },
    muda: { nome: "Muda", planta: "muda", cor: "#8fd16a" },
  },

  receitas: {
    "placa-ferro": { nome: "Placa de ferro", entra: { "minerio-ferro": 1 }, sai: { "placa-ferro": 1 }, segundos: 3.2 },
    "placa-cobre": { nome: "Placa de cobre", entra: { "minerio-cobre": 1 }, sai: { "placa-cobre": 1 }, segundos: 3.2 },
    mudas: {
      nome: "Mudas", entra: { madeira: 1 }, sai: { muda: 2 }, segundos: 8,
      exige: { vizinho: "agua", aviso: "Precisa ficar ao lado da água" },
    },
    engrenagem: { nome: "Engrenagem", entra: { "placa-ferro": 2 }, sai: { engrenagem: 1 }, mao: true },
    ciencia: { nome: "Pacote de ciência", entra: { "placa-cobre": 1, engrenagem: 1 }, sai: { ciencia: 1 }, mao: true },
  },

  // Elementos = protótipo + componentes. O núcleo só executa componentes.
  elementos: {
    mineradora: {
      nome: "Mineradora a carvão", custo: { "placa-ferro": 5, pedra: 5 }, exige: "deposito",
      energia: { consumo: 3 }, extrai: { segundos: 4 }, entrega: true,
      inventario: { saida: 20, combustivel: 10 }, emite: { poluicao: 1000 },
      dica: "Fica sobre um depósito. Entrega o que extrai para a máquina ao lado.",
      aparencia: { cor: "#6d7782", simbolo: "⛏" },
    },
    fornalha: {
      nome: "Fornalha de pedra", custo: { pedra: 5 },
      energia: { consumo: 2 }, processo: ["placa-ferro", "placa-cobre"], entrega: true,
      inventario: { entrada: 20, saida: 50, combustivel: 10 }, emite: { poluicao: 400 },
      dica: "Derrete minério em placas. Queima carvão ou madeira.",
      aparencia: { cor: "#8c7b6b", simbolo: "🔥" },
    },
    bau: {
      nome: "Baú de madeira", custo: { madeira: 4 },
      inventario: { guarda: 100 },
      dica: "Guarda o que as máquinas vizinhas entregam.",
      aparencia: { cor: "#9a6b3a", simbolo: "▣" },
    },
    viveiro: {
      nome: "Viveiro", custo: { madeira: 10, pedra: 5 },
      processo: ["mudas"], entrega: true,
      inventario: { entrada: 20, saida: 20 },
      dica: "Transforma madeira em mudas. Precisa de água ao lado. Não polui.",
      aparencia: { cor: "#4c7d5b", simbolo: "🌱" },
    },
  },

  campos: {
    poluicao: {
      nome: "Poluição",
      difusao: { limiar: 1000, divisor: 16 },
      inibeCrescimento: 2500,
      dano: { limiar: 4000, chance: 3, aviso: "A poluição está adoecendo as árvores." },
    },
  },

  eras: [
    { nome: "Carvão e vapor", resumo: "Mineração e fundição a carvão, plantio manual de mudas." },
    { nome: "Eletricidade", resumo: "Rede elétrica, montadoras, viveiro automatizado, primeiras fazendas." },
    { nome: "Aço e automação", resumo: "Aço, esteiras, ferrovia; monocultura × policultura." },
    { nome: "Petróleo e química", resumo: "Plástico e combustíveis; filtros e tratamento de efluentes." },
    { nome: "Informação e robótica", resumo: "Robôs logísticos e monitoramento ambiental." },
    { nome: "Nuclear", resumo: "Muita energia, risco concentrado." },
    { nome: "Renovável", resumo: "Solar, eólica, restauração do solo." },
    { nome: "Espacial", resumo: "O satélite de observação da Terra." },
  ],

  tecnologias: {
    viveiro: { nome: "Viveiro", custo: { ciencia: 5 }, libera: ["viveiro"], descricao: "Produzir mudas em série." },
    eletricidade: { nome: "Eletricidade", custo: { ciencia: 20 }, requer: ["viveiro"], abreEra: 2, descricao: "Abre a era 2." },
  },

  inicio: {
    inventario: { madeira: 10, pedra: 15, "placa-ferro": 10, carvao: 5, muda: 4 },
    liberados: ["mineradora", "fornalha", "bau"],
  },

  missao: {
    nome: "Primeira fundição",
    abertura: "Bem-vindo à era do carvão e do vapor. Missão: primeira fundição.",
    metas: [
      { tipo: "producao", nome: "Placas de ferro", item: "placa-ferro", quantidade: 50 },
      { tipo: "cobertura", nome: "Floresta", marca: "floresta", minimoPct: 50 },
    ],
  },
};
