//! Simulation core of DuckGR.
//!
//! The core doesn't know that forests, furnaces or pollution exist. It
//! knows: cells with layers (substrate, deposit, cover, occupant), elements
//! made of components (energy, extraction, process, delivery, emission),
//! fields per chunk, eras, technologies, missions and commands. All content
//! comes from the world package.
//!
//! Determinism rules: fixed tick, only integer arithmetic in the state,
//! random draws from the state's own generator, no wall clock.
//! Same package + same configuration + same commands = same hash.
//!
//! Interface text is not written here: the core speaks in messages (key +
//! arguments, see `message`) and the interface renders them in the
//! player's language.

pub mod generator;
pub mod js;
pub mod message;
pub mod package;
pub mod player;
pub mod state;

use std::collections::VecDeque;

use serde_json::{json, Value};

pub use package::{compile, Package};
pub use state::State;

use js::{next_random, num, round, to_uint32};
use message::{items, key, list, msg, name_ref, text_ref, text_ref_or};
use package::{Id, ItemList};
use player::{accepts, bag_mut, reply, STOPPED_NO_FUEL, STOPPED_NO_INPUT};
use state::{Bag, Entity, Mission, Player};

/// Ticks per second.
pub const TPS: i64 = 30;
pub(crate) const NEIGHBORS: [(i64, i64); 4] = [(1, 0), (0, 1), (-1, 0), (0, -1)];

// ---------- utilities
pub fn chunk_of(s: &State, p: &Package, x: i64, y: i64) -> usize {
    (x / p.chunk + (y / p.chunk) * s.cw) as usize
}

fn neighbor_has(s: &State, p: &Package, x: i64, y: i64, mark: Id) -> bool {
    NEIGHBORS.iter().any(|(dx, dy)| {
        s.inside(x + dx, y + dy) && p.substrates[s.sub[s.idx(x + dx, y + dy)] as usize].marks.contains(&mark)
    })
}

/// How many cells have a cover with this mark.
pub fn count_mark(s: &State, p: &Package, mark: &str) -> i64 {
    let Some(m) = p.mark(mark) else { return 0 };
    let has: Vec<bool> = p.covers.iter().map(|c| c.marks.contains(&m)).collect();
    s.cov.iter().filter(|c| c.is_some_and(|c| has[c as usize])).count() as i64
}

pub(crate) fn has_items(s: &State, cost: &ItemList) -> bool {
    cost.iter().all(|(it, n)| s.inv.get(*it) >= *n)
}

/// What is missing to pay a cost, as a message argument ("2 madeira, 1 pedra").
pub(crate) fn missing_items(s: &State, p: &Package, cost: &ItemList) -> Value {
    let missing: ItemList =
        cost.iter().filter(|(it, n)| s.inv.get(*it) < *n).map(|(it, n)| (*it, n - s.inv.get(*it))).collect();
    items(p, &missing)
}

pub(crate) fn push_event(s: &mut State, ev: Value) {
    s.events.push(ev);
}

/// A goal's number as JavaScript would write it in a text.
fn number_text(v: Option<&Value>) -> String {
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

fn mark_key(m: &Value) -> String {
    match m.get("marca") {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Null) => "null".into(),
        Some(v) => v.to_string(),
        None => "undefined".into(),
    }
}

