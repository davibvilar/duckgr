//! Núcleo da simulação do DuckGR.
//!
//! O núcleo não sabe que existe floresta, fornalha ou poluição. Ele conhece:
//! células com camadas (substrato, depósito, cobertura, ocupante), elementos
//! feitos de componentes (energia, extração, processo, entrega, emissão),
//! campos por chunk, eras, tecnologias, missões e comandos. Todo o conteúdo
//! vem do pacote do mundo.
//!
//! Regras de determinismo: tick fixo, só aritmética inteira no estado,
//! sorteios pelo gerador do próprio estado, nada de relógio de parede.
//! Mesmo pacote + mesma configuração + mesmos comandos = mesmo hash.

pub mod estado;
pub mod gerador;
pub mod jogador;
pub mod js;
pub mod pacote;

use std::collections::VecDeque;

use serde_json::{json, Value};

pub use estado::Estado;
pub use pacote::{compilar, Pacote};

use estado::{Bolsa, Jog, Missao};
use jogador::{aceita, bolsa_mut, resposta};
use js::{num, round, sortear, to_uint32};
use pacote::{Id, Itens};

/// Ticks por segundo.
pub const TPS: i64 = 30;
pub(crate) const VIZINHOS: [(i64, i64); 4] = [(1, 0), (0, 1), (-1, 0), (0, -1)];

// ---------- utilidades
pub fn chunk_de(s: &Estado, p: &Pacote, x: i64, y: i64) -> usize {
    (x / p.chunk + (y / p.chunk) * s.cl) as usize
}

fn vizinho_tem(s: &Estado, p: &Pacote, x: i64, y: i64, marca: Id) -> bool {
    VIZINHOS.iter().any(|(dx, dy)| {
        s.dentro(x + dx, y + dy) && p.substratos[s.sub[s.idx(x + dx, y + dy)] as usize].marcas.contains(&marca)
    })
}

/// Quantas células têm uma cobertura com esta marca.
pub fn contar_marca(s: &Estado, p: &Pacote, marca: &str) -> i64 {
    let Some(m) = p.marca(marca) else { return 0 };
    let tem: Vec<bool> = p.coberturas.iter().map(|c| c.marcas.contains(&m)).collect();
    s.cov.iter().filter(|c| c.is_some_and(|c| tem[c as usize])).count() as i64
}

pub(crate) fn tem_itens(s: &Estado, custo: &Itens) -> bool {
    custo.iter().all(|(it, n)| s.inv.get(*it) >= *n)
}

pub(crate) fn faltam(s: &Estado, p: &Pacote, custo: &Itens) -> String {
    custo
        .iter()
        .filter(|(it, n)| s.inv.get(*it) < *n)
        .map(|(it, n)| format!("{} {}", n - s.inv.get(*it), p.itens[*it as usize].nome.to_lowercase()))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn registrar_evento(s: &mut Estado, ev: Value) {
    s.eventos.push(ev);
}

/// Número de uma meta como o JavaScript escreveria num texto.
fn numero_txt(v: Option<&Value>) -> String {
    match v {
        Some(Value::Number(n)) => match (n.as_i64(), n.as_f64()) {
            (Some(i), _) => i.to_string(),
            (None, Some(f)) if f.fract() == 0.0 && f.abs() < 1e21 => format!("{}", f as i64),
            (None, Some(f)) => f.to_string(),
            _ => n.to_string(),
        },
        Some(Value::String(s)) => s.clone(),
        _ => "undefined".into(),
    }
}

fn chave_marca(m: &Value) -> String {
    match m.get("marca") {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Null) => "null".into(),
        Some(v) => v.to_string(),
        None => "undefined".into(),
    }
}

