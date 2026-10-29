//! The DuckGR core for the browser.
//!
//! The interface with JavaScript is minimal and has no dependencies:
//! JavaScript writes a JSON text into the module's memory, calls a function
//! and reads the answer (also JSON) from an output buffer. The only one that
//! talks to this module is `web/platform/core.js`; the rest of the web
//! interface doesn't know it exists.
//!
//! Worlds and packages live in here and are identified by a number.

use std::cell::RefCell;
use std::rc::Rc;

use duckgr_sim::state::{Action, Order};
use duckgr_sim::{player, Package, State};
use serde_json::{json, Value};

pub struct World {
    pub p: Rc<Package>,
    pub s: State,
}

thread_local! {
    static PACKAGES: RefCell<Vec<Rc<Package>>> = const { RefCell::new(Vec::new()) };
    static WORLDS: RefCell<Vec<Option<World>>> = const { RefCell::new(Vec::new()) };
    static OUTPUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

fn output(txt: &str) {
    OUTPUT.with(|s| {
        let mut s = s.borrow_mut();
        s.clear();
        s.extend_from_slice(txt.as_bytes());
    });
}

fn store(w: World) -> i32 {
    WORLDS.with(|ws| {
        let mut ws = ws.borrow_mut();
        match ws.iter().position(Option::is_none) {
            Some(k) => {
                ws[k] = Some(w);
                k as i32
            }
            None => {
                ws.push(Some(w));
                (ws.len() - 1) as i32
            }
        }
    })
}

fn with_world<T>(h: i32, f: impl FnOnce(&mut World) -> T) -> Option<T> {
    WORLDS.with(|ws| ws.borrow_mut().get_mut(h as usize).and_then(Option::as_mut).map(f))
}

fn package(h: i32) -> Option<Rc<Package>> {
    PACKAGES.with(|ps| ps.borrow().get(h as usize).cloned())
}

/// Reads the JSON text JavaScript left in memory.
///
/// # Safety
/// `ptr` and `len` must point to bytes allocated with [`alloc`].
unsafe fn read(ptr: *const u8, len: usize) -> Result<Value, String> {
    let bytes = std::slice::from_raw_parts(ptr, len);
    serde_json::from_slice(bytes).map_err(|e| format!("invalid JSON: {e}"))
}

fn handle(r: Result<i32, String>) -> i32 {
    match r {
        Ok(h) => h,
        Err(e) => {
            output(&e);
            -1
        }
    }
}

/// Reserves `len` bytes for JavaScript to write into.
#[no_mangle]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    let mut v = Vec::<u8>::with_capacity(len.max(1));
    let p = v.as_mut_ptr();
    std::mem::forget(v);
    p
}

/// Gives back the bytes reserved with [`alloc`].
///
/// # Safety
/// `ptr` and `len` must be exactly those of a call to [`alloc`].
#[no_mangle]
pub unsafe extern "C" fn dealloc(ptr: *mut u8, len: usize) {
    drop(Vec::from_raw_parts(ptr, 0, len.max(1)));
}

#[no_mangle]
pub extern "C" fn out_ptr() -> *const u8 {
    OUTPUT.with(|s| s.borrow().as_ptr())
}

#[no_mangle]
pub extern "C" fn out_len() -> usize {
    OUTPUT.with(|s| s.borrow().len())
}

/// Compiles a package (JSON). Returns the package's number and leaves the
/// converted data in the output; or -1 and the error message.
///
/// # Safety
/// See [`read`].
#[no_mangle]
pub unsafe extern "C" fn package_compile(ptr: *const u8, len: usize) -> i32 {
    handle(read(ptr, len).and_then(|v| {
        let (p, converted) = duckgr_sim::compile(&v)?;
        output(&converted.to_string());
        Ok(PACKAGES.with(|ps| {
            let mut ps = ps.borrow_mut();
            ps.push(Rc::new(p));
            (ps.len() - 1) as i32
        }))
    }))
}

/// Creates a new world from the configuration (JSON).
///
/// # Safety
/// See [`read`].
#[no_mangle]
pub unsafe extern "C" fn world_new(pkg: i32, ptr: *const u8, len: usize) -> i32 {
    handle(read(ptr, len).and_then(|cfg| {
        let p = package(pkg).ok_or("unknown package")?;
        let s = duckgr_sim::new_state(&p, &cfg)?;
        Ok(store(World { p, s }))
    }))
}

