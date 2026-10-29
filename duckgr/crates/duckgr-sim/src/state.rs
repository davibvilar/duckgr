//! The world state, in dense tables, and its JSON form.
//!
//! The JSON is the same one the JavaScript prototype produced, with the same
//! keys in the same order. That is why it serves at once as the save
//! checkpoint, as the view for the interface and as the base of the hash, and
//! why old saves keep opening (and matching their hash) in the new core.
//! Those keys are a persisted format: they stay as they are.

use std::collections::VecDeque;

use serde_json::{json, Map, Value};

use crate::js::fnv_utf16;
use crate::message::from_legacy_log;
use crate::package::{Id, Package};

/// Amounts per item, in the order they came in (like a JavaScript object).
/// The order breaks ties: which item leaves the machine first, which fuel
/// burns first.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Bag(pub Vec<(Id, i64)>);

impl Bag {
    pub fn get(&self, it: Id) -> i64 {
        self.0.iter().find(|(k, _)| *k == it).map_or(0, |(_, n)| *n)
    }
    /// Adds `n` to the item; if it ends at zero or less, the item leaves the bag.
    pub fn add(&mut self, it: Id, n: i64) {
        match self.0.iter().position(|(k, _)| *k == it) {
            Some(p) => {
                self.0[p].1 += n;
                if self.0[p].1 <= 0 {
                    self.0.remove(p);
                }
            }
            None if n > 0 => self.0.push((it, n)),
            None => {}
        }
    }
    pub fn total(&self) -> i64 {
        self.0.iter().map(|(_, n)| n).sum()
    }
    pub fn first(&self) -> Option<Id> {
        self.0.first().map(|(k, _)| *k)
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// A built element (machine, chest...).
#[derive(Clone, Debug)]
pub struct Entity {
    pub id: u32,
    pub proto: Id,
    pub x: i64,
    pub y: i64,
    pub input: Bag,
    pub output: Bag,
    pub fuel: Bag,
    pub burn: i64,
    pub progress: i64,
    pub recipe: Option<Id>,
    pub active: i64,
    /// why it stopped, as the prototype wrote it (enters the hash)
    pub stopped: String,
    /// the same reason as a message for the interface (outside the hash)
    pub stopped_msg: Value,
}

/// The actions the player knows. The core doesn't know what a tree or an
/// ore is: actions are generic and the names come from the package.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Go,
    Harvest,
    Mine,
    Plant,
    Build,
    Dismantle,
    Collect,
    Refuel,
    Supply,
}

impl Action {
    pub const ALL: [Action; 9] = [
        Action::Go,
        Action::Harvest,
        Action::Mine,
        Action::Plant,
        Action::Build,
        Action::Dismantle,
        Action::Collect,
        Action::Refuel,
        Action::Supply,
    ];
    /// The action's name in commands and saves (a persisted format).
    pub fn wire_name(self) -> &'static str {
        match self {
            Action::Go => "ir",
            Action::Harvest => "colher",
            Action::Mine => "minerar",
            Action::Plant => "plantar",
            Action::Build => "construir",
            Action::Dismantle => "desmontar",
            Action::Collect => "recolher",
            Action::Refuel => "abastecer",
            Action::Supply => "fornecer",
        }
    }
    pub fn from_wire(name: &str) -> Option<Action> {
        Action::ALL.into_iter().find(|a| a.wire_name() == name)
    }
}

/// An order in the player's queue: "go near this cell and do this".
/// Item and element stay as text, the way they came in the command.
#[derive(Clone, Debug)]
pub struct Order {
    pub x: i64,
    pub y: i64,
    pub action: Action,
    pub item: Option<String>,
    pub element: Option<String>,
    pub times: Option<i64>,
    pub done: Option<i64>,
}

#[derive(Clone, Debug, Default)]
pub struct Player {
    pub x: i64,
    pub y: i64,
    pub queue: Vec<Order>,
    pub path: Vec<i64>,
    pub step: i64,
    pub task: i64,
}

#[derive(Clone, Debug)]
pub struct LogEntry {
    pub t: i64,
    /// the text as the prototype wrote it (enters the hash)
    pub text: String,
    pub cls: String,
    /// the same entry as a message for the interface (outside the hash)
    pub msg: Value,
}

#[derive(Clone, Debug)]
pub struct Mission {
    pub status: String,
    /// the goals as they came in the configuration (free text from the package)
    pub goals: Vec<Value>,
    /// each mark's count at the start (for cover goals)
    pub refs: Vec<(String, i64)>,
}