// ---------- estado inicial
pub fn novo_estado(p: &Pacote, cfg: &Value) -> Result<Estado, String> {
    let mut config = p.padrao.clone();
    if let Some(c) = cfg.as_object() {
        for (k, v) in c {
            config.insert(k.clone(), v.clone());
        }
    }
    let mapa = gerador::gerar_mapa(p, &config)?;
    let (l, a) = (mapa.l, mapa.a);
    let ch = p.chunk;
    let (cl, ca) = ((l + ch - 1) / ch, (a + ch - 1) / ch);
    let mult = num(config.get("recursosIniciais")).unwrap_or(1.0);
    let inv = Bolsa(p.inicio_inventario.iter().map(|(it, v)| (*it, round(v * mult) as i64)).collect());
    let semente = to_uint32(num(config.get("semente")).unwrap_or(f64::NAN));
    let metas = match config.get("metas") {
        Some(Value::Array(m)) => m.clone(),
        Some(v) if !v.is_null() => return Err("\"metas\" precisa ser uma lista".into()),
        _ => p.metas.as_array().cloned().unwrap_or_default(),
    };
    let n = (l * a) as usize;
    let mut s = Estado {
        tick: 0,
        rng: if semente == 0 { 1 } else { semente as i64 },
        l,
        a,
        cl,
        ca,
        sub: mapa.sub,
        dep: mapa.dep,
        dep_qtd: mapa.dep_qtd,
        cov: mapa.cov,
        cres: vec![0; n],
        occ: vec![0; n],
        ents: Vec::new(),
        prox: 1,
        inv,
        prod: Bolsa::default(),
        campos: vec![vec![0; (cl * ca) as usize]; p.campos.len()],
        techs: Vec::new(),
        era: 1,
        liberados: p.inicio_liberados.clone(),
        log: VecDeque::new(),
        ultimo_dano: -1_000_000_000,
        missao: Missao { estado: "andamento".into(), metas, refs: Vec::new() },
        jog: Jog::default(),
        eventos: Vec::new(),
    };
    // o jogador é um agente: fica fora da célula, com fila de ordens e caminho
    let (x, y) = jogador::posicao_inicial(&s, p);
    s.jog.x = x;
    s.jog.y = y;
    for m in s.missao.metas.clone() {
        if m.get("tipo").and_then(Value::as_str) == Some("cobertura") {
            let k = chave_marca(&m);
            let n = contar_marca(&s, p, &k);
            match s.missao.refs.iter_mut().find(|(r, _)| *r == k) {
                Some(r) => r.1 = n,
                None => s.missao.refs.push((k, n)),
            }
        }
    }
    let abertura = p.abertura.clone().unwrap_or_else(|| "O mundo começou.".into());
    s.registrar(abertura, "");
    Ok(s)
}

// ---------- passo da simulação (um tick)
pub fn passo(s: &mut Estado, p: &Pacote) {
    jogador::passo_jogador(s, p);
    for k in 0..s.ents.len() {
        passo_elemento(s, p, k);
    }
    s.tick += 1;
    if s.tick % TPS == 0 {
        por_segundo(s, p);
    }
}

fn energia(s: &mut Estado, p: &Pacote, k: usize) -> bool {
    let pr = &p.elementos[s.ents[k].p as usize];
    let Some(g) = pr.consumo else { return true };
    let e = &mut s.ents[k];
    if e.queima < g {
        let Some(comb) = e.comb.primeiro() else {
            definir_parado(e, "Sem combustível");
            return false;
        };
        e.comb.por(comb, -1);
        e.queima += p.itens[comb as usize].combustivel;
    }
    e.queima -= g;
    true
}

fn definir_parado(e: &mut estado::Ent, msg: &str) {
    if e.parado != msg {
        e.parado.clear();
        e.parado.push_str(msg);
    }
}