/// Rebuilds a world from its state in JSON (a checkpoint).
///
/// # Safety
/// See [`read`].
#[no_mangle]
pub unsafe extern "C" fn world_restore(pkg: i32, ptr: *const u8, len: usize) -> i32 {
    handle(read(ptr, len).and_then(|v| {
        let p = package(pkg).ok_or("unknown package")?;
        let s = State::from_json(&v, &p)?;
        Ok(store(World { p, s }))
    }))
}

/// Rebuilds a world from zero: `{ cfg, cmds, until }`.
///
/// # Safety
/// See [`read`].
#[no_mangle]
pub unsafe extern "C" fn world_replay(pkg: i32, ptr: *const u8, len: usize) -> i32 {
    handle(read(ptr, len).and_then(|v| {
        let p = package(pkg).ok_or("unknown package")?;
        let cmds = v.get("cmds").and_then(Value::as_array).cloned().unwrap_or_default();
        let until = v.get("until").and_then(Value::as_i64).unwrap_or(0);
        let s = duckgr_sim::replay(&p, v.get("cfg").unwrap_or(&Value::Null), &cmds, until)?;
        Ok(store(World { p, s }))
    }))
}

#[no_mangle]
pub extern "C" fn world_free(h: i32) {
    WORLDS.with(|ws| {
        if let Some(w) = ws.borrow_mut().get_mut(h as usize) {
            *w = None;
        }
    });
}

/// Advances `n` ticks. Returns how many events are waiting for the interface.
#[no_mangle]
pub extern "C" fn world_step(h: i32, n: u32) -> u32 {
    with_world(h, |w| {
        for _ in 0..n {
            duckgr_sim::step(&mut w.s, &w.p);
        }
        w.s.events.len() as u32
    })
    .unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn world_tick(h: i32) -> f64 {
    with_world(h, |w| w.s.tick as f64).unwrap_or(-1.0)
}

#[no_mangle]
pub extern "C" fn world_passage_cost(h: i32, x: i32, y: i32) -> i32 {
    with_world(h, |w| player::passage_cost(&w.s, &w.p, x as i64, y as i64) as i32).unwrap_or(-1)
}

/// An operation on the world, described in JSON (`{"op": ..., ...}`). The
/// answer goes to the output. Returns 0, or -1 on error.
///
/// # Safety
/// See [`read`].
#[no_mangle]
pub unsafe extern "C" fn world_op(h: i32, ptr: *const u8, len: usize) -> i32 {
    let r = read(ptr, len).and_then(|op| with_world(h, |w| operate(w, &op)).unwrap_or(Err("unknown world".into())));
    match r {
        Ok(v) => {
            output(&v.to_string());
            0
        }
        Err(e) => {
            output(&e);
            -1
        }
    }
}

fn int(v: &Value, k: &str) -> Result<i64, String> {
    v.get(k).and_then(Value::as_i64).ok_or_else(|| format!("missing \"{k}\""))
}

/// The world operations, in plain Rust (so they can be tested without a browser).
pub fn operate(w: &mut World, op: &Value) -> Result<Value, String> {
    let (p, s) = (&*w.p, &mut w.s);
    Ok(match op.get("op").and_then(Value::as_str).unwrap_or("") {
        "state" => s.view_json(p),
        "hash" => Value::String(s.hash(p)),
        "apply" => duckgr_sim::apply(s, p, op.get("cmd").unwrap_or(&Value::Null)),
        "events" => Value::Array(std::mem::take(&mut s.events)),
        "actions" => player::possible_actions(s, p, int(op, "x")?, int(op, "y")?),
        "progress" => Value::Array(duckgr_sim::mission_progress(s, p)),
        "countMark" => json!(duckgr_sim::count_mark(s, p, op.get("mark").and_then(Value::as_str).unwrap_or(""))),
        "advance" => {
            let cmds = op.get("cmds").and_then(Value::as_array).cloned().unwrap_or_default();
            duckgr_sim::advance(s, p, &cmds, int(op, "until")?);
            Value::Null
        }
        "path" => {
            let o = op.get("order").ok_or("missing the order")?;
            let order = Order {
                x: int(o, "x")?,
                y: int(o, "y")?,
                action: o.get("acao").and_then(Value::as_str).and_then(Action::from_wire).ok_or("unknown action")?,
                item: None,
                element: None,
                times: None,
                done: None,
            };
            let (px, py) = (int(op, "x")?, int(op, "y")?);
            player::path(s, p, px, py, &order).map_or(Value::Null, |r| json!(r))
        }
        other => return Err(format!("unknown operation: {other}")),
    })
}
