//! The duck's flight: a straight jump of exactly 3 cells (also diagonal) in
//! the time of one step, from clean ground to clean ground, over water but
//! not over trees or machines.

use duckgr_sim::player::{move_cost, path};
use duckgr_sim::state::{Action, Order};
use duckgr_sim::{compile, new_state, step, Package, State, TPS};
use serde_json::{json, Value};

fn data(name: &str) -> Value {
    let path = format!("{}/../../tests/data/{name}", env!("CARGO_MANIFEST_DIR"));
    let txt = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e} (run `npm run packages`)"));
    serde_json::from_str(&txt).expect("valid JSON")
}

/// A 20×9 world of bare soil, with the duck at (2, 4).
fn flat_world() -> (Package, State) {
    let p = compile(&data("terra-viva.json")).expect("the package compiles").0;
    let mut s = new_state(&p, &json!({ "largura": 20, "altura": 9 })).unwrap();
    let soil = p.substrate("solo").unwrap();
    for i in 0..(s.w * s.h) as usize {
        s.sub[i] = soil;
        s.cov[i] = None;
        s.dep[i] = None;
        s.occ[i] = 0;
    }
    s.player.x = 2;
    s.player.y = 4;
    (p, s)
}

fn set_sub(p: &Package, s: &mut State, x: i64, y: i64, id: &str) {
    let i = s.idx(x, y);
    s.sub[i] = p.substrate(id).unwrap();
}

fn set_cov(p: &Package, s: &mut State, x: i64, y: i64, id: &str) {
    let i = s.idx(x, y);
    s.cov[i] = Some(p.cover(id).unwrap());
}

/// A column of deep water, which the duck can't walk through.
fn river(p: &Package, s: &mut State, xs: &[i64]) {
    for &x in xs {
        for y in 0..s.h {
            set_sub(p, s, x, y, "agua");
        }
    }
}

fn go(x: i64, y: i64) -> Order {
    Order { x, y, action: Action::Go, item: None, element: None, times: None, done: None }
}

/// Cells of a path as (x, y).
fn cells(s: &State, route: &[i64]) -> Vec<(i64, i64)> {
    route.iter().map(|c| (c % s.w, c / s.w)).collect()
}

/// Gives the duck an order to go to (x, y) and counts the ticks until it
/// gets there (the order is done on the tick after the last move).
fn walk_to(p: &Package, s: &mut State, x: i64, y: i64) -> i64 {
    s.player.queue.push(go(x, y));
    for t in 1..60 * TPS {
        step(s, p);
        if s.player.queue.is_empty() {
            return t - 1;
        }
    }
    panic!("the duck did not get to ({x}, {y})");
}

#[test]
fn on_open_ground_the_duck_flies_three_cells_per_step() {
    let (p, s) = flat_world();
    let route = path(&s, &p, 2, 4, &go(14, 4)).unwrap();
    assert_eq!(cells(&s, &route), vec![(5, 4), (8, 4), (11, 4), (14, 4)]);
    // and diagonally too
    let route = path(&s, &p, 2, 1, &go(8, 7)).unwrap();
    assert_eq!(cells(&s, &route), vec![(5, 4), (8, 7)]);
}

#[test]
fn a_flight_takes_the_time_of_one_step() {
    let (p, mut s) = flat_world();
    let tpc = p.player.ticks_per_cell;
    // 12 cells: 4 flights instead of 12 steps
    assert_eq!(walk_to(&p, &mut s, 14, 4), 4 * tpc);
    assert_eq!((s.player.x, s.player.y), (14, 4));
    // 2 cells: no flight lands there, so it walks
    assert_eq!(walk_to(&p, &mut s, 16, 4), 2 * tpc);
}

#[test]
fn the_path_mixes_flights_and_steps() {
    let (p, s) = flat_world();
    // 7 cells: two flights and a step
    let route = path(&s, &p, 2, 4, &go(9, 4)).unwrap();
    assert_eq!(route.len(), 3);
    assert_eq!(cells(&s, &route).last(), Some(&(9, 4)));
}

#[test]
fn the_duck_flies_over_water_to_dry_land() {
    let (p, mut s) = flat_world();
    river(&p, &mut s, &[6, 7]);
    // walking it can't cross deep water, but flying from 5 it lands on 8
    let route = path(&s, &p, 5, 4, &go(8, 4)).unwrap();
    assert_eq!(cells(&s, &route), vec![(8, 4)]);
    let tpc = p.player.ticks_per_cell;
    s.player.x = 5;
    assert_eq!(walk_to(&p, &mut s, 8, 4), tpc);
    assert_eq!((s.player.x, s.player.y), (8, 4));
}