fn passo_elemento(s: &mut Estado, p: &Pacote, k: usize) {
    let pr = &p.elementos[s.ents[k].p as usize];
    s.ents[k].parado.clear();
    let (x, y) = (s.ents[k].x, s.ents[k].y);
    // componente "entrega": passa um item por tick para um vizinho que aceite
    if pr.entrega {
        if let Some(item) = s.ents[k].sai.primeiro() {
            for (dx, dy) in VIZINHOS {
                let Some(v) = s.ent_em(x + dx, y + dy) else { continue };
                if let Some(slot) = aceita(p, &s.ents[v], item) {
                    s.ents[k].sai.por(item, -1);
                    bolsa_mut(&mut s.ents[v], slot).por(item, 1);
                    break;
                }
            }
        }
    }
    // componente "extrai": tira itens do depósito sob o elemento
    if let Some(ticks) = pr.extrai_ticks {
        let i = s.idx(x, y);
        match s.dep[i] {
            None => definir_parado(&mut s.ents[k], "Depósito esgotado"),
            Some(_) if pr.inv.saida.is_some_and(|c| s.ents[k].sai.total() >= c) => {
                definir_parado(&mut s.ents[k], "Saída cheia")
            }
            Some(d) => {
                if energia(s, p, k) {
                    let e = &mut s.ents[k];
                    e.ativo += 1;
                    e.prog += 1;
                    if e.prog >= ticks {
                        e.prog = 0;
                        let item = p.depositos[d as usize].item;
                        e.sai.por(item, 1);
                        s.prod.por(item, 1);
                        s.dep_qtd[i] -= 1;
                        if s.dep_qtd[i] <= 0 {
                            s.dep[i] = None;
                            s.dep_qtd[i] = 0;
                            let nome = p.depositos[d as usize].nome.to_lowercase();
                            s.registrar(format!("Um depósito de {nome} se esgotou em ({x},{y})."), "");
                        }
                    }
                }
            }
        }
    }
    // componente "processo": transforma entradas em saídas segundo receitas
    if let Some(procs) = &pr.processo {
        if s.ents[k].rec.is_none() {
            let e = &s.ents[k];
            let cand = procs
                .iter()
                .map(|r| (*r, &p.receitas[*r as usize]))
                .find(|(_, r)| r.entra.iter().all(|(it, n)| e.ent.get(*it) >= *n));
            match cand {
                None => definir_parado(&mut s.ents[k], "Sem entrada"),
                Some((_, r)) if r.exige_vizinho.is_some_and(|m| !vizinho_tem(s, p, x, y, m)) => {
                    let aviso = r.exige_aviso.as_deref().unwrap_or("Condição do ambiente não atendida");
                    definir_parado(&mut s.ents[k], aviso)
                }
                Some((_, r))
                    if pr.inv.saida.is_some_and(|c| e.sai.total() + r.sai.iter().map(|(_, n)| n).sum::<i64>() > c) =>
                {
                    definir_parado(&mut s.ents[k], "Saída cheia")
                }
                Some((id, r)) => {
                    let e = &mut s.ents[k];
                    for (it, n) in &r.entra {
                        e.ent.por(*it, -n);
                    }
                    e.rec = Some(id);
                    e.prog = 0;
                }
            }
        }
        if let Some(rec) = s.ents[k].rec {
            if energia(s, p, k) {
                let r = &p.receitas[rec as usize];
                let e = &mut s.ents[k];
                e.ativo += 1;
                e.prog += 1;
                if e.prog >= r.ticks {
                    for (it, n) in &r.sai {
                        e.sai.por(*it, *n);
                        s.prod.por(*it, *n);
                    }
                    e.rec = None;
                    e.prog = 0;
                }
            }
        }
    }
}

// ---------- o que acontece uma vez por segundo: campos, crescimento, missão
fn por_segundo(s: &mut Estado, p: &Pacote) {
    let ch = p.chunk;
    let (l, a, cl, ca) = (s.l, s.a, s.cl, s.ca);
    for (f, campo) in p.campos.iter().enumerate() {
        let nk = s.campos[f].len();
        // fontes: elementos que trabalharam neste segundo
        for k in 0..s.ents.len() {
            let e = &s.ents[k];
            let em = p.elementos[e.p as usize].emite[f];
            if em != 0.0 && !em.is_nan() {
                let c = chunk_de(s, p, e.x, e.y);
                s.campos[f][c] += ((em * e.ativo as f64) / TPS as f64).floor() as i64;
            }
        }
        // sumidouros: coberturas que absorvem
        let mut abs = vec![0i64; nk];
        for y in 0..a {
            for x in 0..l {
                if let Some(c) = s.cov[s.idx(x, y)] {
                    let q = p.coberturas[c as usize].absorve[f];
                    if q != 0 {
                        abs[chunk_de(s, p, x, y)] += q;
                    }
                }
            }
        }
        let v = &mut s.campos[f];
        for k in 0..nk {
            v[k] = (v[k] - abs[k]).max(0);
        }
        // difusão em duas fases: calcula tudo, depois aplica
        let mut delta = vec![0i64; nk];
        for cy in 0..ca {
            for cx in 0..cl {
                let k = (cx + cy * cl) as usize;
                if v[k] as f64 <= campo.difusao_limiar {
                    continue;
                }
                let parte = (v[k] as f64 / campo.difusao_divisor).floor() as i64;
                for (dx, dy) in VIZINHOS {
                    let (nx, ny) = (cx + dx, cy + dy);
                    if nx < 0 || ny < 0 || nx >= cl || ny >= ca {
                        continue;
                    }
                    delta[k] -= parte;
                    delta[(nx + ny * cl) as usize] += parte;
                }
            }
        }
        for k in 0..nk {
            v[k] += delta[k];
        }
        // dano: acima do limiar, a cobertura regride
        if let Some(dano) = &campo.dano {
            for k in 0..nk {
                if s.campos[f][k] as f64 <= dano.limiar || (sortear(&mut s.rng) as f64 % dano.chance) != 0.0 {
                    continue;
                }
                let (cx, cy) = ((k as i64) % cl, (k as i64) / cl);
                let mut alvos = Vec::new();
                for y in cy * ch..a.min(cy * ch + ch) {
                    for x in cx * ch..l.min(cx * ch + ch) {
                        let i = s.idx(x, y);
                        if s.cov[i].is_some_and(|c| p.coberturas[c as usize].regride.is_some()) {
                            alvos.push(i);
                        }
                    }
                }
                if alvos.is_empty() {
                    continue;
                }
                let i = alvos[(sortear(&mut s.rng) as usize) % alvos.len()];
                let c = s.cov[i].expect("alvo tem cobertura");
                s.cov[i] = p.coberturas[c as usize].regride.expect("alvo regride");
                s.cres[i] = 0;
                if s.tick - s.ultimo_dano > 15 * TPS {
                    s.ultimo_dano = s.tick;
                    s.registrar(dano.aviso.clone(), "ruim");
                }
            }
        }
    }
    for y in 0..a {
        for x in 0..l {
            let i = s.idx(x, y);
            let Some(cr) = s.cov[i].and_then(|c| p.coberturas[c as usize].cresce.as_ref()) else { continue };
            let k = chunk_de(s, p, x, y);
            if p.campos
                .iter()
                .enumerate()
                .any(|(f, c)| c.inibe != 0.0 && !c.inibe.is_nan() && s.campos[f][k] as f64 > c.inibe)
            {
                continue;
            }
            s.cres[i] += if cr.bonus_vizinho.is_some_and(|m| vizinho_tem(s, p, x, y, m)) { 2 } else { 1 };
            if s.cres[i] as f64 >= cr.segundos {
                s.cov[i] = cr.para;
                s.cres[i] = 0;
            }
        }
    }
    for e in s.ents.iter_mut() {
        e.ativo = 0;
    }
    avaliar_missao(s, p);
}

