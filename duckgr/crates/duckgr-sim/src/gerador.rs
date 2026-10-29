//! Gerador de mapa. Recebe as regras de geração do pacote (rios, manchas de
//! cobertura, depósitos) e devolve as camadas iniciais. Determinístico: só
//! usa o sorteio semeado.

use serde_json::{Map, Value};

use crate::js::{round, sortear, to_int32};
use crate::pacote::{Id, Mancha, Pacote};

pub struct Mapa {
    pub l: i64,
    pub a: i64,
    pub sub: Vec<Id>,
    pub cov: Vec<Option<Id>>,
    pub dep: Vec<Option<Id>>,
    pub dep_qtd: Vec<i64>,
}

pub fn dimensao(cfg: &Map<String, Value>, k: &str) -> Result<i64, String> {
    let v = cfg.get(k).and_then(Value::as_f64).ok_or_else(|| format!("a configuração precisa de \"{k}\""))?;
    if v.fract() != 0.0 || !(1.0..=4096.0).contains(&v) {
        return Err(format!("\"{k}\" precisa ser um inteiro entre 1 e 4096"));
    }
    Ok(v as i64)
}

pub fn gerar_mapa(p: &Pacote, cfg: &Map<String, Value>) -> Result<Mapa, String> {
    let (l, a) = (dimensao(cfg, "largura")?, dimensao(cfg, "altura")?);
    let g = &p.geracao;
    let semente = to_int32(cfg.get("semente").and_then(Value::as_f64).unwrap_or(0.0));
    let mut rng = match semente.wrapping_mul(0x9e37_79b1_u32 as i32) as u32 {
        0 => 7,
        r => r as i64,
    };
    let n = (l * a) as usize;
    let mut sub = vec![g.substrato; n];
    let mut cov = vec![g.cobertura; n];
    let mut dep: Vec<Option<Id>> = vec![None; n];
    let mut dep_qtd = vec![0i64; n];
    let i = |x: i64, y: i64| (x + y * l) as usize;
    let dentro = |x: i64, y: i64| x >= 0 && y >= 0 && x < l && y < a;
    let entre = |rng: &mut i64, a: i64, b: i64| a + (sortear(rng) as i64) % (b - a + 1);
    // as células de um círculo, na ordem do protótipo
    let circulo = |cx: i64, cy: i64, raio: i64| {
        let mut r = Vec::new();
        for y in cy - raio..=cy + raio {
            for x in cx - raio..=cx + raio {
                if dentro(x, y) && (x - cx).pow(2) + (y - cy).pow(2) <= raio * raio + raio {
                    r.push((x, y));
                }
            }
        }
        r
    };

    // rios: uma faixa que serpenteia de cima a baixo
    for rio in &g.rios {
        let mut x = (l as f64 * rio.posicao).floor() as i64;
        let vaus: Vec<i64> = (0..rio.vaus).map(|k| (a * (k + 1)) / (rio.vaus + 1)).collect();
        for y in 0..a {
            x = (x + entre(&mut rng, -1, 1)).min(l - rio.largura - 1).max(1);
            // vaus: trechos rasos, distribuídos ao longo do rio, por onde dá para atravessar
            let vau = rio.vau.filter(|_| vaus.contains(&y));
            for k in 0..rio.largura {
                let c = i(x + k, y);
                if c < n {
                    sub[c] = vau.unwrap_or(rio.substrato);
                    cov[c] = None;
                }
            }
        }
    }
    let agua = |sub: &[Id], x: i64, y: i64| {
        let s = &p.substratos[sub[i(x, y)] as usize];
        s.plantar_falso && !s.construir
    };

    for m in &g.manchas {
        match m {
            Mancha::Cobertura { tipos, por_area, escala_por, raio, preenche } => {
                let soma: f64 = tipos.iter().map(|(_, p)| p).sum();
                let escala = escala_por.as_ref().and_then(|k| cfg.get(k)).and_then(Value::as_f64).unwrap_or(1.0);
                let qtd = round(n as f64 * por_area * escala) as i64;
                for _ in 0..qtd {
                    let cx = entre(&mut rng, 0, l - 1);
                    let cy = entre(&mut rng, 0, a - 1);
                    let r = entre(&mut rng, raio.0, raio.1);
                    for (x, y) in circulo(cx, cy, r) {
                        if agua(&sub, x, y) || (sortear(&mut rng) % 100) as f64 >= *preenche {
                            continue;
                        }
                        let mut s = sortear(&mut rng) as f64 % soma;
                        for (t, peso) in tipos {
                            if s < *peso {
                                cov[i(x, y)] = Some(*t);
                                break;
                            }
                            s -= peso;
                        }
                    }
                }
            }
            Mancha::Deposito { tipo, em, raio, qtd } => {
                let cx = (l as f64 * em.0).floor() as i64 + entre(&mut rng, -2, 2);
                let cy = (a as f64 * em.1).floor() as i64 + entre(&mut rng, -1, 1);
                for (x, y) in circulo(cx, cy, *raio) {
                    if agua(&sub, x, y) {
                        continue;
                    }
                    dep[i(x, y)] = Some(*tipo);
                    dep_qtd[i(x, y)] = entre(&mut rng, qtd.0, qtd.1);
                    cov[i(x, y)] = g.cobertura;
                }
            }
        }
    }
    Ok(Mapa { l, a, sub, cov, dep, dep_qtd })
}
