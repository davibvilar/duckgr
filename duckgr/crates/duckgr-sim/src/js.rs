//! Aritmética com a mesma semântica do protótipo em JavaScript.
//!
//! O núcleo nasceu em JavaScript e os saves antigos guardam o hash que ele
//! calculava. Para abrir esses saves e conferir o hash, o núcleo em Rust
//! precisa arredondar, truncar e sortear exatamente como o JavaScript fazia.

use serde_json::Value;

/// `Math.round`: arredonda para o inteiro mais próximo; empate vai para +∞.
pub fn round(x: f64) -> f64 {
    let f = x.floor();
    if x - f >= 0.5 {
        f + 1.0
    } else {
        f
    }
}

fn modulo_2_32(x: f64) -> u32 {
    if !x.is_finite() {
        return 0;
    }
    x.trunc().rem_euclid(4_294_967_296.0) as u32
}

/// `x | 0`
pub fn to_int32(x: f64) -> i32 {
    modulo_2_32(x) as i32
}

/// `x >>> 0`
pub fn to_uint32(x: f64) -> u32 {
    modulo_2_32(x)
}

/// Sorteio determinístico (mulberry32). O estado do gerador mora num número
/// guardado no próprio mundo, então salvar o mundo salva também a sorte.
pub fn sortear(rng: &mut i64) -> u32 {
    let r = (*rng).wrapping_add(0x6d2b_79f5) as u32 as i32;
    *rng = r as i64;
    let mut t = r as u32;
    t = (t ^ (t >> 15)).wrapping_mul(t | 1);
    t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(t | 61));
    t ^ (t >> 14)
}

/// Verdadeiro no sentido do JavaScript (`if (v)`).
pub fn truthy(v: Option<&Value>) -> bool {
    match v {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64().is_some_and(|f| f != 0.0 && !f.is_nan()),
        Some(Value::String(s)) => !s.is_empty(),
        Some(_) => true,
    }
}

/// Número, se o valor for um número.
pub fn num(v: Option<&Value>) -> Option<f64> {
    v.and_then(Value::as_f64)
}

/// Texto, se o valor for um texto não vazio (`if (v)` com string).
pub fn texto(v: Option<&Value>) -> Option<&str> {
    v.and_then(Value::as_str).filter(|s| !s.is_empty())
}

/// Primeira letra minúscula, como `minusc` do protótipo.
pub fn minusc(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(p) => p.to_lowercase().chain(c).collect(),
        None => String::new(),
    }
}

/// Hash FNV-1a de 32 bits sobre as unidades UTF-16 do texto, como o
/// `hashEstado` do protótipo (que usava `charCodeAt`).
pub fn fnv_utf16(txt: &str) -> String {
    let mut h: u32 = 0x811c_9dc5;
    for u in txt.encode_utf16() {
        h ^= u as u32;
        h = h.wrapping_mul(0x0100_0193);
    }
    format!("{h:08x}")
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn round_como_javascript() {
        assert_eq!(round(2.5), 3.0);
        assert_eq!(round(-2.5), -2.0);
        assert_eq!(round(0.49999999999999994), 0.0);
        assert_eq!(round(17.454545454545453), 17.0);
    }

    #[test]
    fn inteiros_de_32_bits() {
        assert_eq!(to_uint32(-1.0), 4_294_967_295);
        assert_eq!(to_int32(4_294_967_295.0), -1);
        assert_eq!(to_int32(2026.7), 2026);
        assert_eq!(to_uint32(f64::NAN), 0);
    }

    #[test]
    fn mulberry32_igual_ao_javascript() {
        // valores tirados do protótipo: s = {rng: 1}; sortear(s) três vezes
        let mut r = 1;
        assert_eq!(sortear(&mut r), 2_693_262_067);
        assert_eq!(sortear(&mut r), 11_749_833);
        assert_eq!(sortear(&mut r), 2_265_367_787);
        assert_eq!(r, 1_199_730_144);
    }

    #[test]
    fn minusculas() {
        assert_eq!(minusc("Árvore adulta"), "árvore adulta");
        assert_eq!(minusc(""), "");
    }
}
