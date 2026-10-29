//! The player: an agent with a position, a speed and a reach. It only acts
//! on what is within reach; for the rest, it walks there. The player gets
//! orders (command "ordem") that go into a queue; each order is "go near
//! this cell and do this action". Walking, waiting and acting happen tick
//! by tick, so everything stays deterministic and enters the replay.

use serde_json::{json, Map, Value};

use crate::js::{lower_first, nonempty_str, num, to_int32, truthy};
use crate::message::{element_ref, item_ref, items, key, msg, name_ref, text_ref, text_ref_or};
use crate::package::{Id, ItemList, Package, Passage};
use crate::state::{Action, Bag, Entity, Order, State};
use crate::{has_items, missing_items, push_event, NEIGHBORS};

/// How much it costs (in multiples of a step) to enter a cell; -1 blocks.
pub fn passage_cost(s: &State, p: &Package, x: i64, y: i64) -> i64 {
    if !s.inside(x, y) {
        return -1;
    }
    let i = s.idx(x, y);
    if s.occ[i] != 0 {
        return -1;
    }
    let mut cost = 1;
    let sub = Some(p.substrates[s.sub[i] as usize].passage);
    let cov = s.cov[i].map(|c| p.covers[c as usize].passage);
    for passage in [sub, cov].into_iter().flatten() {
        match passage {
            Passage::Blocked => return -1,
            Passage::Cost(n) => cost *= n,
            Passage::Free => {}
        }
    }
    cost
}

/// Starting position: the walkable cell closest to the map's center.
pub fn start_position(s: &State, p: &Package) -> (i64, i64) {
    let (cx, cy) = (s.w / 2, s.h / 2);
    for r in 0..s.w.max(s.h) {
        for dy in -r..=r {
            for dx in -r..=r {
                if dx.abs().max(dy.abs()) != r {
                    continue;
                }
                if passage_cost(s, p, cx + dx, cy + dy) == 1 {
                    return (cx + dx, cy + dy);
                }
            }
        }
    }
    (0, 0)
}

/// Can the order be done from (x, y)?
pub fn in_reach(p: &Package, x: i64, y: i64, o: &Order) -> bool {
    match o.action {
        Action::Go => x == o.x && y == o.y,
        Action::Build if x == o.x && y == o.y => false, // doesn't build where it stands
        _ => (x - o.x).abs().max((y - o.y).abs()) <= p.player.reach,
    }
}

/// Cheapest path (Dijkstra with buckets; neighbors in a fixed order =
/// deterministic) to any cell from where the order is within reach.
/// Returns the cell indices to walk (without the current cell): empty if
/// already there, `None` if there is no path.
pub fn path(s: &State, p: &Package, px: i64, py: i64, o: &Order) -> Option<Vec<i64>> {
    let target_blocked = o.action == Action::Go && passage_cost(s, p, o.x, o.y) < 0;
    let ok = |x: i64, y: i64| in_reach(p, x, y, o) || (target_blocked && (x - o.x).abs().max((y - o.y).abs()) <= 1);
    if ok(px, py) {
        return Some(Vec::new());
    }
    let n = (s.w * s.h) as usize;
    let mut dist = vec![i64::MAX; n];
    let mut from = vec![usize::MAX; n];
    let start = s.idx(px, py);
    dist[start] = 0;
    let mut buckets: Vec<Vec<usize>> = vec![vec![start]];
    let mut d = 0;
    while d < buckets.len() {
        let mut k = 0;
        while k < buckets[d].len() {
            let i = buckets[d][k];
            k += 1;
            if dist[i] != d as i64 {
                continue;
            }
            let (x, y) = (i as i64 % s.w, i as i64 / s.w);
            if i != start && ok(x, y) {
                let mut route = Vec::new();
                let mut c = i;
                while c != start {
                    route.push(c as i64);
                    c = from[c];
                }
                route.reverse();
                return Some(route);
            }
            for (dx, dy) in NEIGHBORS {
                let c = passage_cost(s, p, x + dx, y + dy);
                if c < 0 {
                    continue;
                }
                let j = s.idx(x + dx, y + dy);
                let nd = d as i64 + c;
                if nd < dist[j] {
                    dist[j] = nd;
                    from[j] = i;
                    if buckets.len() <= nd as usize {
                        buckets.resize(nd as usize + 1, Vec::new());
                    }
                    buckets[nd as usize].push(j);
                }
            }
        }
        d += 1;
    }
    None
}

