//! Compiling the world package: checks the references and turns the data
//! (which arrives as JSON, in the order it was written) into dense tables
//! indexed by number. The core knows no item, machine or cover by name:
//! everything comes from here.
//!
//! The package keys ("substratos", "coberturas"...) are the package format,
//! shared with the game data and with old saves, so they stay as they are.

use std::collections::BTreeMap;

use serde_json::{Map, Value};

use crate::js::{nonempty_str, num, round, truthy};
use crate::TPS;

/// Index into a package table (item, cover, element...).
pub type Id = u16;

/// List of (item, amount) in package order.
pub type ItemList = Vec<(Id, i64)>;

/// How much it costs to enter a cell, for the player's passage class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Passage {
    Free,
    Blocked,
    Cost(i64),
}

#[derive(Debug)]
pub struct Substrate {
    pub id: String,
    pub name: String,
    pub buildable: bool,
    pub plantable: bool,
    /// `plantar === false`: together with `!buildable`, marks water for the map generator.
    pub plantable_is_false: bool,
    pub marks: Vec<Id>,
    pub passage: Passage,
}

#[derive(Debug)]
pub struct Deposit {
    pub id: String,
    pub name: String,
    pub item: Id,
}

#[derive(Debug)]
pub struct Growth {
    pub into: Option<Id>,
    pub seconds: f64,
    pub neighbor_bonus: Option<Id>,
}

#[derive(Debug)]
pub struct Harvest {
    pub verb: Option<String>,
    pub gives: ItemList,
    pub becomes: Option<Id>,
}

#[derive(Debug)]
pub struct Cover {
    pub id: String,
    pub name: String,
    pub replaceable: bool,
    /// how much it absorbs from each field per second (in field order)
    pub absorbs: Vec<i64>,
    /// `None`: never regresses; `Some(None)`: regresses to nothing
    pub regresses: Option<Option<Id>>,
    pub growth: Option<Growth>,
    pub harvest: Option<Harvest>,
    pub marks: Vec<Id>,
    pub passage: Passage,
}

#[derive(Debug)]
pub struct Item {
    pub id: String,
    pub name: String,
    pub fuel: i64,
    pub plants: Option<Id>,
}

#[derive(Debug)]
pub struct Recipe {
    pub id: String,
    pub name: String,
    pub inputs: ItemList,
    pub outputs: ItemList,
    pub ticks: i64,
    pub by_hand: bool,
    pub needs_neighbor: Option<Id>,
    pub needs_warning: Option<String>,
}

#[derive(Debug, Default)]
pub struct Inventory {
    pub input: Option<i64>,
    pub output: Option<i64>,
    pub fuel: Option<i64>,
    pub storage: Option<i64>,
}

#[derive(Debug)]
pub struct Element {
    pub id: String,
    pub name: String,
    pub cost: ItemList,
    pub needs_deposit: bool,
    /// "energy" component: how much it burns per tick
    pub consumption: Option<i64>,
    /// "extract" component: ticks per item
    pub extract_ticks: Option<i64>,
    /// "process" component: the recipes the machine knows
    pub process: Option<Vec<Id>>,
    pub delivers: bool,
    pub inv: Inventory,
    /// "emit" component: how much it emits into each field (0 = nothing)
    pub emits: Vec<f64>,
}

#[derive(Debug)]
pub struct Damage {
    pub threshold: f64,
    pub chance: f64,
    pub warning: String,
}

#[derive(Debug)]
pub struct Field {
    pub id: String,
    pub spread_threshold: f64,
    pub spread_divisor: f64,
    /// above this, nothing grows in the chunk (0 = never inhibits)
    pub inhibits: f64,
    pub damage: Option<Damage>,
}

#[derive(Debug)]
pub struct Technology {
    pub id: String,
    pub name: String,
    pub cost: ItemList,
    pub unlocks: Vec<Id>,
    pub requires: Vec<Id>,
    pub opens_era: i64,
}

#[derive(Debug)]
pub struct PlayerDef {
    pub ticks_per_cell: i64,
    pub reach: i64,
    pub mine_times: i64,
    /// ticks of each action, by the action's name
    pub durations: Vec<(String, i64)>,
}

