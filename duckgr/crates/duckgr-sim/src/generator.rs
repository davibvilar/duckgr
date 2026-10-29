//! Map generator. Takes the package's generation rules (rivers, cover
//! patches, deposits) and returns the initial layers. Deterministic: it only
//! uses the seeded random draw.

use serde_json::{Map, Value};

use crate::js::{next_random, round, to_int32};
use crate::package::{Id, Package, Patch};

pub struct Layers {
    pub w: i64,
    pub h: i64,
    pub sub: Vec<Id>,
    pub cov: Vec<Option<Id>>,
    pub dep: Vec<Option<Id>>,
    pub dep_amount: Vec<i64>,
}

/// A map dimension from the configuration ("largura" or "altura").
pub fn dimension(cfg: &Map<String, Value>, k: &str) -> Result<i64, String> {
    let v = cfg.get(k).and_then(Value::as_f64).ok_or_else(|| format!("the configuration needs \"{k}\""))?;
    if v.fract() != 0.0 || !(1.0..=4096.0).contains(&v) {
        return Err(format!("\"{k}\" must be an integer between 1 and 4096"));
    }
    Ok(v as i64)
}

pub fn generate_map(p: &Package, cfg: &Map<String, Value>) -> Result<Layers, String> {
    let (w, h) = (dimension(cfg, "largura")?, dimension(cfg, "altura")?);
    let g = &p.generation;
    let seed = to_int32(cfg.get("semente").and_then(Value::as_f64).unwrap_or(0.0));
    let mut rng = match seed.wrapping_mul(0x9e37_79b1_u32 as i32) as u32 {
        0 => 7,
        r => r as i64,
    };
    let n = (w * h) as usize;
    let mut sub = vec![g.substrate; n];
    let mut cov = vec![g.cover; n];
    let mut dep: Vec<Option<Id>> = vec![None; n];
    let mut dep_amount = vec![0i64; n];
    let i = |x: i64, y: i64| (x + y * w) as usize;
    let inside = |x: i64, y: i64| x >= 0 && y >= 0 && x < w && y < h;
    let between = |rng: &mut i64, a: i64, b: i64| a + (next_random(rng) as i64) % (b - a + 1);
    // the cells of a disc, in the prototype's order
    let disc = |cx: i64, cy: i64, radius: i64| {
        let mut r = Vec::new();
        for y in cy - radius..=cy + radius {
            for x in cx - radius..=cx + radius {
                if inside(x, y) && (x - cx).pow(2) + (y - cy).pow(2) <= radius * radius + radius {
                    r.push((x, y));
                }
            }
        }
        r
    };

    // rivers: a band that winds from top to bottom
    for river in &g.rivers {
        let mut x = (w as f64 * river.position).floor() as i64;
        let fords: Vec<i64> = (0..river.fords).map(|k| (h * (k + 1)) / (river.fords + 1)).collect();
        for y in 0..h {
            x = (x + between(&mut rng, -1, 1)).min(w - river.width - 1).max(1);
            // fords: shallow stretches along the river where it can be crossed
            let ford = river.ford.filter(|_| fords.contains(&y));
            for k in 0..river.width {
                let c = i(x + k, y);
                if c < n {
                    sub[c] = ford.unwrap_or(river.substrate);
                    cov[c] = None;
                }
            }
        }
    }
    let water = |sub: &[Id], x: i64, y: i64| {
        let s = &p.substrates[sub[i(x, y)] as usize];
        s.plantable_is_false && !s.buildable
    };

    for patch in &g.patches {
        match patch {
            Patch::Cover { kinds, per_area, scale_by, radius, fill } => {
                let sum: f64 = kinds.iter().map(|(_, p)| p).sum();
                let scale = scale_by.as_ref().and_then(|k| cfg.get(k)).and_then(Value::as_f64).unwrap_or(1.0);
                let count = round(n as f64 * per_area * scale) as i64;
                for _ in 0..count {
                    let cx = between(&mut rng, 0, w - 1);
                    let cy = between(&mut rng, 0, h - 1);
                    let r = between(&mut rng, radius.0, radius.1);
                    for (x, y) in disc(cx, cy, r) {
                        if water(&sub, x, y) || (next_random(&mut rng) % 100) as f64 >= *fill {
                            continue;
                        }
                        let mut s = next_random(&mut rng) as f64 % sum;
                        for (kind, weight) in kinds {
                            if s < *weight {
                                cov[i(x, y)] = Some(*kind);
                                break;
                            }
                            s -= weight;
                        }
                    }
                }
            }
            Patch::Deposit { kind, at, radius, amount } => {
                let cx = (w as f64 * at.0).floor() as i64 + between(&mut rng, -2, 2);
                let cy = (h as f64 * at.1).floor() as i64 + between(&mut rng, -1, 1);
                for (x, y) in disc(cx, cy, *radius) {
                    if water(&sub, x, y) {
                        continue;
                    }
                    dep[i(x, y)] = Some(*kind);
                    dep_amount[i(x, y)] = between(&mut rng, amount.0, amount.1);
                    cov[i(x, y)] = g.cover;
                }
            }
        }
    }
    Ok(Layers { w, h, sub, cov, dep, dep_amount })
}
