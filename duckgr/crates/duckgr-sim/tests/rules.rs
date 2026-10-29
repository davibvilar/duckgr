//! Terra Viva rules reproduced from command sequences, and compatibility
//! with the saves written by the JavaScript core.

use duckgr_sim::state::Action;
use duckgr_sim::{advance, apply, compile, count_mark, new_state, replay, step, Package, State, TPS};
use serde_json::{json, Value};

fn data(name: &str) -> Value {
    let path = format!("{}/../../tests/data/{name}", env!("CARGO_MANIFEST_DIR"));
    let txt = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e} (run `npm run packages`)"));
    serde_json::from_str(&txt).expect("valid JSON")
}

fn terra_viva() -> Package {
    compile(&data("terra-viva.json")).expect("the package compiles").0
}

/// A test player: applies commands keeping the log, like the interface does.
struct Game {
    p: Package,
    s: State,
    cfg: Value,
    cmds: Vec<Value>,
}

impl Game {
    fn new(cfg: Value) -> Game {
        let p = terra_viva();
        let s = new_state(&p, &cfg).expect("initial state");
        Game { p, s, cfg, cmds: Vec::new() }
    }
    fn cmd(&mut self, mut c: Value) -> Value {
        c["t"] = self.s.tick.into();
        c["autor"] = "test".into();
        let r = apply(&mut self.s, &self.p, &c);
        if r["ok"] == true {
            self.cmds.push(c);
        }
        r
    }
    fn seconds(&mut self, s: i64) {
        for _ in 0..s * TPS {
            step(&mut self.s, &self.p);
        }
    }
    /// Gives an order and runs the world until the queue is empty; true if all went well.
    fn order(&mut self, order: Value) -> bool {
        self.s.events.clear();
        let mut c = json!({ "tipo": "ordem" });
        for (k, v) in order.as_object().unwrap() {
            c[k] = v.clone();
        }
        if self.cmd(c)["ok"] != true {
            return false;
        }
        for _ in 0..120 * TPS {
            if self.s.player.queue.is_empty() {
                break;
            }
            step(&mut self.s, &self.p);
        }
        self.s.player.queue.is_empty() && !self.s.events.iter().any(|e| e["type"] == "warning")
    }
    fn find(&self, test: impl Fn(usize, i64, i64) -> bool) -> Option<(i64, i64)> {
        for y in 0..self.s.h {
            for x in 0..self.s.w {
                if test(self.s.idx(x, y), x, y) {
                    return Some((x, y));
                }
            }
        }
        None
    }
    fn cov(&self, i: usize) -> Option<&str> {
        self.s.cov[i].map(|c| self.p.covers[c as usize].id.as_str())
    }
    fn item(&self, name: &str) -> i64 {
        self.s.inv.get(self.p.item(name).unwrap())
    }
    fn hash(&self) -> String {
        self.s.hash(&self.p)
    }
    /// A deposit of a kind with a free cell to its right (to put a machine next to it).
    fn deposit_with_free_neighbor(&self, kind: &str) -> (i64, i64) {
        let d = self.p.deposit(kind).unwrap();
        let soil = self.p.substrate("solo").unwrap();
        self.find(|i, x, _| {
            self.s.dep[i] == Some(d)
                && x + 1 < self.s.w
                && self.s.dep[i + 1].is_none()
                && self.s.sub[i + 1] == soil
                && self.s.cov[i + 1].is_none_or(|c| self.p.covers[c as usize].replaceable)
        })
        .expect("there is a deposit with a free neighbor")
    }
}

#[test]
fn the_package_compiles_and_converts_seconds_to_ticks() {
    let p = terra_viva();
    assert_eq!(p.id, "terra-viva");
    // 3.2 s × 30 ticks
    assert_eq!(p.recipes[p.recipe("placa-ferro").unwrap() as usize].ticks, 96);
}

#[test]
fn a_package_with_a_broken_reference_is_refused() {
    let mut v = data("terra-viva.json");
    v["receitas"]["placa-ferro"]["sai"] = json!({ "placa-de-ouro": 1 });
    let error = compile(&v).unwrap_err();
    assert!(error.contains("placa-de-ouro"), "{error}");
}