#[derive(Debug)]
pub struct River {
    pub substrate: Id,
    pub ford: Option<Id>,
    pub fords: i64,
    pub position: f64,
    pub width: i64,
}

#[derive(Debug)]
pub enum Patch {
    Cover { kinds: Vec<(Id, f64)>, per_area: f64, scale_by: Option<String>, radius: (i64, i64), fill: f64 },
    Deposit { kind: Id, at: (f64, f64), radius: i64, amount: (i64, i64) },
}

#[derive(Debug)]
pub struct Generation {
    pub substrate: Id,
    pub cover: Option<Id>,
    pub rivers: Vec<River>,
    pub patches: Vec<Patch>,
}

/// The compiled package.
#[derive(Debug)]
pub struct Package {
    pub id: String,
    pub version: String,
    pub chunk: i64,
    pub defaults: Map<String, Value>,
    pub generation: Generation,
    pub player: PlayerDef,
    pub substrates: Vec<Substrate>,
    pub deposits: Vec<Deposit>,
    pub covers: Vec<Cover>,
    pub items: Vec<Item>,
    pub recipes: Vec<Recipe>,
    pub elements: Vec<Element>,
    pub fields: Vec<Field>,
    pub eras: Vec<String>,
    pub technologies: Vec<Technology>,
    pub start_inventory: Vec<(Id, f64)>,
    pub start_unlocked: Vec<Id>,
    pub mission_name: String,
    pub opening: Option<String>,
    pub goals: Value,
    /// marks ("agua", "floresta"...) mentioned in the package
    pub marks: Vec<String>,
    index: BTreeMap<(u8, String), Id>,
}

// tables, for the name index
const T_SUB: u8 = 0;
const T_DEP: u8 = 1;
const T_COV: u8 = 2;
const T_ITEM: u8 = 3;
const T_REC: u8 = 4;
const T_EL: u8 = 5;
const T_FIELD: u8 = 6;
const T_TECH: u8 = 7;
const T_MARK: u8 = 8;

impl Package {
    fn find(&self, t: u8, name: &str) -> Option<Id> {
        self.index.get(&(t, name.to_string())).copied()
    }
    pub fn substrate(&self, name: &str) -> Option<Id> {
        self.find(T_SUB, name)
    }
    pub fn deposit(&self, name: &str) -> Option<Id> {
        self.find(T_DEP, name)
    }
    pub fn cover(&self, name: &str) -> Option<Id> {
        self.find(T_COV, name)
    }
    pub fn item(&self, name: &str) -> Option<Id> {
        self.find(T_ITEM, name)
    }
    pub fn recipe(&self, name: &str) -> Option<Id> {
        self.find(T_REC, name)
    }
    pub fn element(&self, name: &str) -> Option<Id> {
        self.find(T_EL, name)
    }
    pub fn field(&self, name: &str) -> Option<Id> {
        self.find(T_FIELD, name)
    }
    pub fn technology(&self, name: &str) -> Option<Id> {
        self.find(T_TECH, name)
    }
    pub fn mark(&self, name: &str) -> Option<Id> {
        self.find(T_MARK, name)
    }
    /// The item's name for messages; if it doesn't exist, the id itself.
    pub fn item_name(&self, id: &str) -> String {
        match self.item(id) {
            Some(i) => self.items[i as usize].name.clone(),
            None => id.to_string(),
        }
    }
    pub fn duration(&self, action: &str) -> i64 {
        self.player.durations.iter().find(|(a, _)| a == action).map(|(_, t)| *t).filter(|t| *t != 0).unwrap_or(1)
    }
}

/// A key as JavaScript would print it in `${id}`.
fn key_text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn entries(v: Option<&Value>) -> impl Iterator<Item = (&String, &Value)> {
    v.and_then(Value::as_object).into_iter().flat_map(|m| m.iter())
}

fn list(v: Option<&Value>) -> impl Iterator<Item = &Value> {
    v.and_then(Value::as_array).into_iter().flat_map(|a| a.iter())
}

fn must_exist(c: &Value, errors: &mut Vec<String>, table: &str, id: Option<&Value>, place: &str) {
    let Some(id) = id.filter(|v| !v.is_null()) else { return };
    let k = key_text(id);
    if !truthy(c.get(table).and_then(|t| t.get(&k))) {
        errors.push(format!("{place}: \"{k}\" does not exist in {table}"));
    }
}

