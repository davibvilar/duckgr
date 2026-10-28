// Lista dos mundos (pacotes) que a plataforma oferece. Para criar um mundo
// novo, faça uma pasta em web/jogos/ com um pacote.js e registre aqui.
import terraViva from "./terra-viva/pacote.js";

export const PACOTES = [terraViva];
export const PACOTE_PADRAO = "terra-viva";

// Mundos planejados, mostrados na montagem como "em breve".
export const EM_BREVE = [
  { nome: "Paz ou Guerra", descricao: "As regras do Davi: só quem colocou pode quebrar." },
  { nome: "Mundo submarino", descricao: "Pressão, correntes e oxigênio produzido." },
];