/// Progresso de cada meta da missão: as metas como vieram, com o andamento.
pub fn progresso_missao(s: &Estado, p: &Pacote) -> Vec<Value> {
    s.missao
        .metas
        .iter()
        .map(|m| {
            let mut r = m.as_object().cloned().unwrap_or_default();
            if m.get("tipo").and_then(Value::as_str) == Some("producao") {
                let item = m.get("item").and_then(Value::as_str).and_then(|i| p.item(i));
                let feito = item.map_or(0, |i| s.prod.get(i));
                let ok = num(m.get("quantidade")).is_some_and(|q| feito as f64 >= q);
                r.insert("feito".into(), feito.into());
                r.insert("ok".into(), ok.into());
            } else {
                let k = chave_marca(m);
                let atual = contar_marca(s, p, &k);
                let rf = s.missao.refs.iter().find(|(r, _)| *r == k).map_or(0, |(_, n)| *n);
                let rf = if rf == 0 { 1 } else { rf };
                let pct = (atual * 100).div_euclid(rf);
                let ok = num(m.get("minimoPct")).is_some_and(|q| pct as f64 >= q);
                r.insert("atual".into(), atual.into());
                r.insert("pct".into(), pct.into());
                r.insert("ok".into(), ok.into());
            }
            Value::Object(r)
        })
        .collect()
}

fn avaliar_missao(s: &mut Estado, p: &Pacote) {
    if s.missao.estado != "andamento" {
        return;
    }
    let prog = progresso_missao(s, p);
    let ok = |m: &Value| m.get("ok") == Some(&Value::Bool(true));
    let falha = prog.iter().find(|m| m.get("tipo").and_then(Value::as_str) == Some("cobertura") && !ok(m));
    if let Some(f) = falha {
        s.missao.estado = "falhou".into();
        let nome = f.get("nome").and_then(Value::as_str).unwrap_or("").to_lowercase();
        let txt = format!("Missão perdida: {nome} caiu abaixo de {}%.", numero_txt(f.get("minimoPct")));
        s.registrar(txt, "ruim");
        registrar_evento(s, json!({ "tipo": "missao", "estado": "falhou", "meta": f }));
    } else if prog.iter().all(ok) {
        s.missao.estado = "cumprida".into();
        s.registrar(format!("Missão cumprida: {}!", p.missao_nome), "bom");
        registrar_evento(s, json!({ "tipo": "missao", "estado": "cumprida" }));
    }
}