// ---------- actions: validate (without looking at the reach) and execute

/// What the action gained or spent (for the floating "+1 madeira" texts).
#[derive(Default)]
struct Outcome {
    gain: Option<ItemList>,
    loss: Option<ItemList>,
}

fn item_of(p: &Package, o: &Order) -> Option<Id> {
    o.item.as_deref().and_then(|i| p.item(i))
}

/// The name of the order's item, also for items the package doesn't have.
fn order_item_ref(o: &Order) -> Value {
    let id = o.item.as_deref().unwrap_or("undefined");
    text_ref_or(&["itens", id, "nome"], id)
}

/// Why the order can't be done here, as a message; `None` if it can.
fn validate(s: &State, p: &Package, o: &Order) -> Option<Value> {
    let i = s.idx(o.x, o.y);
    match o.action {
        Action::Go => None,
        Action::Harvest => match s.cov[i] {
            Some(c) if p.covers[c as usize].harvest.is_some() => None,
            _ => Some(key("error.nothingToHarvest")),
        },
        Action::Mine => {
            if s.dep[i].is_some() && s.occ[i] == 0 {
                None
            } else if s.occ[i] != 0 {
                Some(key("error.machineOnTop"))
            } else {
                Some(key("error.noDeposit"))
            }
        }
        Action::Plant => {
            let it = item_of(p, o).filter(|it| p.items[*it as usize].plants.is_some());
            let Some(it) = it else {
                return Some(key("error.cannotPlantThis"));
            };
            if s.inv.get(it) == 0 {
                return Some(msg("error.youHaveNone", json!({ "item": item_ref(p, it) })));
            }
            if s.occ[i] != 0 {
                return Some(key("error.machineHere"));
            }
            let sub = &p.substrates[s.sub[i] as usize];
            if !sub.plantable {
                return Some(msg("error.cannotPlantOn", json!({ "substrate": name_ref("substratos", &sub.id) })));
            }
            match s.cov[i] {
                Some(c) if !p.covers[c as usize].replaceable => Some(key("error.somethingGrowing")),
                _ => None,
            }
        }
        Action::Build => {
            let el = o.element.as_deref().and_then(|e| p.element(e));
            let Some(el) = el.filter(|e| s.unlocked.contains(e)) else {
                return Some(key("error.notResearched"));
            };
            let proto = &p.elements[el as usize];
            if s.occ[i] != 0 {
                return Some(key("error.alreadyMachine"));
            }
            let sub = &p.substrates[s.sub[i] as usize];
            if !sub.buildable {
                return Some(msg("error.cannotBuildOn", json!({ "substrate": name_ref("substratos", &sub.id) })));
            }
            if let Some(c) = s.cov[i] {
                let c = &p.covers[c as usize];
                if !c.replaceable {
                    return Some(msg("error.clearFirst", json!({ "cover": name_ref("coberturas", &c.id) })));
                }
            }
            if proto.needs_deposit && s.dep[i].is_none() {
                return Some(msg("error.needsDeposit", json!({ "element": element_ref(p, el) })));
            }
            if !has_items(s, &proto.cost) {
                return Some(msg("error.missing", json!({ "items": missing_items(s, p, &proto.cost) })));
            }
            None
        }
        Action::Dismantle => s.entity_at(o.x, o.y).is_none().then(|| key("error.noMachine")),
        Action::Collect => match s.entity_at(o.x, o.y) {
            None => Some(key("error.noMachine")),
            Some(k) if s.entities[k].output.total() != 0 => None,
            Some(_) => Some(key("error.nothingToCollect")),
        },
        Action::Refuel | Action::Supply => {
            let Some(k) = s.entity_at(o.x, o.y) else {
                return Some(key("error.noMachine"));
            };
            let it = item_of(p, o).filter(|it| s.inv.get(*it) != 0);
            let Some(it) = it else {
                return Some(msg("error.youHaveNone", json!({ "item": order_item_ref(o) })));
            };
            let e = &s.entities[k];
            if accepts(p, e, it) != Some(slot_of(o.action)) {
                let args = json!({ "element": element_ref(p, e.proto), "item": order_item_ref(o) });
                return Some(msg("error.doesNotAccept", args));
            }
            None
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Slot {
    Input,
    Fuel,
    Output,
}

fn slot_of(a: Action) -> Slot {
    if a == Action::Refuel {
        Slot::Fuel
    } else {
        Slot::Input
    }
}

pub fn bag_mut(e: &mut Entity, slot: Slot) -> &mut Bag {
    match slot {
        Slot::Input => &mut e.input,
        Slot::Fuel => &mut e.fuel,
        Slot::Output => &mut e.output,
    }
}

/// In which slot does the element accept this item?
pub fn accepts(p: &Package, e: &Entity, item: Id) -> Option<Slot> {
    let proto = &p.elements[e.proto as usize];
    let below = |total: i64, cap: Option<i64>| cap.is_some_and(|c| total < c);
    if let Some(recipes) = &proto.process {
        let uses = recipes.iter().any(|r| p.recipes[*r as usize].inputs.iter().any(|(it, n)| *it == item && *n != 0));
        if uses && below(e.input.total(), proto.inv.input) {
            return Some(Slot::Input);
        }
    }
    if proto.consumption.is_some() && p.items[item as usize].fuel != 0 && below(e.fuel.total(), proto.inv.fuel) {
        return Some(Slot::Fuel);
    }
    if proto.inv.storage.is_some_and(|g| g != 0) && below(e.output.total(), proto.inv.storage) {
        return Some(Slot::Output);
    }
    None
}

fn execute(s: &mut State, p: &Package, o: &Order) -> Outcome {
    let i = s.idx(o.x, o.y);
    match o.action {
        Action::Go => Outcome::default(),
        Action::Harvest => {
            let harvest = p.covers[s.cov[i].expect("validated") as usize].harvest.as_ref().expect("validated");
            for (it, n) in &harvest.gives {
                s.inv.add(*it, *n);
            }
            s.cov[i] = harvest.becomes;
            s.growth[i] = 0;
            Outcome { gain: Some(harvest.gives.clone()), loss: None }
        }
        Action::Mine => {
            let it = p.deposits[s.dep[i].expect("validated") as usize].item;
            s.inv.add(it, 1);
            s.produced.add(it, 1);
            s.dep_amount[i] -= 1;
            if s.dep_amount[i] <= 0 {
                s.dep[i] = None;
                s.dep_amount[i] = 0;
            }
            Outcome { gain: Some(vec![(it, 1)]), loss: None }
        }
        Action::Plant => {
            let it = item_of(p, o).expect("validated");
            s.inv.add(it, -1);
            s.cov[i] = p.items[it as usize].plants;
            s.growth[i] = 0;
            Outcome::default()
        }
        Action::Build => {
            let el = o.element.as_deref().and_then(|e| p.element(e)).expect("validated");
            let proto = &p.elements[el as usize];
            for (it, n) in &proto.cost {
                s.inv.add(*it, -n);
            }
            let e = Entity {
                id: s.next_id,
                proto: el,
                x: o.x,
                y: o.y,
                input: Bag::default(),
                output: Bag::default(),
                fuel: Bag::default(),
                burn: 0,
                progress: 0,
                recipe: None,
                active: 0,
                stopped: String::new(),
                stopped_msg: Value::Null,
            };
            s.next_id += 1;
            s.occ[i] = e.id;
            s.entities.push(e);
            if s.cov[i].is_some() {
                s.cov[i] = None;
            }
            let text = format!("Construiu {} em ({},{}).", lower_first(&proto.name), o.x, o.y);
            s.record(text, "", msg("log.built", json!({ "element": element_ref(p, el), "x": o.x, "y": o.y })));
            Outcome::default()
        }
        Action::Dismantle => {
            let k = s.entity_at(o.x, o.y).expect("validated");
            let e = s.entities.remove(k);
            let proto = &p.elements[e.proto as usize];
            let mut gain = Bag::default();
            for (it, n) in proto.cost.iter().chain(&e.input.0).chain(&e.output.0).chain(&e.fuel.0) {
                s.inv.add(*it, *n);
                gain.add(*it, *n);
            }
            s.occ[i] = 0;
            let text = format!("Desmontou {} em ({},{}).", lower_first(&proto.name), o.x, o.y);
            s.record(
                text,
                "",
                msg("log.dismantled", json!({ "element": element_ref(p, e.proto), "x": o.x, "y": o.y })),
            );
            Outcome { gain: Some(gain.0), loss: None }
        }
        Action::Collect => {
            let k = s.entity_at(o.x, o.y).expect("validated");
            let output = std::mem::take(&mut s.entities[k].output);
            for (it, n) in &output.0 {
                s.inv.add(*it, *n);
            }
            Outcome { gain: Some(output.0), loss: None }
        }
        Action::Refuel | Action::Supply => {
            let k = s.entity_at(o.x, o.y).expect("validated");
            let it = item_of(p, o).expect("validated");
            let slot = slot_of(o.action);
            let proto = &p.elements[s.entities[k].proto as usize];
            let cap = if slot == Slot::Fuel { proto.inv.fuel } else { proto.inv.input }.unwrap_or(0);
            let n = s.inv.get(it).min(cap - bag_mut(&mut s.entities[k], slot).total());
            s.inv.add(it, -n);
            bag_mut(&mut s.entities[k], slot).add(it, n);
            Outcome { gain: None, loss: Some(vec![(it, n)]) }
        }
    }
}

fn items_json(p: &Package, list: &[(Id, i64)]) -> Value {
    let mut m = Map::new();
    for (it, n) in list {
        m.insert(p.items[*it as usize].id.clone(), (*n).into());
    }
    Value::Object(m)
}

// ---------- what can be done on a cell, with labels (for the board menu)
pub fn possible_actions(s: &State, p: &Package, x: i64, y: i64) -> Value {
    if !s.inside(x, y) {
        return json!({ "actions": [], "notes": [] });
    }
    let i = s.idx(x, y);
    let mut actions = Vec::new();
    let mut notes = Vec::new();
    let note = |kind: &str, msg: Value| json!({ "kind": kind, "msg": msg });
    let mut try_order = |o: Order, label: Value, detail: Option<Value>| {
        if validate(s, p, &o).is_none() {
            let mut v = o.to_json();
            let m = v.as_object_mut().expect("object");
            m.insert("label".into(), label);
            if let Some(d) = detail {
                m.insert("detail".into(), d);
            }
            actions.push(v);
        }
    };
    let order = |action: Action| Order { x, y, action, item: None, element: None, times: None, done: None };
    let item_order = |action: Action, it: Id| Order { item: Some(p.items[it as usize].id.clone()), ..order(action) };
    let title;
    if let Some(k) = s.entity_at(x, y) {
        let e = &s.entities[k];
        let proto = &p.elements[e.proto as usize];
        if e.output.total() != 0 {
            try_order(order(Action::Collect), msg("action.collect", json!({ "items": items(p, &e.output.0) })), None);
        }
        if proto.consumption.is_some() {
            let room = proto.inv.fuel.unwrap_or(0) - e.fuel.total();
            // the strongest fuel the player has (tie: the first in the package)
            let mut fuel: Option<Id> = None;
            for (k, it) in p.items.iter().enumerate() {
                if it.fuel != 0 && s.inv.get(k as Id) != 0 && fuel.is_none_or(|c| it.fuel > p.items[c as usize].fuel) {
                    fuel = Some(k as Id);
                }
            }
            if e.fuel.total() == 0 && e.burn == 0 {
                notes.push(note("bad", key("note.noFuel")));
            } else if room > 0 {
                notes.push(note("", msg("note.fuelRoom", json!({ "n": room }))));
            }
            if let Some(c) = fuel.filter(|_| room > 0) {
                let args = json!({ "n": room.min(s.inv.get(c)), "item": item_ref(p, c) });
                try_order(item_order(Action::Refuel, c), msg("action.refuel", args), None);
            }
        }
        if let Some(recipes) = &proto.process {
            let room = proto.inv.input.unwrap_or(0) - e.input.total();
            // what is already in the machine; otherwise, the input the player has most of
            let uses =
                |it: Id| recipes.iter().any(|r| p.recipes[*r as usize].inputs.iter().any(|(x, n)| *x == it && *n != 0));
            let mut input = e.input.first().filter(|it| s.inv.get(*it) != 0);
            if input.is_none() {
                for k in 0..p.items.len() as Id {
                    if s.inv.get(k) != 0 && uses(k) && input.is_none_or(|c| s.inv.get(k) > s.inv.get(c)) {
                        input = Some(k);
                    }
                }
            }
            if e.input.total() == 0 && e.recipe.is_none() {
                notes.push(note("bad", key("note.noInput")));
            }
            if let Some(it) = input.filter(|_| room > 0) {
                let args = json!({ "n": room.min(s.inv.get(it)), "item": item_ref(p, it) });
                try_order(item_order(Action::Supply, it), msg("action.supply", args), None);
            }
        }
        if !e.stopped.is_empty() && e.stopped != STOPPED_NO_FUEL && e.stopped != STOPPED_NO_INPUT {
            let m = if e.stopped_msg.is_null() { Value::from(e.stopped.clone()) } else { e.stopped_msg.clone() };
            notes.push(note("bad", m));
        }
        try_order(
            order(Action::Dismantle),
            msg("action.dismantle", json!({ "element": element_ref(p, e.proto) })),
            Some(key("action.dismantleDetail")),
        );
        title = element_ref(p, e.proto);
    } else {
        let c = s.cov[i];
        let d = s.dep[i];
        if let Some(cover) = c.map(|c| &p.covers[c as usize]) {
            if let Some(harvest) = &cover.harvest {
                let verb = match harvest.verb {
                    Some(_) => text_ref(&["coberturas", &cover.id, "colheita", "verbo"]),
                    None => key("action.harvestVerb"),
                };
                let args = json!({ "verb": verb, "cover": name_ref("coberturas", &cover.id) });
                let detail = msg("action.gain", json!({ "items": items(p, &harvest.gives) }));
                try_order(order(Action::Harvest), msg("action.harvest", args), Some(detail));
            }
        }
        if let Some(d) = d {
            let times = p.player.mine_times.min(s.dep_amount[i]);
            let o = Order { times: Some(times), ..order(Action::Mine) };
            let args = json!({ "deposit": name_ref("depositos", &p.deposits[d as usize].id), "n": times });
            let detail = msg("action.remaining", json!({ "n": s.dep_amount[i] }));
            try_order(o, msg("action.mine", args), Some(detail));
        }
        let plantable = (0..p.items.len() as Id).find(|k| p.items[*k as usize].plants.is_some() && s.inv.get(*k) != 0);
        if let Some(it) = plantable {
            let detail = msg("action.youHave", json!({ "n": s.inv.get(it) }));
            try_order(
                item_order(Action::Plant, it),
                msg("action.plant", json!({ "item": item_ref(p, it) })),
                Some(detail),
            );
        }
        for (k, el) in p.elements.iter().enumerate() {
            if s.unlocked.contains(&(k as Id)) {
                let o = Order { element: Some(el.id.clone()), ..order(Action::Build) };
                let label = msg("action.build", json!({ "element": element_ref(p, k as Id) }));
                try_order(o, label, Some(msg("action.cost", json!({ "items": items(p, &el.cost) }))));
            }
        }
        if passage_cost(s, p, x, y) > 0 {
            try_order(order(Action::Go), key("action.go"), None);
        }
        title = match (c, d) {
            (Some(c), _) => name_ref("coberturas", &p.covers[c as usize].id),
            (None, Some(d)) => name_ref("depositos", &p.deposits[d as usize].id),
            _ => name_ref("substratos", &p.substrates[s.sub[i] as usize].id),
        };
    }
    json!({ "actions": actions, "notes": notes, "title": title })
}

pub(crate) const STOPPED_NO_FUEL: &str = "Sem combustível";
pub(crate) const STOPPED_NO_INPUT: &str = "Sem entrada";

// ---------- one tick of the player
pub fn player_step(s: &mut State, p: &Package) {
    let Some(o) = s.player.queue.first().cloned() else { return };
    if !in_reach(p, s.player.x, s.player.y, &o) {
        s.player.task = 0;
        let mut next = s.player.path.first().copied();
        let cost_of = |s: &State, c: i64| passage_cost(s, p, c % s.w, c / s.w);
        let mut cost = next.map_or(-1, |c| cost_of(s, c));
        if cost < 0 {
            // no path yet, or the path got blocked: compute it again
            match path(s, p, s.player.x, s.player.y, &o) {
                Some(route) if !route.is_empty() => {
                    next = Some(route[0]);
                    cost = cost_of(s, route[0]);
                    s.player.path = route;
                    s.player.step = 0;
                }
                _ => return fail(s, key("error.noPath")),
            }
        }
        let next = next.expect("there is a next step");
        s.player.step += 1;
        if s.player.step >= p.player.ticks_per_cell * cost {
            s.player.step = 0;
            s.player.path.remove(0);
            s.player.x = next % s.w;
            s.player.y = next / s.w;
        }
        return;
    }
    s.player.path.clear();
    s.player.step = 0;
    if let Some(error) = validate(s, p, &o) {
        return fail(s, error);
    }
    s.player.task += 1;
    if s.player.task < p.duration(o.action.wire_name()) {
        return;
    }
    s.player.task = 0;
    let r = execute(s, p, &o);
    if r.gain.is_some() || r.loss.is_some() {
        let ev = json!({
            "type": "gain", "x": s.player.x, "y": s.player.y,
            "gain": items_json(p, r.gain.as_deref().unwrap_or(&[])),
            "loss": items_json(p, r.loss.as_deref().unwrap_or(&[])),
        });
        push_event(s, ev);
    }
    let done = o.done.unwrap_or(0) + 1;
    s.player.queue[0].done = Some(done);
    if done >= o.times.filter(|v| *v != 0).unwrap_or(1) {
        s.player.queue.remove(0);
    }
}

fn clear_queue(s: &mut State) {
    s.player.queue.clear();
    s.player.path.clear();
    s.player.step = 0;
    s.player.task = 0;
}

fn fail(s: &mut State, message: Value) {
    s.player.queue.remove(0);
    s.player.path.clear();
    s.player.step = 0;
    s.player.task = 0;
    push_event(s, json!({ "type": "warning", "msg": message }));
}

/// A command's reply: did it work? and the message for the player, if any.
pub fn reply(ok: bool, message: Option<Value>) -> Value {
    match message {
        Some(m) => json!({ "ok": ok, "msg": m }),
        None => json!({ "ok": ok }),
    }
}

fn integer(v: Option<&Value>) -> Option<i64> {
    num(v).filter(|f| f.fract() == 0.0 && f.abs() < 9e15).map(|f| f as i64)
}

/// Command "ordem": gives the player an order. With `adicionar`, it goes to
/// the end of the queue; without, it replaces the queue.
pub fn order_cmd(s: &mut State, p: &Package, cmd: &Value) -> Value {
    let action = cmd.get("acao").and_then(Value::as_str).and_then(Action::from_wire);
    let (x, y) = (integer(cmd.get("x")), integer(cmd.get("y")));
    let (Some(action), Some(x), Some(y)) = (action, x, y) else { return reply(false, None) };
    if !s.inside(x, y) {
        return reply(false, None);
    }
    let times = num(cmd.get("vezes")).filter(|v| *v > 1.0).map(|v| (to_int32(v) as i64).min(99));
    let o = Order {
        x,
        y,
        action,
        item: nonempty_str(cmd.get("item")).map(str::to_string),
        element: nonempty_str(cmd.get("elemento")).map(str::to_string),
        times,
        done: None,
    };
    if let Some(error) = validate(s, p, &o) {
        return reply(false, Some(error));
    }
    if action == Action::Go && passage_cost(s, p, x, y) < 0 {
        return reply(false, Some(key("error.cannotStep")));
    }
    if truthy(cmd.get("adicionar")) {
        if s.player.queue.len() >= 20 {
            return reply(false, Some(key("error.queueFull")));
        }
        s.player.queue.push(o);
    } else {
        clear_queue(s);
        s.player.queue.push(o);
    }
    reply(true, None)
}

/// Command "cancelar": empties the task queue.
pub fn cancel(s: &mut State) -> Value {
    if s.player.queue.is_empty() {
        return reply(false, None);
    }
    clear_queue(s);
    reply(true, Some(key("info.tasksCancelled")))
}