#[test]
fn the_duck_does_not_land_on_water() {
    let (p, mut s) = flat_world();
    // a river 3 cells wide: from 5 it would land on 8, which is water
    river(&p, &mut s, &[6, 7, 8]);
    assert_eq!(move_cost(&s, &p, 5, 4, 8, 4), -1);
    // but from 4 (or diagonally) it can't get over either: no flight is 4 cells
    assert!(path(&s, &p, 5, 4, &go(9, 4)).is_none(), "no way across a 3-wide river");
}

#[test]
fn shallow_water_is_not_clean_ground_for_takeoff_or_landing() {
    let (p, mut s) = flat_world();
    set_sub(&p, &mut s, 8, 4, "vau");
    assert_eq!(move_cost(&s, &p, 5, 4, 8, 4), -1, "can't land on the ford");
    assert_eq!(move_cost(&s, &p, 8, 4, 11, 4), -1, "can't take off from the ford");
    // flying over it is fine
    assert_eq!(move_cost(&s, &p, 7, 4, 10, 4), 1);
}

#[test]
fn trees_block_the_flight() {
    let (p, mut s) = flat_world();
    set_cov(&p, &mut s, 6, 4, "adulta");
    assert_eq!(move_cost(&s, &p, 5, 4, 8, 4), -1, "can't fly over a tree");
    set_cov(&p, &mut s, 6, 4, "jovem");
    assert_eq!(move_cost(&s, &p, 5, 4, 8, 4), -1, "nor over a young tree");
    // a tree under the landing or the takeoff cell blocks too
    let (p, mut s) = flat_world();
    set_cov(&p, &mut s, 8, 4, "adulta");
    assert_eq!(move_cost(&s, &p, 5, 4, 8, 4), -1);
    assert_eq!(move_cost(&s, &p, 8, 4, 11, 4), -1);
    // low covers (grass, sapling, stump) don't
    let (p, mut s) = flat_world();
    for (x, c) in [(5, "grama"), (6, "muda"), (7, "toco"), (8, "grama")] {
        set_cov(&p, &mut s, x, 4, c);
    }
    assert_eq!(move_cost(&s, &p, 5, 4, 8, 4), 1);
}

#[test]
fn a_tree_line_over_water_leaves_no_way_across() {
    let (p, mut s) = flat_world();
    river(&p, &mut s, &[6, 7]);
    for y in 0..s.h {
        set_cov(&p, &mut s, 5, y, "adulta");
    }
    // the trees on the bank: it walks through them, but can't take off from them
    assert!(path(&s, &p, 2, 4, &go(10, 4)).is_none());
}

#[test]
fn machines_block_the_flight() {
    let (p, mut s) = flat_world();
    let i = s.idx(7, 4);
    s.occ[i] = 1;
    assert_eq!(move_cost(&s, &p, 5, 4, 8, 4), -1);
    // the path goes around: diagonal flights, steps, whatever is fastest
    let route = path(&s, &p, 5, 4, &go(8, 4)).unwrap();
    assert!(!cells(&s, &route).contains(&(7, 4)));
}

#[test]
fn a_flight_that_gets_blocked_on_the_way_is_recomputed() {
    let (p, mut s) = flat_world();
    s.player.queue.push(go(14, 4));
    step(&mut s, &p); // computes the path: 5, 8, 11, 14
    assert_eq!(cells(&s, &s.player.path), vec![(5, 4), (8, 4), (11, 4), (14, 4)]);
    // a tree grows under the first landing cell
    set_cov(&p, &mut s, 5, 4, "adulta");
    for _ in 0..30 * TPS {
        if s.player.queue.is_empty() {
            break;
        }
        step(&mut s, &p);
    }
    assert_eq!((s.player.x, s.player.y), (14, 4));
}

#[test]
fn without_flight_in_the_package_the_duck_only_walks() {
    let mut v = data("terra-viva.json");
    v["jogador"].as_object_mut().unwrap().remove("voo");
    let p = compile(&v).unwrap().0;
    let (_, s) = flat_world();
    let route = path(&s, &p, 2, 4, &go(14, 4)).unwrap();
    assert_eq!(route.len(), 12);
}

#[test]
fn flying_is_deterministic() {
    let run = || {
        let (p, mut s) = flat_world();
        river(&p, &mut s, &[9, 10]);
        set_cov(&p, &mut s, 4, 3, "adulta");
        walk_to(&p, &mut s, 17, 7);
        walk_to(&p, &mut s, 1, 1);
        s.hash(&p)
    };
    assert_eq!(run(), run());
}

#[test]
fn a_package_with_a_bad_flight_is_refused() {
    let mut v = data("terra-viva.json");
    v["jogador"]["voo"]["celulas"] = json!(1);
    assert!(compile(&v).unwrap_err().contains("voo"));
}
