//! O núcleo do DuckGR para o navegador.
//!
//! A interface com o JavaScript é mínima e sem dependências: o JavaScript
//! escreve um texto JSON na memória do módulo, chama uma função e lê a
//! resposta (também JSON) de um buffer de saída. Quem fala com este módulo
//! é `web/plataforma/nucleo.js`; o resto da interface web não sabe que ele
//! existe.
//!
//! Mundos e pacotes vivem aqui dentro e são identificados por um número.

use std::cell::RefCell;
use std::rc::Rc;

use duckgr_sim::{estado::Acao, estado::Ordem, jogador, Estado, Pacote};
use serde_json::{json, Value};

pub struct Mundo {
    pub p: Rc<Pacote>,
    pub s: Estado,
}

thread_local! {
    static PACOTES: RefCell<Vec<Rc<Pacote>>> = const { RefCell::new(Vec::new()) };
    static MUNDOS: RefCell<Vec<Option<Mundo>>> = const { RefCell::new(Vec::new()) };
    static SAIDA: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

fn sair(txt: &str) {
    SAIDA.with(|s| {
        let mut s = s.borrow_mut();
        s.clear();
        s.extend_from_slice(txt.as_bytes());
    });
}

fn guardar(m: Mundo) -> i32 {
    MUNDOS.with(|ms| {
        let mut ms = ms.borrow_mut();
        match ms.iter().position(Option::is_none) {
            Some(k) => {
                ms[k] = Some(m);
                k as i32
            }
            None => {
                ms.push(Some(m));
                (ms.len() - 1) as i32
            }
        }
    })
}

fn com_mundo<T>(h: i32, f: impl FnOnce(&mut Mundo) -> T) -> Option<T> {
    MUNDOS.with(|ms| ms.borrow_mut().get_mut(h as usize).and_then(Option::as_mut).map(f))
}

fn pacote(h: i32) -> Option<Rc<Pacote>> {
    PACOTES.with(|ps| ps.borrow().get(h as usize).cloned())
}

/// Lê o texto JSON que o JavaScript deixou na memória.
///
/// # Safety
/// `ptr` e `len` precisam apontar para bytes alocados com [`alocar`].
unsafe fn ler(ptr: *const u8, len: usize) -> Result<Value, String> {
    let bytes = std::slice::from_raw_parts(ptr, len);
    serde_json::from_slice(bytes).map_err(|e| format!("JSON inválido: {e}"))
}

fn resultado(r: Result<i32, String>) -> i32 {
    match r {
        Ok(h) => h,
        Err(e) => {
            sair(&e);
            -1
        }
    }
}

/// Reserva `len` bytes para o JavaScript escrever.
#[no_mangle]
pub extern "C" fn alocar(len: usize) -> *mut u8 {
    let mut v = Vec::<u8>::with_capacity(len.max(1));
    let p = v.as_mut_ptr();
    std::mem::forget(v);
    p
}

/// Devolve os bytes reservados com [`alocar`].
///
/// # Safety
/// `ptr` e `len` precisam ser exatamente os de uma chamada a [`alocar`].
#[no_mangle]
pub unsafe extern "C" fn desalocar(ptr: *mut u8, len: usize) {
    drop(Vec::from_raw_parts(ptr, 0, len.max(1)));
}

#[no_mangle]
pub extern "C" fn saida_ptr() -> *const u8 {
    SAIDA.with(|s| s.borrow().as_ptr())
}

#[no_mangle]
pub extern "C" fn saida_len() -> usize {
    SAIDA.with(|s| s.borrow().len())
}

/// Compila um pacote (JSON). Devolve o número do pacote e deixa na saída os
/// dados convertidos; ou -1 e a mensagem de erro.
///
/// # Safety
/// Ver [`ler`].
#[no_mangle]
pub unsafe extern "C" fn pacote_compilar(ptr: *const u8, len: usize) -> i32 {
    resultado(ler(ptr, len).and_then(|v| {
        let (p, convertido) = duckgr_sim::compilar(&v)?;
        sair(&convertido.to_string());
        Ok(PACOTES.with(|ps| {
            let mut ps = ps.borrow_mut();
            ps.push(Rc::new(p));
            (ps.len() - 1) as i32
        }))
    }))
}

/// Cria um mundo novo a partir da configuração (JSON).
///
/// # Safety
/// Ver [`ler`].
#[no_mangle]
pub unsafe extern "C" fn mundo_novo(pac: i32, ptr: *const u8, len: usize) -> i32 {
    resultado(ler(ptr, len).and_then(|cfg| {
        let p = pacote(pac).ok_or("pacote desconhecido")?;
        let s = duckgr_sim::novo_estado(&p, &cfg)?;
        Ok(guardar(Mundo { p, s }))
    }))
}

/// Recria um mundo a partir do estado em JSON (um checkpoint).
///
/// # Safety
/// Ver [`ler`].
#[no_mangle]
pub unsafe extern "C" fn mundo_restaurar(pac: i32, ptr: *const u8, len: usize) -> i32 {
    resultado(ler(ptr, len).and_then(|v| {
        let p = pacote(pac).ok_or("pacote desconhecido")?;
        let s = Estado::de_json(&v, &p)?;
        Ok(guardar(Mundo { p, s }))
    }))
}

/// Refaz um mundo do zero: `{ cfg, cmds, ate }`.
///
/// # Safety
/// Ver [`ler`].
#[no_mangle]
pub unsafe extern "C" fn mundo_rodar(pac: i32, ptr: *const u8, len: usize) -> i32 {
    resultado(ler(ptr, len).and_then(|v| {
        let p = pacote(pac).ok_or("pacote desconhecido")?;
        let cmds = v.get("cmds").and_then(Value::as_array).cloned().unwrap_or_default();
        let ate = v.get("ate").and_then(Value::as_i64).unwrap_or(0);
        let s = duckgr_sim::rodar(&p, v.get("cfg").unwrap_or(&Value::Null), &cmds, ate)?;
        Ok(guardar(Mundo { p, s }))
    }))
}

#[no_mangle]
pub extern "C" fn mundo_liberar(h: i32) {
    MUNDOS.with(|ms| {
        if let Some(m) = ms.borrow_mut().get_mut(h as usize) {
            *m = None;
        }
    });
}

/// Avança `n` ticks. Devolve quantos eventos estão esperando a interface.
#[no_mangle]
pub extern "C" fn mundo_passo(h: i32, n: u32) -> u32 {
    com_mundo(h, |m| {
        for _ in 0..n {
            duckgr_sim::passo(&mut m.s, &m.p);
        }
        m.s.eventos.len() as u32
    })
    .unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn mundo_tick(h: i32) -> f64 {
    com_mundo(h, |m| m.s.tick as f64).unwrap_or(-1.0)
}

#[no_mangle]
pub extern "C" fn mundo_custo_passagem(h: i32, x: i32, y: i32) -> i32 {
    com_mundo(h, |m| jogador::custo_passagem(&m.s, &m.p, x as i64, y as i64) as i32).unwrap_or(-1)
}

/// Uma operação sobre o mundo, descrita em JSON (`{"op": ..., ...}`). A
/// resposta fica na saída. Devolve 0, ou -1 se deu erro.
///
/// # Safety
/// Ver [`ler`].
#[no_mangle]
pub unsafe extern "C" fn mundo_op(h: i32, ptr: *const u8, len: usize) -> i32 {
    let r = ler(ptr, len).and_then(|op| com_mundo(h, |m| operar(m, &op)).unwrap_or(Err("mundo desconhecido".into())));
    match r {
        Ok(v) => {
            sair(&v.to_string());
            0
        }
        Err(e) => {
            sair(&e);
            -1
        }
    }
}

fn int(v: &Value, k: &str) -> Result<i64, String> {
    v.get(k).and_then(Value::as_i64).ok_or_else(|| format!("falta \"{k}\""))
}

/// As operações do mundo, em Rust puro (para poder testar sem o navegador).
pub fn operar(m: &mut Mundo, op: &Value) -> Result<Value, String> {
    let (p, s) = (&*m.p, &mut m.s);
    Ok(match op.get("op").and_then(Value::as_str).unwrap_or("") {
        "estado" => s.para_json(p),
        "hash" => Value::String(s.hash(p)),
        "aplicar" => duckgr_sim::aplicar(s, p, op.get("cmd").unwrap_or(&Value::Null)),
        "eventos" => Value::Array(std::mem::take(&mut s.eventos)),
        "acoes" => jogador::acoes_possiveis(s, p, int(op, "x")?, int(op, "y")?),
        "progresso" => Value::Array(duckgr_sim::progresso_missao(s, p)),
        "contarMarca" => json!(duckgr_sim::contar_marca(s, p, op.get("marca").and_then(Value::as_str).unwrap_or(""))),
        "avancar" => {
            let cmds = op.get("cmds").and_then(Value::as_array).cloned().unwrap_or_default();
            duckgr_sim::avancar(s, p, &cmds, int(op, "ate")?);
            Value::Null
        }
        "caminho" => {
            let o = op.get("ordem").ok_or("falta a ordem")?;
            let ordem = Ordem {
                x: int(o, "x")?,
                y: int(o, "y")?,
                acao: o.get("acao").and_then(Value::as_str).and_then(Acao::de).ok_or("ação desconhecida")?,
                item: None,
                elemento: None,
                vezes: None,
                feitas: None,
            };
            let (jx, jy) = (int(op, "x")?, int(op, "y")?);
            jogador::caminho(s, p, jx, jy, &ordem).map_or(Value::Null, |r| json!(r))
        }
        outro => return Err(format!("operação desconhecida: {outro}")),
    })
}