#[test]
fn the_same_seed_makes_the_same_map() {
    let p = terra_viva();
    let h = |s: i64| new_state(&p, &json!({ "semente": s })).unwrap().hash(&p);
    assert_eq!(h(7), h(7));
    assert_ne!(h(7), h(8));
}

#[test]
fn the_default_map_has_water_forest_and_deposits() {
    let g = Game::new(json!({}));
    let water = g.p.substrate("agua").unwrap();
    assert!(g.s.sub.contains(&water));
    assert!(count_mark(&g.s, &g.p, "floresta") > 100);
    for d in ["ferro", "cobre", "carvao", "pedra"] {
        assert!(g.s.dep.contains(&g.p.deposit(d)), "missing {d}");
    }
}

#[test]
fn determinism_redoing_the_commands_gives_the_same_hash() {
    let mut g = Game::new(json!({}));
    let (x, y) = g.find(|i, _, _| g.cov(i) == Some("adulta")).unwrap();
    g.cmd(json!({ "tipo": "ordem", "x": x, "y": y, "acao": "colher" }));
    g.seconds(20);
    let (x, y) = g.find(|i, _, _| g.cov(i) == Some("grama") && g.s.dep[i].is_none()).unwrap();
    g.cmd(json!({ "tipo": "ordem", "x": x, "y": y, "acao": "plantar", "item": "muda", "adicionar": true }));
    g.cmd(json!({ "tipo": "ordem", "x": 0, "y": 0, "acao": "ir", "adicionar": true }));
    g.seconds(40);
    let redone = replay(&g.p, &g.cfg, &g.cmds, g.s.tick).unwrap();
    assert_eq!(redone.hash(&g.p), g.hash());
}

#[test]
fn a_failed_command_does_not_change_the_world() {
    let mut g = Game::new(json!({}));
    let before = g.hash();
    let water = g.p.substrate("agua").unwrap();
    let (x, y) = g.find(|i, _, _| g.s.sub[i] == water).unwrap();
    let r = g.cmd(json!({ "tipo": "ordem", "x": x, "y": y, "acao": "construir", "elemento": "fornalha" }));
    assert_eq!(r["ok"], false);
    assert_eq!(r["msg"]["key"], "error.cannotBuildOn");
    assert_eq!(r["msg"]["args"]["substrate"]["ref"], json!(["substratos", "agua", "nome"]));
    assert_eq!(g.hash(), before);
}

#[test]
fn cutting_a_tree_gives_wood_and_a_sapling_and_leaves_a_stump() {
    let mut g = Game::new(json!({}));
    let (x, y) = g.find(|i, _, _| g.cov(i) == Some("adulta")).unwrap();
    let wood = g.item("madeira");
    assert!(g.order(json!({ "x": x, "y": y, "acao": "colher" })));
    assert_eq!(g.item("madeira"), wood + 4);
    assert_eq!(g.item("muda"), 5);
    assert_eq!(g.cov(g.s.idx(x, y)), Some("toco"));
}

#[test]
fn the_player_walks_to_the_target_before_acting() {
    let mut g = Game::new(json!({}));
    let (px, py) = (g.s.player.x, g.s.player.y);
    let (x, y) = g.find(|i, x, y| g.cov(i) == Some("adulta") && (x - px).abs().max((y - py).abs()) > 3).unwrap();
    g.cmd(json!({ "tipo": "ordem", "x": x, "y": y, "acao": "colher" }));
    assert_eq!(g.s.player.queue[0].action, Action::Harvest);
    g.seconds(30);
    assert!((g.s.player.x - x).abs().max((g.s.player.y - y).abs()) <= g.p.player.reach);
    assert_eq!(g.cov(g.s.idx(x, y)), Some("toco"));
}