// ---------- initial state
pub fn new_state(p: &Package, cfg: &Value) -> Result<State, String> {
    let mut config = p.defaults.clone();
    if let Some(c) = cfg.as_object() {
        for (k, v) in c {
            config.insert(k.clone(), v.clone());
        }
    }
    let map = generator::generate_map(p, &config)?;
    let (w, h) = (map.w, map.h);
    let chunk = p.chunk;
    let (cw, ch) = ((w + chunk - 1) / chunk, (h + chunk - 1) / chunk);
    let mult = num(config.get("recursosIniciais")).unwrap_or(1.0);
    let inv = Bag(p.start_inventory.iter().map(|(it, v)| (*it, round(v * mult) as i64)).collect());
    let seed = to_uint32(num(config.get("semente")).unwrap_or(f64::NAN));
    let goals = match config.get("metas") {
        Some(Value::Array(m)) => m.clone(),
        Some(v) if !v.is_null() => return Err("\"metas\" must be a list".into()),
        _ => p.goals.as_array().cloned().unwrap_or_default(),
    };
    let n = (w * h) as usize;
    let mut s = State {
        tick: 0,
        rng: if seed == 0 { 1 } else { seed as i64 },
        w,
        h,
        cw,
        ch,
        sub: map.sub,
        dep: map.dep,
        dep_amount: map.dep_amount,
        cov: map.cov,
        growth: vec![0; n],
        occ: vec![0; n],
        entities: Vec::new(),
        next_id: 1,
        inv,
        produced: Bag::default(),
        fields: vec![vec![0; (cw * ch) as usize]; p.fields.len()],
        techs: Vec::new(),
        era: 1,
        unlocked: p.start_unlocked.clone(),
        log: VecDeque::new(),
        last_damage: -1_000_000_000,
        mission: Mission { status: "andamento".into(), goals, refs: Vec::new() },
        player: Player::default(),
        events: Vec::new(),
    };
    // the player is an agent: it stands outside the cell, with an order queue and a path
    let (x, y) = player::start_position(&s, p);
    s.player.x = x;
    s.player.y = y;
    for g in s.mission.goals.clone() {
        if g.get("tipo").and_then(Value::as_str) == Some("cobertura") {
            let k = mark_key(&g);
            let n = count_mark(&s, p, &k);
            match s.mission.refs.iter_mut().find(|(r, _)| *r == k) {
                Some(r) => r.1 = n,
                None => s.mission.refs.push((k, n)),
            }
        }
    }
    match p.opening.clone() {
        Some(text) => s.record(text, "", text_ref(&["missao", "abertura"])),
        None => s.record("O mundo começou.", "", key("log.worldStarted")),
    }
    Ok(s)
}

// ---------- simulation step (one tick)
pub fn step(s: &mut State, p: &Package) {
    player::player_step(s, p);
    for k in 0..s.entities.len() {
        element_step(s, p, k);
    }
    s.tick += 1;
    if s.tick % TPS == 0 {
        every_second(s, p);
    }
}

fn burn_fuel(s: &mut State, p: &Package, k: usize) -> bool {
    let proto = &p.elements[s.entities[k].proto as usize];
    let Some(g) = proto.consumption else { return true };
    let e = &mut s.entities[k];
    if e.burn < g {
        let Some(fuel) = e.fuel.first() else {
            set_stopped(e, STOPPED_NO_FUEL, || key("stopped.noFuel"));
            return false;
        };
        e.fuel.add(fuel, -1);
        e.burn += p.items[fuel as usize].fuel;
    }
    e.burn -= g;
    true
}

/// Marks why the element stopped: the frozen text (hash) and the message (interface).
fn set_stopped(e: &mut Entity, text: &str, message: impl FnOnce() -> Value) {
    if e.stopped != text {
        e.stopped.clear();
        e.stopped.push_str(text);
        e.stopped_msg = message();
    }
}

fn clear_stopped(e: &mut Entity) {
    e.stopped.clear();
    e.stopped_msg = Value::Null;
}