/// The whole world at one tick.
#[derive(Clone, Debug)]
pub struct State {
    pub tick: i64,
    pub rng: i64,
    pub w: i64,
    pub h: i64,
    pub cw: i64,
    pub ch: i64,
    pub sub: Vec<Id>,
    pub dep: Vec<Option<Id>>,
    pub dep_amount: Vec<i64>,
    pub cov: Vec<Option<Id>>,
    pub growth: Vec<i64>,
    pub occ: Vec<u32>,
    pub entities: Vec<Entity>,
    pub next_id: u32,
    pub inv: Bag,
    pub produced: Bag,
    pub fields: Vec<Vec<i64>>,
    pub techs: Vec<Id>,
    pub era: i64,
    pub unlocked: Vec<Id>,
    pub log: VecDeque<LogEntry>,
    pub last_damage: i64,
    pub mission: Mission,
    pub player: Player,
    /// transient: notices for the interface, outside the hash and the save
    pub events: Vec<Value>,
}

pub const LOG_MAX: usize = 80;

impl State {
    pub fn idx(&self, x: i64, y: i64) -> usize {
        (x + y * self.w) as usize
    }
    pub fn inside(&self, x: i64, y: i64) -> bool {
        x >= 0 && y >= 0 && x < self.w && y < self.h
    }
    pub fn entity_at(&self, x: i64, y: i64) -> Option<usize> {
        if !self.inside(x, y) {
            return None;
        }
        let id = self.occ[self.idx(x, y)];
        if id == 0 {
            return None;
        }
        // ids grow in build order, and the list keeps that order
        self.entities.binary_search_by_key(&id, |e| e.id).ok()
    }
    /// Writes to the world log: the frozen text (for the hash) and the message (for the interface).
    pub fn record(&mut self, text: impl Into<String>, cls: &str, msg: Value) {
        self.log.push_front(LogEntry { t: self.tick, text: text.into(), cls: cls.to_string(), msg });
        if self.log.len() > LOG_MAX {
            self.log.pop_back();
        }
    }

    /// The state in JSON, as the JavaScript prototype wrote it (no events,
    /// no messages). This is what the hash is computed from.
    pub fn to_json(&self, p: &Package) -> Value {
        self.json(p, false)
    }

    /// The state in JSON for the interface and the save checkpoint: the
    /// same as `to_json`, plus the messages next to the frozen texts.
    pub fn view_json(&self, p: &Package) -> Value {
        self.json(p, true)
    }

    fn json(&self, p: &Package, with_msgs: bool) -> Value {
        fn names<'a>(v: &[Id], f: &dyn Fn(Id) -> &'a str) -> Value {
            Value::Array(v.iter().map(|i| Value::from(f(*i))).collect())
        }
        fn optional<'a>(v: &[Option<Id>], f: &dyn Fn(Id) -> &'a str) -> Value {
            Value::Array(v.iter().map(|i| i.map_or(Value::Null, |i| Value::from(f(i)))).collect())
        }
        let bag = |b: &Bag| {
            Value::Object(b.0.iter().map(|(k, n)| (p.items[*k as usize].id.clone(), Value::from(*n))).collect())
        };
        let el = |i: Id| p.elements[i as usize].id.as_str();
        let mut m = Map::new();
        m.insert("tick".into(), self.tick.into());
        m.insert("rng".into(), self.rng.into());
        m.insert("L".into(), self.w.into());
        m.insert("A".into(), self.h.into());
        m.insert("CL".into(), self.cw.into());
        m.insert("CA".into(), self.ch.into());
        m.insert("sub".into(), names(&self.sub, &|i| p.substrates[i as usize].id.as_str()));
        m.insert("dep".into(), optional(&self.dep, &|i| p.deposits[i as usize].id.as_str()));
        m.insert("depQtd".into(), self.dep_amount.clone().into());
        m.insert("cov".into(), optional(&self.cov, &|i| p.covers[i as usize].id.as_str()));
        m.insert("cres".into(), self.growth.clone().into());
        m.insert("occ".into(), self.occ.clone().into());
        let entities = self
            .entities
            .iter()
            .map(|e| {
                let mut v = json!({
                    "id": e.id, "p": el(e.proto), "x": e.x, "y": e.y,
                    "ent": bag(&e.input), "sai": bag(&e.output), "comb": bag(&e.fuel),
                    "queima": e.burn, "prog": e.progress,
                    "rec": e.recipe.map(|r| p.recipes[r as usize].id.as_str()),
                    "ativo": e.active, "parado": e.stopped,
                });
                if with_msgs && !e.stopped_msg.is_null() {
                    v.as_object_mut().expect("object").insert("stoppedMsg".into(), e.stopped_msg.clone());
                }
                v
            })
            .collect();
        m.insert("ents".into(), Value::Array(entities));
        m.insert("prox".into(), self.next_id.into());
        m.insert("inv".into(), bag(&self.inv));
        m.insert("prod".into(), bag(&self.produced));
        m.insert(
            "campos".into(),
            Value::Object(p.fields.iter().zip(&self.fields).map(|(f, v)| (f.id.clone(), v.clone().into())).collect()),
        );
        m.insert("techs".into(), names(&self.techs, &|i| p.technologies[i as usize].id.as_str()));
        m.insert("era".into(), self.era.into());
        m.insert("liberados".into(), names(&self.unlocked, &el));
        let log = self
            .log
            .iter()
            .map(|r| {
                let mut v = json!({ "t": r.t, "txt": r.text, "cls": r.cls });
                if with_msgs && !r.msg.is_null() {
                    v.as_object_mut().expect("object").insert("msg".into(), r.msg.clone());
                }
                v
            })
            .collect();
        m.insert("log".into(), Value::Array(log));
        m.insert("ultimoDano".into(), self.last_damage.into());
        let refs: Map<String, Value> = self.mission.refs.iter().map(|(k, n)| (k.clone(), Value::from(*n))).collect();
        m.insert("missao".into(), json!({ "estado": self.mission.status, "metas": self.mission.goals, "ref": refs }));
        let queue: Vec<Value> = self.player.queue.iter().map(|o| o.to_json()).collect();
        let pl = &self.player;
        m.insert(
            "jog".into(),
            json!({ "x": pl.x, "y": pl.y, "fila": queue, "cam": pl.path, "passo": pl.step, "tarefa": pl.task }),
        );
        Value::Object(m)
    }

    /// World hash: FNV-1a of the JSON, the same as the prototype's.
    pub fn hash(&self, p: &Package) -> String {
        fnv_utf16(&self.to_json(p).to_string())
    }

    /// Rebuilds the state from JSON (from a checkpoint, new or old).
    pub fn from_json(v: &Value, p: &Package) -> Result<State, String> {
        Reader { p }.state(v)
    }
}