/// Validation with the same messages as the JavaScript prototype.
fn validate(c: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    let e = &mut errors;
    let s = |k: &String| Value::String(k.clone());
    for (id, d) in entries(c.get("depositos")) {
        must_exist(c, e, "itens", d.get("item"), &format!("deposit {id}"));
    }
    for (id, cv) in entries(c.get("coberturas")) {
        let place = format!("cover {id}");
        if let Some(gr) = cv.get("cresce").filter(|v| truthy(Some(v))) {
            must_exist(c, e, "coberturas", gr.get("para"), &place);
        }
        if let Some(r) = cv.get("regride") {
            must_exist(c, e, "coberturas", Some(r), &place);
        }
        if let Some(h) = cv.get("colheita").filter(|v| truthy(Some(v))) {
            must_exist(c, e, "coberturas", h.get("vira"), &place);
            for (it, _) in entries(h.get("da")) {
                must_exist(c, e, "itens", Some(&s(it)), &format!("harvest of {id}"));
            }
        }
    }
    for (id, it) in entries(c.get("itens")) {
        if truthy(it.get("planta")) {
            must_exist(c, e, "coberturas", it.get("planta"), &format!("item {id}"));
        }
    }
    for (id, r) in entries(c.get("receitas")) {
        for (it, _) in entries(r.get("entra")).chain(entries(r.get("sai"))) {
            must_exist(c, e, "itens", Some(&s(it)), &format!("recipe {id}"));
        }
    }
    for (id, el) in entries(c.get("elementos")) {
        for (it, _) in entries(el.get("custo")) {
            must_exist(c, e, "itens", Some(&s(it)), &format!("cost of {id}"));
        }
        for r in list(el.get("processo")) {
            must_exist(c, e, "receitas", Some(r), &format!("element {id}"));
        }
        for (f, _) in entries(el.get("emite")) {
            must_exist(c, e, "campos", Some(&s(f)), &format!("element {id}"));
        }
    }
    for (id, t) in entries(c.get("tecnologias")) {
        let place = format!("technology {id}");
        for (it, _) in entries(t.get("custo")) {
            must_exist(c, e, "itens", Some(&s(it)), &place);
        }
        for x in list(t.get("libera")) {
            must_exist(c, e, "elementos", Some(x), &place);
        }
        for r in list(t.get("requer")) {
            must_exist(c, e, "tecnologias", Some(r), &place);
        }
    }
    let start = c.get("inicio");
    for x in list(start.and_then(|i| i.get("liberados"))) {
        must_exist(c, e, "elementos", Some(x), "inicio");
    }
    if !truthy(c.get("jogador")) {
        e.push("the player definition (jogador) is missing".to_string());
    }
    for (it, _) in entries(start.and_then(|i| i.get("inventario"))) {
        must_exist(c, e, "itens", Some(&s(it)), "inicio");
    }
    errors
}

fn ticks_of(seconds: Option<f64>) -> i64 {
    (round(seconds.unwrap_or(0.0) * TPS as f64) as i64).max(1)
}

/// The changes compiling makes to the data (as the prototype did): each
/// recipe gets its own id and its duration in ticks; so do extraction and
/// the player.
fn convert(c: &mut Value) {
    if let Some(rs) = c.get_mut("receitas").and_then(Value::as_object_mut) {
        for (id, r) in rs.iter_mut() {
            if let Some(r) = r.as_object_mut() {
                let ticks = ticks_of(num(r.get("segundos")).filter(|s| *s != 0.0));
                r.insert("id".into(), Value::String(id.clone()));
                r.insert("ticks".into(), ticks.into());
            }
        }
    }
    if let Some(es) = c.get_mut("elementos").and_then(Value::as_object_mut) {
        for (_, e) in es.iter_mut() {
            if let Some(x) = e.get_mut("extrai").and_then(Value::as_object_mut) {
                let ticks = ticks_of(num(x.get("segundos")));
                x.insert("ticks".into(), ticks.into());
            }
        }
    }
    if let Some(j) = c.get_mut("jogador").and_then(Value::as_object_mut) {
        let cells_per_second = num(j.get("celulasPorSegundo")).unwrap_or(f64::NAN);
        let tpc = (round(TPS as f64 / cells_per_second) as i64).max(1);
        j.insert("ticksPorCelula".into(), tpc.into());
        if let Some(d) = j.get_mut("duracoes").and_then(Value::as_object_mut) {
            for (_, s) in d.iter_mut() {
                *s = ticks_of(s.as_f64()).into();
            }
        }
    }
}