fn element_step(s: &mut State, p: &Package, k: usize) {
    let proto = &p.elements[s.entities[k].proto as usize];
    clear_stopped(&mut s.entities[k]);
    let (x, y) = (s.entities[k].x, s.entities[k].y);
    // "delivery" component: passes one item per tick to a neighbor that accepts it
    if proto.delivers {
        if let Some(item) = s.entities[k].output.first() {
            for (dx, dy) in NEIGHBORS {
                let Some(v) = s.entity_at(x + dx, y + dy) else { continue };
                if let Some(slot) = accepts(p, &s.entities[v], item) {
                    s.entities[k].output.add(item, -1);
                    bag_mut(&mut s.entities[v], slot).add(item, 1);
                    break;
                }
            }
        }
    }
    // "extract" component: takes items from the deposit under the element
    if let Some(ticks) = proto.extract_ticks {
        let i = s.idx(x, y);
        match s.dep[i] {
            None => set_stopped(&mut s.entities[k], "Depósito esgotado", || key("stopped.depleted")),
            Some(_) if proto.inv.output.is_some_and(|c| s.entities[k].output.total() >= c) => {
                set_stopped(&mut s.entities[k], "Saída cheia", || key("stopped.outputFull"))
            }
            Some(d) => {
                if burn_fuel(s, p, k) {
                    let e = &mut s.entities[k];
                    e.active += 1;
                    e.progress += 1;
                    if e.progress >= ticks {
                        e.progress = 0;
                        let item = p.deposits[d as usize].item;
                        e.output.add(item, 1);
                        s.produced.add(item, 1);
                        s.dep_amount[i] -= 1;
                        if s.dep_amount[i] <= 0 {
                            s.dep[i] = None;
                            s.dep_amount[i] = 0;
                            let deposit = &p.deposits[d as usize];
                            let text =
                                format!("Um depósito de {} se esgotou em ({x},{y}).", deposit.name.to_lowercase());
                            let args = json!({ "deposit": name_ref("depositos", &deposit.id), "x": x, "y": y });
                            s.record(text, "", msg("log.depositDepleted", args));
                        }
                    }
                }
            }
        }
    }
    // "process" component: turns inputs into outputs following recipes
    if let Some(recipes) = &proto.process {
        if s.entities[k].recipe.is_none() {
            let e = &s.entities[k];
            let candidate = recipes
                .iter()
                .map(|r| (*r, &p.recipes[*r as usize]))
                .find(|(_, r)| r.inputs.iter().all(|(it, n)| e.input.get(*it) >= *n));
            match candidate {
                None => set_stopped(&mut s.entities[k], STOPPED_NO_INPUT, || key("stopped.noInput")),
                Some((_, r)) if r.needs_neighbor.is_some_and(|m| !neighbor_has(s, p, x, y, m)) => {
                    match &r.needs_warning {
                        Some(w) => set_stopped(&mut s.entities[k], w, || {
                            text_ref_or(&["receitas", &r.id, "exige", "aviso"], w)
                        }),
                        None => set_stopped(&mut s.entities[k], "Condição do ambiente não atendida", || {
                            key("stopped.condition")
                        }),
                    }
                }
                Some((_, r))
                    if proto
                        .inv
                        .output
                        .is_some_and(|c| e.output.total() + r.outputs.iter().map(|(_, n)| n).sum::<i64>() > c) =>
                {
                    set_stopped(&mut s.entities[k], "Saída cheia", || key("stopped.outputFull"))
                }
                Some((id, r)) => {
                    let e = &mut s.entities[k];
                    for (it, n) in &r.inputs {
                        e.input.add(*it, -n);
                    }
                    e.recipe = Some(id);
                    e.progress = 0;
                }
            }
        }
        if let Some(rec) = s.entities[k].recipe {
            if burn_fuel(s, p, k) {
                let r = &p.recipes[rec as usize];
                let e = &mut s.entities[k];
                e.active += 1;
                e.progress += 1;
                if e.progress >= r.ticks {
                    for (it, n) in &r.outputs {
                        e.output.add(*it, *n);
                        s.produced.add(*it, *n);
                    }
                    e.recipe = None;
                    e.progress = 0;
                }
            }
        }
    }
}

// ---------- what happens once per second: fields, growth, mission
fn every_second(s: &mut State, p: &Package) {
    let chunk = p.chunk;
    let (w, h, cw, ch) = (s.w, s.h, s.cw, s.ch);
    for (f, field) in p.fields.iter().enumerate() {
        let nk = s.fields[f].len();
        // sources: elements that worked during this second
        for k in 0..s.entities.len() {
            let e = &s.entities[k];
            let em = p.elements[e.proto as usize].emits[f];
            if em != 0.0 && !em.is_nan() {
                let c = chunk_of(s, p, e.x, e.y);
                s.fields[f][c] += ((em * e.active as f64) / TPS as f64).floor() as i64;
            }
        }
        // sinks: covers that absorb
        let mut absorbed = vec![0i64; nk];
        for y in 0..h {
            for x in 0..w {
                if let Some(c) = s.cov[s.idx(x, y)] {
                    let q = p.covers[c as usize].absorbs[f];
                    if q != 0 {
                        absorbed[chunk_of(s, p, x, y)] += q;
                    }
                }
            }
        }
        let v = &mut s.fields[f];
        for k in 0..nk {
            v[k] = (v[k] - absorbed[k]).max(0);
        }
        // spreading in two phases: compute everything, then apply
        let mut delta = vec![0i64; nk];
        for cy in 0..ch {
            for cx in 0..cw {
                let k = (cx + cy * cw) as usize;
                if v[k] as f64 <= field.spread_threshold {
                    continue;
                }
                let share = (v[k] as f64 / field.spread_divisor).floor() as i64;
                for (dx, dy) in NEIGHBORS {
                    let (nx, ny) = (cx + dx, cy + dy);
                    if nx < 0 || ny < 0 || nx >= cw || ny >= ch {
                        continue;
                    }
                    delta[k] -= share;
                    delta[(nx + ny * cw) as usize] += share;
                }
            }
        }
        for k in 0..nk {
            v[k] += delta[k];
        }
        // damage: above the threshold, the cover regresses
        if let Some(damage) = &field.damage {
            for k in 0..nk {
                if s.fields[f][k] as f64 <= damage.threshold || (next_random(&mut s.rng) as f64 % damage.chance) != 0.0
                {
                    continue;
                }
                let (cx, cy) = ((k as i64) % cw, (k as i64) / cw);
                let mut targets = Vec::new();
                for y in cy * chunk..h.min(cy * chunk + chunk) {
                    for x in cx * chunk..w.min(cx * chunk + chunk) {
                        let i = s.idx(x, y);
                        if s.cov[i].is_some_and(|c| p.covers[c as usize].regresses.is_some()) {
                            targets.push(i);
                        }
                    }
                }
                if targets.is_empty() {
                    continue;
                }
                let i = targets[(next_random(&mut s.rng) as usize) % targets.len()];
                let c = s.cov[i].expect("target has a cover");
                s.cov[i] = p.covers[c as usize].regresses.expect("target regresses");
                s.growth[i] = 0;
                if s.tick - s.last_damage > 15 * TPS {
                    s.last_damage = s.tick;
                    s.record(damage.warning.clone(), "ruim", text_ref(&["campos", &field.id, "dano", "aviso"]));
                }
            }
        }
    }
    for y in 0..h {
        for x in 0..w {
            let i = s.idx(x, y);
            let Some(g) = s.cov[i].and_then(|c| p.covers[c as usize].growth.as_ref()) else { continue };
            let k = chunk_of(s, p, x, y);
            if p.fields
                .iter()
                .enumerate()
                .any(|(f, c)| c.inhibits != 0.0 && !c.inhibits.is_nan() && s.fields[f][k] as f64 > c.inhibits)
            {
                continue;
            }
            s.growth[i] += if g.neighbor_bonus.is_some_and(|m| neighbor_has(s, p, x, y, m)) { 2 } else { 1 };
            if s.growth[i] as f64 >= g.seconds {
                s.cov[i] = g.into;
                s.growth[i] = 0;
            }
        }
    }
    for e in s.entities.iter_mut() {
        e.active = 0;
    }
    check_mission(s, p);
}