impl Order {
    pub fn to_json(&self) -> Value {
        let mut m = Map::new();
        m.insert("x".into(), self.x.into());
        m.insert("y".into(), self.y.into());
        m.insert("acao".into(), self.action.wire_name().into());
        if let Some(i) = &self.item {
            m.insert("item".into(), i.clone().into());
        }
        if let Some(e) = &self.element {
            m.insert("elemento".into(), e.clone().into());
        }
        if let Some(v) = self.times {
            m.insert("vezes".into(), v.into());
        }
        if let Some(f) = self.done {
            m.insert("feitas".into(), f.into());
        }
        Value::Object(m)
    }
}

struct Reader<'a> {
    p: &'a Package,
}

fn field<'v>(v: &'v Value, k: &str) -> Result<&'v Value, String> {
    v.get(k).ok_or_else(|| format!("state is missing \"{k}\""))
}
fn int(v: &Value, k: &str) -> Result<i64, String> {
    let x = field(v, k)?;
    x.as_i64()
        .or_else(|| x.as_f64().filter(|f| f.fract() == 0.0).map(|f| f as i64))
        .ok_or_else(|| format!("\"{k}\" must be an integer"))
}
fn arr<'v>(v: &'v Value, k: &str) -> Result<&'v Vec<Value>, String> {
    field(v, k)?.as_array().ok_or_else(|| format!("\"{k}\" must be a list"))
}
fn ints(v: &Value, k: &str) -> Result<Vec<i64>, String> {
    arr(v, k)?.iter().map(|x| x.as_i64().ok_or_else(|| format!("\"{k}\" must hold only integers"))).collect()
}
fn text(v: &Value, k: &str) -> Result<String, String> {
    Ok(field(v, k)?.as_str().unwrap_or("").to_string())
}