/// Helps build the dense tables, collecting the errors the prototype did not
/// catch (fractional numbers where the core needs integers, missing names).
struct Builder<'a> {
    c: &'a Value,
    index: BTreeMap<(u8, String), Id>,
    marks: Vec<String>,
    errors: Vec<String>,
}

impl Builder<'_> {
    fn add_table(&mut self, t: u8, table: &str) {
        for (k, (id, _)) in entries(self.c.get(table)).enumerate() {
            self.index.insert((t, id.clone()), k as Id);
        }
    }
    fn id(&mut self, t: u8, v: Option<&Value>, place: &str) -> Option<Id> {
        let k = key_text(v?);
        let r = self.index.get(&(t, k.clone())).copied();
        if r.is_none() {
            self.errors.push(format!("{place}: \"{k}\" does not exist"));
        }
        r
    }
    fn mark(&mut self, name: &str) -> Id {
        if let Some(i) = self.index.get(&(T_MARK, name.to_string())) {
            return *i;
        }
        self.marks.push(name.to_string());
        let i = (self.marks.len() - 1) as Id;
        self.index.insert((T_MARK, name.to_string()), i);
        i
    }
    fn marks_of(&mut self, v: Option<&Value>) -> Vec<Id> {
        list(v).filter_map(Value::as_str).map(|m| self.mark(m)).collect::<Vec<_>>()
    }
    fn int(&mut self, v: Option<&Value>, place: &str) -> Option<i64> {
        match v {
            None | Some(Value::Null) => None,
            Some(Value::Number(n)) => match n.as_i64() {
                Some(i) => Some(i),
                None => {
                    let f = n.as_f64().unwrap_or(f64::NAN);
                    if f.fract() == 0.0 && f.abs() < 9e15 {
                        Some(f as i64)
                    } else {
                        self.errors.push(format!("{place}: {n} must be an integer"));
                        None
                    }
                }
            },
            Some(other) => {
                self.errors.push(format!("{place}: {other} must be a number"));
                None
            }
        }
    }
    fn items(&mut self, v: Option<&Value>, place: &str) -> ItemList {
        let mut r = Vec::new();
        for (it, n) in entries(v) {
            let id = self.id(T_ITEM, Some(&Value::String(it.clone())), place);
            let n = self.int(Some(n), &format!("{place}, {it}")).unwrap_or(0);
            if let Some(id) = id {
                r.push((id, n));
            }
        }
        r
    }
    fn passage(&mut self, def: &Value, class: &str, place: &str) -> Passage {
        match def.get("passagem").and_then(|p| p.get(class)) {
            Some(Value::String(s)) if s == "bloqueia" => Passage::Blocked,
            Some(n @ Value::Number(_)) => Passage::Cost(self.int(Some(n), place).unwrap_or(1)),
            _ => Passage::Free,
        }
    }
    fn per_field<T: Copy>(&mut self, v: Option<&Value>, zero: T, mut f: impl FnMut(&Value) -> Option<T>) -> Vec<T> {
        let n = entries(self.c.get("campos")).count();
        let mut r = vec![zero; n];
        for (fid, x) in entries(v) {
            if let Some(k) = self.index.get(&(T_FIELD, fid.clone())) {
                if let Some(val) = f(x) {
                    r[*k as usize] = val;
                }
            }
        }
        r
    }
}

fn str_of(v: Option<&Value>) -> String {
    v.and_then(Value::as_str).unwrap_or("").to_string()
}