/// Progress of each mission goal: the goals as they came, with how far along they are.
pub fn mission_progress(s: &State, p: &Package) -> Vec<Value> {
    s.mission
        .goals
        .iter()
        .map(|g| {
            let mut r = g.as_object().cloned().unwrap_or_default();
            if g.get("tipo").and_then(Value::as_str) == Some("producao") {
                let item = g.get("item").and_then(Value::as_str).and_then(|i| p.item(i));
                let done = item.map_or(0, |i| s.produced.get(i));
                let ok = num(g.get("quantidade")).is_some_and(|q| done as f64 >= q);
                r.insert("feito".into(), done.into());
                r.insert("ok".into(), ok.into());
            } else {
                let k = mark_key(g);
                let now = count_mark(s, p, &k);
                let start = s.mission.refs.iter().find(|(r, _)| *r == k).map_or(0, |(_, n)| *n);
                let start = if start == 0 { 1 } else { start };
                let pct = (now * 100).div_euclid(start);
                let ok = num(g.get("minimoPct")).is_some_and(|q| pct as f64 >= q);
                r.insert("atual".into(), now.into());
                r.insert("pct".into(), pct.into());
                r.insert("ok".into(), ok.into());
            }
            Value::Object(r)
        })
        .collect()
}

fn check_mission(s: &mut State, p: &Package) {
    if s.mission.status != "andamento" {
        return;
    }
    let progress = mission_progress(s, p);
    let ok = |g: &Value| g.get("ok") == Some(&Value::Bool(true));
    let failed = progress.iter().position(|g| g.get("tipo").and_then(Value::as_str) == Some("cobertura") && !ok(g));
    if let Some(k) = failed {
        let g = &progress[k];
        s.mission.status = "falhou".into();
        let name = g.get("nome").and_then(Value::as_str).unwrap_or("");
        let pct = number_text(g.get("minimoPct"));
        let text = format!("Missão perdida: {} caiu abaixo de {pct}%.", name.to_lowercase());
        let goal = text_ref_or(&["missao", "metas", &k.to_string(), "nome"], name);
        let message = msg("log.missionFailed", json!({ "goal": goal, "pct": g.get("minimoPct") }));
        s.record(text, "ruim", message);
        push_event(s, json!({ "type": "mission", "status": "falhou", "goal": k, "pct": g.get("minimoPct") }));
    } else if progress.iter().all(ok) {
        s.mission.status = "cumprida".into();
        let text = format!("Missão cumprida: {}!", p.mission_name);
        s.record(text, "bom", msg("log.missionDone", json!({ "mission": text_ref(&["missao", "nome"]) })));
        push_event(s, json!({ "type": "mission", "status": "cumprida" }));
    }
}

