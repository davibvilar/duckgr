// Sorteio determinístico (mulberry32). O estado do gerador mora no próprio
// objeto (campo rng), então salvar o mundo salva também a sorte.
export function sortear(obj) {
  obj.rng = (obj.rng + 0x6d2b79f5) | 0;
  let t = obj.rng;
  t = Math.imul(t ^ (t >>> 15), t | 1);
  t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
  return (t ^ (t >>> 14)) >>> 0;
}