/// Compiles the package. Returns the dense tables and the converted data
/// (which the interface uses for names, colours and descriptions), or the
/// list of errors.
pub fn compile(package: &Value) -> Result<(Package, Value), String> {
    let package_id = package.get("id").map(key_text).unwrap_or_else(|| "undefined".into());
    let failure = |errors: Vec<String>| format!("Invalid package {package_id}:\n- {}", errors.join("\n- "));
    let errors = validate(package);
    if !errors.is_empty() {
        return Err(failure(errors));
    }
    let mut converted = package.clone();
    convert(&mut converted);
    let c = &converted;

    let mut b = Builder { c, index: BTreeMap::new(), marks: Vec::new(), errors: Vec::new() };
    b.add_table(T_SUB, "substratos");
    b.add_table(T_DEP, "depositos");
    b.add_table(T_COV, "coberturas");
    b.add_table(T_ITEM, "itens");
    b.add_table(T_REC, "receitas");
    b.add_table(T_EL, "elementos");
    b.add_table(T_FIELD, "campos");
    b.add_table(T_TECH, "tecnologias");

    let pv = c.get("jogador").cloned().unwrap_or(Value::Null);
    let class = str_of(pv.get("passagem"));
    let player = PlayerDef {
        ticks_per_cell: b.int(pv.get("ticksPorCelula"), "jogador").unwrap_or(1),
        reach: b.int(pv.get("alcance"), "jogador.alcance").unwrap_or(0),
        mine_times: b.int(pv.get("minerarVezes"), "jogador.minerarVezes").filter(|v| *v != 0).unwrap_or(1),
        durations: entries(pv.get("duracoes")).map(|(a, t)| (a.clone(), t.as_i64().unwrap_or(1))).collect(),
    };

    let mut substrates = Vec::new();
    for (id, s) in entries(c.get("substratos")) {
        let marks = b.marks_of(s.get("marcas"));
        let passage = b.passage(s, &class, &format!("substrate {id}"));
        substrates.push(Substrate {
            id: id.clone(),
            name: str_of(s.get("nome")),
            buildable: truthy(s.get("construir")),
            plantable: truthy(s.get("plantar")),
            plantable_is_false: s.get("plantar") == Some(&Value::Bool(false)),
            marks,
            passage,
        });
    }
    let mut deposits = Vec::new();
    for (id, d) in entries(c.get("depositos")) {
        let item = b.id(T_ITEM, d.get("item"), &format!("deposit {id}")).unwrap_or(0);
        deposits.push(Deposit { id: id.clone(), name: str_of(d.get("nome")), item });
    }
    let mut covers = Vec::new();
    for (id, cv) in entries(c.get("coberturas")) {
        let place = format!("cover {id}");
        let absorbs = {
            let mut errors = Vec::new();
            let r = b.per_field(cv.get("absorve"), 0i64, |x| match x.as_i64() {
                Some(i) => Some(i),
                None => {
                    errors.push(format!("{place}: absorption {x} must be an integer"));
                    None
                }
            });
            b.errors.extend(errors);
            r
        };
        let regresses = cv.get("regride").map(|r| if r.is_null() { None } else { b.id(T_COV, Some(r), &place) });
        let growth = cv.get("cresce").filter(|v| truthy(Some(v))).map(|gr| Growth {
            into: gr.get("para").filter(|v| !v.is_null()).and_then(|p| b.id(T_COV, Some(p), &place)),
            seconds: num(gr.get("segundos")).unwrap_or(f64::NAN),
            neighbor_bonus: nonempty_str(gr.get("bonusVizinho")).map(|s| s.to_string()).map(|s| b.mark(&s)),
        });
        let harvest = cv.get("colheita").filter(|v| truthy(Some(v))).map(|h| Harvest {
            verb: nonempty_str(h.get("verbo")).map(str::to_string),
            gives: b.items(h.get("da"), &format!("harvest of {id}")),
            becomes: h.get("vira").filter(|v| !v.is_null()).and_then(|p| b.id(T_COV, Some(p), &place)),
        });
        let marks = b.marks_of(cv.get("marcas"));
        let passage = b.passage(cv, &class, &place);
        covers.push(Cover {
            id: id.clone(),
            name: str_of(cv.get("nome")),
            replaceable: truthy(cv.get("substituivel")),
            absorbs,
            regresses,
            growth,
            harvest,
            marks,
            passage,
        });
    }
    let mut items = Vec::new();
    for (id, it) in entries(c.get("itens")) {
        let fuel = b.int(it.get("combustivel"), &format!("item {id}")).unwrap_or(0);
        let plants = if truthy(it.get("planta")) { b.id(T_COV, it.get("planta"), &format!("item {id}")) } else { None };
        items.push(Item { id: id.clone(), name: str_of(it.get("nome")), fuel, plants });
    }
    let mut recipes = Vec::new();
    for (id, r) in entries(c.get("receitas")) {
        let place = format!("recipe {id}");
        let needs = r.get("exige");
        recipes.push(Recipe {
            id: id.clone(),
            name: str_of(r.get("nome")),
            inputs: b.items(r.get("entra"), &place),
            outputs: b.items(r.get("sai"), &place),
            ticks: r.get("ticks").and_then(Value::as_i64).unwrap_or(1),
            by_hand: truthy(r.get("mao")),
            needs_neighbor: nonempty_str(needs.and_then(|e| e.get("vizinho"))).map(str::to_string).map(|s| b.mark(&s)),
            needs_warning: nonempty_str(needs.and_then(|e| e.get("aviso"))).map(str::to_string),
        });
    }
    let mut elements = Vec::new();
    for (id, e) in entries(c.get("elementos")) {
        let place = format!("element {id}");
        let inv = e.get("inventario");
        let slot = |b: &mut Builder, k: &str| b.int(inv.and_then(|i| i.get(k)), &format!("{place}, inventory {k}"));
        let inv = Inventory {
            input: slot(&mut b, "entrada"),
            output: slot(&mut b, "saida"),
            fuel: slot(&mut b, "combustivel"),
            storage: slot(&mut b, "guarda"),
        };
        let consumption = e
            .get("energia")
            .filter(|v| truthy(Some(v)))
            .map(|en| b.int(en.get("consumo"), &format!("{place}, consumo")).unwrap_or(0));
        let process = if truthy(e.get("processo")) {
            Some(list(e.get("processo")).filter_map(|r| b.id(T_REC, Some(r), &place)).collect())
        } else {
            None
        };
        elements.push(Element {
            id: id.clone(),
            name: str_of(e.get("nome")),
            cost: b.items(e.get("custo"), &format!("cost of {id}")),
            needs_deposit: e.get("exige").and_then(Value::as_str) == Some("deposito"),
            consumption,
            extract_ticks: e
                .get("extrai")
                .filter(|v| truthy(Some(v)))
                .map(|x| x.get("ticks").and_then(Value::as_i64).unwrap_or(1)),
            process,
            delivers: truthy(e.get("entrega")),
            inv,
            emits: b.per_field(e.get("emite"), 0.0, Value::as_f64),
        });
    }
    let mut fields = Vec::new();
    for (id, f) in entries(c.get("campos")) {
        let spread = f.get("difusao");
        let divisor = num(spread.and_then(|d| d.get("divisor"))).unwrap_or(0.0);
        if divisor <= 0.0 {
            b.errors.push(format!("field {id}: spreading needs a divisor greater than zero"));
        }
        fields.push(Field {
            id: id.clone(),
            spread_threshold: num(spread.and_then(|d| d.get("limiar"))).unwrap_or(0.0),
            spread_divisor: divisor,
            inhibits: num(f.get("inibeCrescimento")).unwrap_or(0.0),
            damage: f.get("dano").filter(|v| truthy(Some(v))).map(|d| Damage {
                threshold: num(d.get("limiar")).unwrap_or(f64::NAN),
                chance: num(d.get("chance")).unwrap_or(f64::NAN),
                warning: str_of(d.get("aviso")),
            }),
        });
    }
    let eras = list(c.get("eras")).map(|e| str_of(e.get("nome"))).collect();
    let mut technologies = Vec::new();
    for (id, t) in entries(c.get("tecnologias")) {
        let place = format!("technology {id}");
        technologies.push(Technology {
            id: id.clone(),
            name: str_of(t.get("nome")),
            cost: b.items(t.get("custo"), &place),
            unlocks: list(t.get("libera")).filter_map(|x| b.id(T_EL, Some(x), &place)).collect(),
            requires: list(t.get("requer")).filter_map(|x| b.id(T_TECH, Some(x), &place)).collect(),
            opens_era: b.int(t.get("abreEra"), &place).unwrap_or(0),
        });
    }

    let start = c.get("inicio");
    let mut start_inventory = Vec::new();
    for (it, n) in entries(start.and_then(|i| i.get("inventario"))) {
        if let Some(id) = b.id(T_ITEM, Some(&Value::String(it.clone())), "inicio") {
            start_inventory.push((id, n.as_f64().unwrap_or(f64::NAN)));
        }
    }
    let start_unlocked =
        list(start.and_then(|i| i.get("liberados"))).filter_map(|x| b.id(T_EL, Some(x), "inicio")).collect();

    let map = c.get("mapa");
    let chunk = b.int(map.and_then(|x| x.get("chunk")), "mapa.chunk").unwrap_or(0);
    if chunk <= 0 {
        b.errors.push("mapa.chunk must be an integer greater than zero".into());
    }
    let g = map.and_then(|x| x.get("geracao"));
    let gen = |k: &str| g.and_then(|g| g.get(k));
    let substrate = b.id(T_SUB, gen("substrato"), "mapa.geracao").unwrap_or(0);
    let cover = gen("cobertura").filter(|v| !v.is_null()).and_then(|v| b.id(T_COV, Some(v), "mapa.geracao"));
    let mut rivers = Vec::new();
    for r in list(gen("rios")) {
        rivers.push(River {
            substrate: b.id(T_SUB, r.get("substrato"), "rio").unwrap_or(0),
            ford: if truthy(r.get("vau")) { b.id(T_SUB, r.get("vau"), "rio") } else { None },
            fords: b.int(r.get("vaus"), "rio.vaus").unwrap_or(0),
            position: num(r.get("posicao")).unwrap_or(f64::NAN),
            width: b.int(r.get("largura"), "rio.largura").unwrap_or(0),
        });
    }
    let mut patches = Vec::new();
    for x in list(gen("manchas")) {
        let int_pair = |b: &mut Builder, v: Option<&Value>, place: &str| {
            let lo = v.and_then(|v| v.get(0));
            let hi = v.and_then(|v| v.get(1));
            (b.int(lo, place).unwrap_or(0), b.int(hi, place).unwrap_or(0))
        };
        match x.get("camada").and_then(Value::as_str) {
            Some("cobertura") => {
                let mut kinds = Vec::new();
                for (t, p) in entries(x.get("tipos")) {
                    if let Some(id) = b.id(T_COV, Some(&Value::String(t.clone())), "mancha") {
                        kinds.push((id, p.as_f64().unwrap_or(f64::NAN)));
                    }
                }
                let radius = int_pair(&mut b, x.get("raio"), "mancha.raio");
                patches.push(Patch::Cover {
                    kinds,
                    per_area: num(x.get("porArea")).unwrap_or(f64::NAN),
                    scale_by: x.get("escalaPor").map(key_text),
                    radius,
                    fill: num(x.get("preenche")).unwrap_or(f64::NAN),
                });
            }
            Some("deposito") => {
                let kind = b.id(T_DEP, x.get("tipo"), "mancha").unwrap_or(0);
                let at = x.get("em");
                let radius = b.int(x.get("raio"), "mancha.raio").unwrap_or(0);
                let amount = int_pair(&mut b, x.get("qtd"), "mancha.qtd");
                patches.push(Patch::Deposit {
                    kind,
                    at: (
                        num(at.and_then(|e| e.get(0))).unwrap_or(f64::NAN),
                        num(at.and_then(|e| e.get(1))).unwrap_or(f64::NAN),
                    ),
                    radius,
                    amount,
                });
            }
            _ => {}
        }
    }

    let mission = c.get("missao");
    let p = Package {
        id: package_id.clone(),
        version: c.get("versao").map(key_text).unwrap_or_default(),
        chunk,
        defaults: c.get("padrao").and_then(Value::as_object).cloned().unwrap_or_default(),
        generation: Generation { substrate, cover, rivers, patches },
        player,
        substrates,
        deposits,
        covers,
        items,
        recipes,
        elements,
        fields,
        eras,
        technologies,
        start_inventory,
        start_unlocked,
        mission_name: str_of(mission.and_then(|x| x.get("nome"))),
        opening: nonempty_str(mission.and_then(|x| x.get("abertura"))).map(str::to_string),
        goals: mission.and_then(|x| x.get("metas")).cloned().unwrap_or(Value::Array(vec![])),
        marks: b.marks,
        index: b.index,
    };
    if !b.errors.is_empty() {
        return Err(failure(b.errors));
    }
    Ok((p, converted))
}