// ---------- commands: the only way to change the world. A failed command changes nothing.
pub fn apply(s: &mut State, p: &Package, cmd: &Value) -> Value {
    match cmd.get("tipo").and_then(Value::as_str) {
        Some("ordem") => player::order_cmd(s, p, cmd),
        Some("cancelar") => player::cancel(s),
        Some("fabricar") => craft(s, p, cmd),
        Some("pesquisar") => research(s, p, cmd),
        _ => {
            let t = match cmd.get("tipo") {
                Some(Value::String(t)) => t.clone(),
                Some(v) => v.to_string(),
                None => "undefined".into(),
            };
            reply(false, Some(msg("error.unknownCommand", json!({ "type": t }))))
        }
    }
}

fn craft(s: &mut State, p: &Package, cmd: &Value) -> Value {
    let r = cmd.get("receita").and_then(Value::as_str).and_then(|r| p.recipe(r)).map(|r| &p.recipes[r as usize]);
    let Some(r) = r.filter(|r| r.by_hand) else { return reply(false, Some(key("error.notByHand"))) };
    if !has_items(s, &r.inputs) {
        return reply(false, Some(msg("error.missing", json!({ "items": missing_items(s, p, &r.inputs) }))));
    }
    for (it, n) in &r.inputs {
        s.inv.add(*it, -n);
    }
    for (it, n) in &r.outputs {
        s.inv.add(*it, *n);
        s.produced.add(*it, *n);
    }
    reply(true, None)
}

fn research(s: &mut State, p: &Package, cmd: &Value) -> Value {
    let id = cmd.get("tecnologia").and_then(Value::as_str).and_then(|t| p.technology(t));
    let Some(id) = id.filter(|t| !s.techs.contains(t)) else { return reply(false, None) };
    let t = &p.technologies[id as usize];
    let missing: Vec<Value> = t
        .requires
        .iter()
        .filter(|r| !s.techs.contains(r))
        .map(|r| name_ref("tecnologias", &p.technologies[*r as usize].id))
        .collect();
    if !missing.is_empty() {
        return reply(false, Some(msg("error.researchFirst", json!({ "techs": list(missing) }))));
    }
    if !has_items(s, &t.cost) {
        return reply(false, Some(msg("error.missing", json!({ "items": missing_items(s, p, &t.cost) }))));
    }
    for (it, n) in &t.cost {
        s.inv.add(*it, -n);
    }
    s.techs.push(id);
    for el in &t.unlocks {
        if !s.unlocked.contains(el) {
            s.unlocked.push(*el);
        }
    }
    let text = format!("Pesquisa concluída: {}.", t.name);
    s.record(text, "bom", msg("log.researchDone", json!({ "tech": name_ref("tecnologias", &t.id) })));
    if t.opens_era != 0 && t.opens_era > s.era {
        s.era = t.opens_era;
        let k = (t.opens_era - 1) as usize;
        let name = p.eras.get(k).cloned().unwrap_or_default();
        let era = text_ref_or(&["eras", &k.to_string(), "nome"], &name);
        s.record(format!("Estamos entrando na era {name}!"), "bom", msg("log.newEra", json!({ "era": era })));
        push_event(s, json!({ "type": "era", "era": t.opens_era }));
    }
    reply(true, None)
}

// ---------- replay: redoes commands from a state
fn tick_of(c: &Value) -> Option<f64> {
    num(c.get("t"))
}

/// Redoes the commands from a state, up to the given tick. Commands from
/// before the current tick are already in the state and are skipped.
pub fn advance(s: &mut State, p: &Package, commands: &[Value], until_tick: i64) {
    let mut k = 0;
    while k < commands.len() && tick_of(&commands[k]).is_some_and(|t| t < s.tick as f64) {
        k += 1;
    }
    let apply_this_tick = |s: &mut State, k: &mut usize| {
        while *k < commands.len() && tick_of(&commands[*k]) == Some(s.tick as f64) {
            apply(s, p, &commands[*k]);
            *k += 1;
        }
    };
    while s.tick < until_tick {
        apply_this_tick(s, &mut k);
        step(s, p);
    }
    apply_this_tick(s, &mut k);
    s.events.clear();
}

/// Rebuilds the world from the configuration and the command log.
pub fn replay(p: &Package, cfg: &Value, commands: &[Value], until_tick: i64) -> Result<State, String> {
    let mut s = new_state(p, cfg)?;
    advance(&mut s, p, commands, until_tick);
    Ok(s)
}
