// Terra Viva in English: the package's texts, in the same places as in
// package.js. Anything missing here shows in Portuguese.
export default {
  descricao: "First era: coal and steam. Produce without wiping out the forest.",
  mapa: {
    tamanhos: {
      pequeno: { nome: "Small" },
      medio: { nome: "Medium" },
      grande: { nome: "Large" },
    },
  },
  jogador: { nome: "Duck" },
  substratos: {
    solo: { nome: "Soil" },
    vau: { nome: "Shallow water" },
    agua: { nome: "Water" },
  },
  depositos: {
    ferro: { nome: "Iron ore" },
    cobre: { nome: "Copper ore" },
    carvao: { nome: "Coal" },
    pedra: { nome: "Stone" },
  },
  coberturas: {
    grama: { nome: "Grass" },
    muda: { nome: "Sapling" },
    jovem: { nome: "Young tree" },
    adulta: { nome: "Adult tree", colheita: { verbo: "Cut" } },
    toco: { nome: "Stump", colheita: { verbo: "Pull up" } },
  },
  itens: {
    madeira: { nome: "Wood" },
    carvao: { nome: "Coal" },
    pedra: { nome: "Stone" },
    "minerio-ferro": { nome: "Iron ore" },
    "minerio-cobre": { nome: "Copper ore" },
    "placa-ferro": { nome: "Iron plate" },
    "placa-cobre": { nome: "Copper plate" },
    engrenagem: { nome: "Gear" },
    ciencia: { nome: "Science pack" },
    muda: { nome: "Sapling" },
  },
  receitas: {
    "placa-ferro": { nome: "Iron plate" },
    "placa-cobre": { nome: "Copper plate" },
    mudas: { nome: "Saplings", exige: { aviso: "Needs to be next to water" } },
    engrenagem: { nome: "Gear" },
    ciencia: { nome: "Science pack" },
  },
  elementos: {
    mineradora: { nome: "Coal mining drill", dica: "Sits on a deposit. Hands what it mines to the machine next to it." },
    fornalha: { nome: "Stone furnace", dica: "Smelts ore into plates. Burns coal or wood." },
    bau: { nome: "Wooden chest", dica: "Stores what the neighboring machines hand over." },
    viveiro: { nome: "Nursery", dica: "Turns wood into saplings. Needs water next to it. Doesn't pollute." },
  },
  campos: {
    poluicao: { nome: "Pollution", dano: { aviso: "Pollution is making the trees sick." } },
  },
  eras: [
    { nome: "Coal and steam", resumo: "Coal mining and smelting, planting saplings by hand." },
    { nome: "Electricity", resumo: "Power grid, assemblers, automated nursery, first farms." },
    { nome: "Steel and automation", resumo: "Steel, belts, railways; monoculture × polyculture." },
    { nome: "Oil and chemistry", resumo: "Plastic and fuels; filters and wastewater treatment." },
    { nome: "Information and robotics", resumo: "Logistics robots and environmental monitoring." },
    { nome: "Nuclear", resumo: "Lots of energy, concentrated risk." },
    { nome: "Renewable", resumo: "Solar, wind, soil restoration." },
    { nome: "Space", resumo: "The Earth observation satellite." },
  ],
  tecnologias: {
    viveiro: { nome: "Nursery", descricao: "Make saplings in series." },
    eletricidade: { nome: "Electricity", descricao: "Opens era 2." },
  },
  missao: {
    nome: "First smelting",
    abertura: "Welcome to the era of coal and steam. Mission: first smelting.",
    metas: [{ nome: "Iron plates" }, { nome: "Forest" }],
  },
};
