//! Messages for the player, in a form that any language can render.
//!
//! The core never writes interface text. It says *what* happened with a key
//! and arguments, and the interface turns that into text in the player's
//! language (see `web/platform/i18n/`). A message is JSON:
//!
//! ```json
//! { "key": "log.built", "args": { "element": { "ref": ["elementos", "fornalha", "nome"] }, "x": 3, "y": 4 } }
//! ```
//!
//! Argument values are numbers, plain strings, `{"ref": path}` (a text of
//! the package, looked up in the player's language, with an optional
//! `"text"` to show if the path doesn't exist), `{"items": {id: n}}` (a list
//! of amounts), `{"list": [...]}` (several values in a row) or another
//! message. Where a message is expected, a bare `{"ref": path}` also works.
//!
//! The only exception is the world log and the reason a machine stopped:
//! they are part of the world state, enter its hash and must stay as the
//! JavaScript prototype wrote them (in Portuguese), so old saves keep their
//! hash. Next to that frozen text the core keeps the message, which is what
//! the interface shows. Entries from old saves, which only have the text,
//! get their message back from it (`from_legacy_log`).

use serde_json::{json, Map, Value};

use crate::package::{Id, Package};

pub fn msg(key: &str, args: Value) -> Value {
    if args.as_object().is_some_and(Map::is_empty) {
        json!({ "key": key })
    } else {
        json!({ "key": key, "args": args })
    }
}

/// A message without arguments.
pub fn key(key: &str) -> Value {
    json!({ "key": key })
}

/// A text of the package, by its path in the package (`["itens", "madeira", "nome"]`).
pub fn text_ref(path: &[&str]) -> Value {
    json!({ "ref": path })
}

/// The name of something in a package table ("itens", "elementos"...).
pub fn name_ref(table: &str, id: &str) -> Value {
    text_ref(&[table, id, "nome"])
}

/// A text of the package, with the text to show if the path doesn't exist.
pub fn text_ref_or(path: &[&str], fallback: &str) -> Value {
    json!({ "ref": path, "text": fallback })
}

pub fn item_ref(p: &Package, it: Id) -> Value {
    name_ref("itens", &p.items[it as usize].id)
}

pub fn element_ref(p: &Package, el: Id) -> Value {
    name_ref("elementos", &p.elements[el as usize].id)
}

/// A list of amounts of items: "4 madeira, 1 muda".
pub fn items(p: &Package, list: &[(Id, i64)]) -> Value {
    let mut m = Map::new();
    for (it, n) in list {
        m.insert(p.items[*it as usize].id.clone(), (*n).into());
    }
    json!({ "items": m })
}

/// Several values in a row: "Viveiro, Eletricidade".
pub fn list(values: Vec<Value>) -> Value {
    json!({ "list": values })
}

/// The message for a log entry written before the messages existed (old
/// saves keep only the Portuguese text). Recognizes the texts the core
/// writes; anything else stays as plain text.
pub fn from_legacy_log(text: &str, p: &Package, goals: &[Value]) -> Value {
    let between = |prefix: &str, suffix: &str| text.strip_prefix(prefix).and_then(|r| r.strip_suffix(suffix));
    // "<what> em (x,y)."
    let at = |rest: &str| -> Option<(String, i64, i64)> {
        let (what, xy) = rest.rsplit_once(" em (")?;
        let (x, y) = xy.strip_suffix(").")?.split_once(',')?;
        Some((what.to_string(), x.parse().ok()?, y.parse().ok()?))
    };
    if p.opening.as_deref() == Some(text) {
        return text_ref(&["missao", "abertura"]);
    }
    if text == "O mundo começou." {
        return key("log.worldStarted");
    }
    for f in &p.fields {
        if f.damage.as_ref().is_some_and(|d| d.warning == text) {
            return text_ref(&["campos", &f.id, "dano", "aviso"]);
        }
    }
    let depleted = text.strip_prefix("Um depósito de ").and_then(at);
    if let Some((name, x, y)) = depleted.and_then(|(w, x, y)| Some((w.strip_suffix(" se esgotou")?.to_string(), x, y)))
    {
        if let Some(d) = p.deposits.iter().find(|d| d.name.to_lowercase() == name) {
            return msg("log.depositDepleted", json!({ "deposit": name_ref("depositos", &d.id), "x": x, "y": y }));
        }
    }
    for (prefix, k) in [("Construiu ", "log.built"), ("Desmontou ", "log.dismantled")] {
        if let Some((name, x, y)) = text.strip_prefix(prefix).and_then(at) {
            if let Some(el) = p.elements.iter().position(|e| crate::js::lower_first(&e.name) == name) {
                return msg(k, json!({ "element": element_ref(p, el as Id), "x": x, "y": y }));
            }
        }
    }
    if let Some(name) = between("Pesquisa concluída: ", ".") {
        if let Some(t) = p.technologies.iter().find(|t| t.name == name) {
            return msg("log.researchDone", json!({ "tech": name_ref("tecnologias", &t.id) }));
        }
    }
    if let Some(name) = between("Estamos entrando na era ", "!") {
        if let Some(k) = p.eras.iter().position(|e| e == name) {
            return msg("log.newEra", json!({ "era": text_ref_or(&["eras", &k.to_string(), "nome"], name) }));
        }
    }
    if between("Missão cumprida: ", "!") == Some(p.mission_name.as_str()) {
        return msg("log.missionDone", json!({ "mission": text_ref(&["missao", "nome"]) }));
    }
    if let Some((name, pct)) = between("Missão perdida: ", "%.").and_then(|r| r.split_once(" caiu abaixo de ")) {
        let goal =
            goals.iter().position(|g| g.get("nome").and_then(Value::as_str).is_some_and(|n| n.to_lowercase() == name));
        if let (Some(k), Ok(pct)) = (goal, pct.parse::<f64>()) {
            let original = goals[k].get("nome").and_then(Value::as_str).unwrap_or(name);
            let goal = text_ref_or(&["missao", "metas", &k.to_string(), "nome"], original);
            return msg("log.missionFailed", json!({ "goal": goal, "pct": pct }));
        }
    }
    Value::Null
}
