//! Arithmetic with the same semantics as the JavaScript prototype.
//!
//! The core was born in JavaScript, and old saves store the hash it computed.
//! To open those saves and check their hash, the Rust core has to round,
//! truncate and draw random numbers exactly as JavaScript did.

use serde_json::Value;

/// `Math.round`: rounds to the nearest integer; ties go towards +∞.
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

/// Deterministic random draw (mulberry32). The generator state is a number
/// stored in the world itself, so saving the world also saves its luck.
pub fn next_random(rng: &mut i64) -> u32 {
    let r = (*rng).wrapping_add(0x6d2b_79f5) as u32 as i32;
    *rng = r as i64;
    let mut t = r as u32;
    t = (t ^ (t >> 15)).wrapping_mul(t | 1);
    t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(t | 61));
    t ^ (t >> 14)
}

/// Truthy in the JavaScript sense (`if (v)`).
pub fn truthy(v: Option<&Value>) -> bool {
    match v {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64().is_some_and(|f| f != 0.0 && !f.is_nan()),
        Some(Value::String(s)) => !s.is_empty(),
        Some(_) => true,
    }
}

/// The number, if the value is a number.
pub fn num(v: Option<&Value>) -> Option<f64> {
    v.and_then(Value::as_f64)
}

/// The text, if the value is a non-empty string (`if (v)` on a string).
pub fn nonempty_str(v: Option<&Value>) -> Option<&str> {
    v.and_then(Value::as_str).filter(|s| !s.is_empty())
}

/// First letter in lower case, like the prototype's `minusc`.
pub fn lower_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(p) => p.to_lowercase().chain(c).collect(),
        None => String::new(),
    }
}

/// 32-bit FNV-1a hash over the UTF-16 units of the text, like the
/// prototype's `hashEstado` (which used `charCodeAt`).
pub fn fnv_utf16(txt: &str) -> String {
    let mut h: u32 = 0x811c_9dc5;
    for u in txt.encode_utf16() {
        h ^= u as u32;
        h = h.wrapping_mul(0x0100_0193);
    }
    format!("{h:08x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_like_javascript() {
        assert_eq!(round(2.5), 3.0);
        assert_eq!(round(-2.5), -2.0);
        assert_eq!(round(0.49999999999999994), 0.0);
        assert_eq!(round(17.454545454545453), 17.0);
    }

    #[test]
    fn integers_of_32_bits() {
        assert_eq!(to_uint32(-1.0), 4_294_967_295);
        assert_eq!(to_int32(4_294_967_295.0), -1);
        assert_eq!(to_int32(2026.7), 2026);
        assert_eq!(to_uint32(f64::NAN), 0);
    }

    #[test]
    fn mulberry32_matches_javascript() {
        // values taken from the prototype: s = {rng: 1}; sortear(s) three times
        let mut r = 1;
        assert_eq!(next_random(&mut r), 2_693_262_067);
        assert_eq!(next_random(&mut r), 11_749_833);
        assert_eq!(next_random(&mut r), 2_265_367_787);
        assert_eq!(r, 1_199_730_144);
    }

    #[test]
    fn lower_case_first_letter() {
        assert_eq!(lower_first("Árvore adulta"), "árvore adulta");
        assert_eq!(lower_first(""), "");
    }
}
