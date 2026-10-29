// Exporta os pacotes de mundo (web/jogos/) para JSON em testes/dados/, para os
// testes em Rust usarem os mesmos dados que o jogo. Um teste em JavaScript
// confere que o JSON não ficou para trás: se ele falhar, rode `npm run pacotes`.
import { writeFileSync } from "node:fs";
import { PACOTES } from "../web/jogos/catalogo.js";

for (const p of PACOTES) {
  const destino = new URL(`../testes/dados/${p.id}.json`, import.meta.url);
  writeFileSync(destino, JSON.stringify(p, null, 1) + "\n");
  console.log(`testes/dados/${p.id}.json`);
}