#[test]
fn a_miner_next_to_a_furnace_makes_plates_by_itself() {
    let mut g = Game::new(json!({}));
    let (x, y) = g.deposit_with_free_neighbor("ferro");
    assert!(g.order(json!({ "x": x, "y": y, "acao": "construir", "elemento": "mineradora" })));
    assert!(g.order(json!({ "x": x + 1, "y": y, "acao": "construir", "elemento": "fornalha" })));
    assert!(g.order(json!({ "x": x, "y": y, "acao": "abastecer", "item": "carvao" })));
    assert!(g.order(json!({ "x": x + 1, "y": y, "acao": "abastecer", "item": "madeira" })));
    g.seconds(30);
    let plates = g.s.produced.get(g.p.item("placa-ferro").unwrap());
    assert!(plates >= 5, "made {plates}");
}

#[test]
fn machines_pollute_and_the_forest_gets_sick() {
    let mut g = Game::new(json!({ "recursosIniciais": 20 }));
    let ore = g.p.item("minerio-ferro").unwrap();
    g.s.inv.add(ore, 400);
    let mut n = 0;
    'outer: for y in 8..16 {
        for x in 16..24 {
            if n >= 10 {
                break 'outer;
            }
            if !g.order(json!({ "x": x, "y": y, "acao": "construir", "elemento": "fornalha" })) {
                continue;
            }
            g.order(json!({ "x": x, "y": y, "acao": "fornecer", "item": "minerio-ferro" }));
            g.order(json!({ "x": x, "y": y, "acao": "abastecer", "item": "carvao" }));
            n += 1;
        }
    }
    assert!(n >= 6, "built {n} furnaces");
    let forest = count_mark(&g.s, &g.p, "floresta");
    let mut peak = 0;
    for _ in 0..40 {
        g.seconds(1);
        peak = peak.max(g.s.fields[0].iter().copied().max().unwrap());
    }
    assert!(peak > 0, "the chunk got polluted");
    assert!(count_mark(&g.s, &g.p, "floresta") < forest, "trees got sick");
}

#[test]
fn research_unlocks_the_nursery_and_electricity_opens_era_2() {
    let mut g = Game::new(json!({}));
    g.s.inv.add(g.p.item("ciencia").unwrap(), 25);
    let r = g.cmd(json!({ "tipo": "pesquisar", "tecnologia": "eletricidade" }));
    assert_eq!(r["ok"], false);
    assert_eq!(r["msg"]["key"], "error.researchFirst");
    assert_eq!(g.cmd(json!({ "tipo": "pesquisar", "tecnologia": "viveiro" }))["ok"], true);
    assert!(g.s.unlocked.contains(&g.p.element("viveiro").unwrap()));
    assert_eq!(g.cmd(json!({ "tipo": "pesquisar", "tecnologia": "eletricidade" }))["ok"], true);
    assert_eq!(g.s.era, 2);
    // the log keeps the prototype's text (it enters the hash) and the message for the interface
    let entry = &g.s.log[0];
    assert_eq!(entry.text, "Estamos entrando na era Eletricidade!");
    assert_eq!(entry.msg["key"], "log.newEra");
}

#[test]
fn the_state_goes_to_json_and_back_without_losing_anything() {
    let mut g = Game::new(json!({}));
    let (x, y) = g.deposit_with_free_neighbor("ferro");
    g.order(json!({ "x": x, "y": y, "acao": "construir", "elemento": "mineradora" }));
    g.order(json!({ "x": x, "y": y, "acao": "abastecer", "item": "carvao" }));
    g.seconds(10);
    let json = g.s.view_json(&g.p);
    let mut back = State::from_json(&json, &g.p).unwrap();
    assert_eq!(back.hash(&g.p), g.hash());
    assert_eq!(back.view_json(&g.p), json);
    for _ in 0..300 {
        step(&mut back, &g.p);
        step(&mut g.s, &g.p);
    }
    assert_eq!(back.hash(&g.p), g.hash());
}