// ---------- comandos: a única forma de mudar o mundo. Um comando falho não muda nada.
pub fn aplicar(s: &mut Estado, p: &Pacote, cmd: &Value) -> Value {
    match cmd.get("tipo").and_then(Value::as_str) {
        Some("ordem") => jogador::ordem(s, p, cmd),
        Some("cancelar") => jogador::cancelar(s),
        Some("fabricar") => fabricar(s, p, cmd),
        Some("pesquisar") => pesquisar(s, p, cmd),
        _ => {
            let t = match cmd.get("tipo") {
                Some(Value::String(t)) => t.clone(),
                Some(v) => v.to_string(),
                None => "undefined".into(),
            };
            resposta(false, Some(format!("Comando desconhecido: {t}")))
        }
    }
}

fn fabricar(s: &mut Estado, p: &Pacote, cmd: &Value) -> Value {
    let r = cmd.get("receita").and_then(Value::as_str).and_then(|r| p.receita(r)).map(|r| &p.receitas[r as usize]);
    let Some(r) = r.filter(|r| r.mao) else { return resposta(false, Some("Isso não se faz à mão.".into())) };
    if !tem_itens(s, &r.entra) {
        return resposta(false, Some(format!("Faltam {}.", faltam(s, p, &r.entra))));
    }
    for (it, n) in &r.entra {
        s.inv.por(*it, -n);
    }
    for (it, n) in &r.sai {
        s.inv.por(*it, *n);
        s.prod.por(*it, *n);
    }
    resposta(true, None)
}

fn pesquisar(s: &mut Estado, p: &Pacote, cmd: &Value) -> Value {
    let id = cmd.get("tecnologia").and_then(Value::as_str).and_then(|t| p.tecnologia(t));
    let Some(id) = id.filter(|t| !s.techs.contains(t)) else { return resposta(false, None) };
    let t = &p.tecnologias[id as usize];
    let falta: Vec<&str> =
        t.requer.iter().filter(|r| !s.techs.contains(r)).map(|r| p.tecnologias[*r as usize].nome.as_str()).collect();
    if !falta.is_empty() {
        return resposta(false, Some(format!("Antes pesquise: {}.", falta.join(", "))));
    }
    if !tem_itens(s, &t.custo) {
        return resposta(false, Some(format!("Faltam {}.", faltam(s, p, &t.custo))));
    }
    for (it, n) in &t.custo {
        s.inv.por(*it, -n);
    }
    s.techs.push(id);
    for el in &t.libera {
        if !s.liberados.contains(el) {
            s.liberados.push(*el);
        }
    }
    s.registrar(format!("Pesquisa concluída: {}.", t.nome), "bom");
    if t.abre_era != 0 && t.abre_era > s.era {
        s.era = t.abre_era;
        let nome = p.eras.get((t.abre_era - 1) as usize).cloned().unwrap_or_default();
        s.registrar(format!("Estamos entrando na era {nome}!"), "bom");
        registrar_evento(s, json!({ "tipo": "era", "era": t.abre_era }));
    }
    resposta(true, None)
}

// ---------- replay: refaz comandos a partir de um estado
fn tick_de(c: &Value) -> Option<f64> {
    num(c.get("t"))
}

/// Refaz os comandos a partir de um estado, até o tick pedido. Os comandos de
/// antes do tick atual já estão no estado e são pulados.
pub fn avancar(s: &mut Estado, p: &Pacote, comandos: &[Value], ate_tick: i64) {
    let mut k = 0;
    while k < comandos.len() && tick_de(&comandos[k]).is_some_and(|t| t < s.tick as f64) {
        k += 1;
    }
    let aplicar_do_tick = |s: &mut Estado, k: &mut usize| {
        while *k < comandos.len() && tick_de(&comandos[*k]) == Some(s.tick as f64) {
            aplicar(s, p, &comandos[*k]);
            *k += 1;
        }
    };
    while s.tick < ate_tick {
        aplicar_do_tick(s, &mut k);
        passo(s, p);
    }
    aplicar_do_tick(s, &mut k);
    s.eventos.clear();
}

/// Recria o mundo a partir da configuração e do registro de comandos.
pub fn rodar(p: &Pacote, cfg: &Value, comandos: &[Value], ate_tick: i64) -> Result<Estado, String> {
    let mut s = novo_estado(p, cfg)?;
    avancar(&mut s, p, comandos, ate_tick);
    Ok(s)
}