impl Reader<'_> {
    fn id(&self, v: &Value, find: impl Fn(&str) -> Option<Id>, what: &str) -> Result<Id, String> {
        let s = v.as_str().ok_or_else(|| format!("{what} must be text"))?;
        find(s).ok_or_else(|| format!("{what} \"{s}\" does not exist in this package"))
    }
    fn optional(&self, v: &Value, find: impl Fn(&str) -> Option<Id>, what: &str) -> Result<Option<Id>, String> {
        if v.is_null() {
            Ok(None)
        } else {
            self.id(v, find, what).map(Some)
        }
    }
    fn bag(&self, v: &Value) -> Result<Bag, String> {
        let m = v.as_object().ok_or("an inventory must be an object")?;
        let mut b = Bag::default();
        for (k, n) in m {
            let it = self.p.item(k).ok_or_else(|| format!("item \"{k}\" does not exist in this package"))?;
            b.0.push((it, n.as_i64().ok_or("an amount must be an integer")?));
        }
        Ok(b)
    }
    fn state(&self, v: &Value) -> Result<State, String> {
        let p = self.p;
        let (w, h) = (int(v, "L")?, int(v, "A")?);
        let n = (w * h) as usize;
        let sub = arr(v, "sub")?
            .iter()
            .map(|x| self.id(x, |s| p.substrate(s), "substrate"))
            .collect::<Result<Vec<_>, _>>()?;
        let dep = arr(v, "dep")?
            .iter()
            .map(|x| self.optional(x, |s| p.deposit(s), "deposit"))
            .collect::<Result<Vec<_>, _>>()?;
        let cov =
            arr(v, "cov")?.iter().map(|x| self.optional(x, |s| p.cover(s), "cover")).collect::<Result<Vec<_>, _>>()?;
        let dep_amount = ints(v, "depQtd")?;
        let growth = ints(v, "cres")?;
        let occ = ints(v, "occ")?.into_iter().map(|x| x as u32).collect::<Vec<_>>();
        if [sub.len(), dep.len(), cov.len(), dep_amount.len(), growth.len(), occ.len()].iter().any(|k| *k != n) {
            return Err("the map layers do not have the size L×A".into());
        }
        let mut entities = Vec::new();
        for e in arr(v, "ents")? {
            entities.push(Entity {
                id: int(e, "id")? as u32,
                proto: self.id(field(e, "p")?, |s| p.element(s), "element")?,
                x: int(e, "x")?,
                y: int(e, "y")?,
                input: self.bag(field(e, "ent")?)?,
                output: self.bag(field(e, "sai")?)?,
                fuel: self.bag(field(e, "comb")?)?,
                burn: int(e, "queima")?,
                progress: int(e, "prog")?,
                recipe: self.optional(field(e, "rec")?, |s| p.recipe(s), "recipe")?,
                active: int(e, "ativo")?,
                stopped: text(e, "parado")?,
                stopped_msg: e.get("stoppedMsg").cloned().unwrap_or(Value::Null),
            });
        }
        if entities.windows(2).any(|w| w[0].id >= w[1].id) {
            return Err("elements must be in build order".into());
        }
        let fields_v = field(v, "campos")?;
        let mut fields = Vec::new();
        for f in &p.fields {
            fields.push(ints(fields_v, &f.id)?);
        }
        let ids = |k: &str, find: &dyn Fn(&str) -> Option<Id>, what: &str| -> Result<Vec<Id>, String> {
            arr(v, k)?.iter().map(|x| self.id(x, find, what)).collect()
        };
        let mut log = VecDeque::new();
        for r in arr(v, "log")? {
            log.push_back(LogEntry {
                t: int(r, "t")?,
                text: text(r, "txt")?,
                cls: text(r, "cls")?,
                msg: r.get("msg").cloned().unwrap_or(Value::Null),
            });
        }
        let mv = field(v, "missao")?;
        let mission = Mission {
            status: field(mv, "estado")?.as_str().unwrap_or("andamento").to_string(),
            goals: arr(mv, "metas")?.clone(),
            refs: field(mv, "ref")?
                .as_object()
                .ok_or("ref must be an object")?
                .iter()
                .map(|(k, n)| (k.clone(), n.as_i64().unwrap_or(0)))
                .collect(),
        };
        // entries from before the messages existed: recognize the text
        for entry in log.iter_mut().filter(|e| e.msg.is_null()) {
            entry.msg = from_legacy_log(&entry.text, p, &mission.goals);
        }
        let pv = field(v, "jog")?;
        let mut queue = Vec::new();
        for o in arr(pv, "fila")? {
            queue.push(Order {
                x: int(o, "x")?,
                y: int(o, "y")?,
                action: Action::from_wire(field(o, "acao")?.as_str().unwrap_or("")).ok_or("unknown action in queue")?,
                item: o.get("item").and_then(Value::as_str).map(str::to_string),
                element: o.get("elemento").and_then(Value::as_str).map(str::to_string),
                times: o.get("vezes").and_then(Value::as_i64),
                done: o.get("feitas").and_then(Value::as_i64),
            });
        }
        let player = Player {
            x: int(pv, "x")?,
            y: int(pv, "y")?,
            queue,
            path: ints(pv, "cam")?,
            step: int(pv, "passo")?,
            task: int(pv, "tarefa")?,
        };
        Ok(State {
            tick: int(v, "tick")?,
            rng: int(v, "rng")?,
            w,
            h,
            cw: int(v, "CL")?,
            ch: int(v, "CA")?,
            sub,
            dep,
            dep_amount,
            cov,
            growth,
            occ,
            entities,
            next_id: int(v, "prox")? as u32,
            inv: self.bag(field(v, "inv")?)?,
            produced: self.bag(field(v, "prod")?)?,
            fields,
            techs: ids("techs", &|s| p.technology(s), "technology")?,
            era: int(v, "era")?,
            unlocked: ids("liberados", &|s| p.element(s), "element")?,
            log,
            last_damage: int(v, "ultimoDano")?,
            mission,
            player,
            events: Vec::new(),
        })
    }
}