#[test]
fn messages_stay_out_of_the_hash() {
    let mut g = Game::new(json!({}));
    let (x, y) = g.find(|i, _, _| g.cov(i) == Some("grama") && g.s.dep[i].is_none()).unwrap();
    assert!(g.order(json!({ "x": x, "y": y, "acao": "construir", "elemento": "bau" })));
    let view = g.s.view_json(&g.p);
    assert_eq!(view["log"][0]["msg"]["key"], "log.built");
    assert!(g.s.to_json(&g.p)["log"][0].get("msg").is_none());
    let mut plain = g.s.clone();
    for e in plain.log.iter_mut() {
        e.msg = Value::Null;
    }
    assert_eq!(plain.hash(&g.p), g.hash());
}

#[test]
fn log_entries_without_messages_get_them_back_from_the_text() {
    let mut g = Game::new(json!({}));
    let (x, y) = g.find(|i, _, _| g.cov(i) == Some("grama") && g.s.dep[i].is_none()).unwrap();
    assert!(g.order(json!({ "x": x, "y": y, "acao": "construir", "elemento": "bau" })));
    assert!(g.order(json!({ "x": x, "y": y, "acao": "desmontar" })));
    g.s.inv.add(g.p.item("ciencia").unwrap(), 25);
    g.cmd(json!({ "tipo": "pesquisar", "tecnologia": "viveiro" }));
    g.cmd(json!({ "tipo": "pesquisar", "tecnologia": "eletricidade" }));
    // the hash form has no messages, like the checkpoints of old saves
    let old = State::from_json(&g.s.to_json(&g.p), &g.p).unwrap();
    assert_eq!(old.log.len(), 6);
    for (a, b) in old.log.iter().zip(&g.s.log) {
        assert_eq!(a.msg, b.msg, "{}", a.text);
    }
}

/// A save written by the JavaScript core (v0.4), with a 13-minute game:
/// machines, pollution that makes trees sick, research and a completed mission.
#[test]
fn a_save_written_by_the_javascript_core_opens_with_the_same_hash() {
    let p = terra_viva();
    let v = data("save-v0.4.json");
    let save = &v["save"];
    let history = save["historico"].as_array().unwrap();
    // from the checkpoint, redoing only the later commands
    let cp = &save["checkpoint"];
    let mut s = State::from_json(&cp["estado"], &p).unwrap();
    // the old log only had Portuguese text; the core recognizes it and adds the messages
    assert!(s.log.iter().all(|e| !e.msg.is_null()), "every old log entry got a message");
    let after: Vec<Value> = history.iter().filter(|c| c["t"].as_i64() >= cp["tick"].as_i64()).cloned().collect();
    advance(&mut s, &p, &after, save["tick"].as_i64().unwrap());
    assert_eq!(s.hash(&p), save["hash"].as_str().unwrap());
    // from zero, checking the hash at several moments of the game
    let mut z = new_state(&p, &save["cfg"]).unwrap();
    for m in v["milestones"].as_array().unwrap() {
        let t = m["tick"].as_i64().unwrap();
        let before: Vec<Value> = history.iter().filter(|c| c["t"].as_i64().unwrap() < t).cloned().collect();
        advance(&mut z, &p, &before, t);
        assert_eq!(z.hash(&p), m["hash"].as_str().unwrap(), "tick {t}");
    }
}

/// Performance budget: up to 10 ms per tick on Terra Viva's largest map.
#[test]
fn performance_budget() {
    let mut g = Game::new(json!({ "largura": 56, "altura": 32, "recursosIniciais": 20 }));
    for kind in ["ferro", "cobre", "carvao"] {
        let (x, y) = g.deposit_with_free_neighbor(kind);
        g.order(json!({ "x": x, "y": y, "acao": "construir", "elemento": "mineradora" }));
        g.order(json!({ "x": x + 1, "y": y, "acao": "construir", "elemento": "fornalha" }));
        g.order(json!({ "x": x, "y": y, "acao": "abastecer", "item": "carvao" }));
    }
    let ticks = 60 * TPS;
    let start = std::time::Instant::now();
    g.seconds(60);
    let ms = start.elapsed().as_secs_f64() * 1000.0 / ticks as f64;
    println!("{ms:.4} ms per tick");
    assert!(ms < 10.0, "{ms} ms per tick");
}
