//! O estado do mundo, em tabelas densas, e a sua forma em JSON.
//!
//! O JSON é o mesmo que o protótipo em JavaScript produzia, na mesma ordem de
//! chaves. Por isso ele serve ao mesmo tempo de checkpoint do save, de vista
//! para a interface e de base para o hash, e os saves antigos continuam
//! abrindo (e conferindo o hash) no núcleo novo.

use std::collections::VecDeque;

use serde_json::{json, Map, Value};

use crate::js::fnv_utf16;
use crate::pacote::{Id, Pacote};

/// Quantidades por item, na ordem em que entraram (como um objeto do
/// JavaScript). A ordem decide empates: qual item sai primeiro da máquina,
/// qual combustível queima antes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Bolsa(pub Vec<(Id, i64)>);

impl Bolsa {
    pub fn get(&self, it: Id) -> i64 {
        self.0.iter().find(|(k, _)| *k == it).map_or(0, |(_, n)| *n)
    }
    /// Soma `n` ao item; se ficar em zero ou menos, o item sai da bolsa.
    pub fn por(&mut self, it: Id, n: i64) {
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
    pub fn primeiro(&self) -> Option<Id> {
        self.0.first().map(|(k, _)| *k)
    }
    pub fn vazia(&self) -> bool {
        self.0.is_empty()
    }
}

/// Um elemento construído (máquina, baú...).
#[derive(Clone, Debug)]
pub struct Ent {
    pub id: u32,
    pub p: Id,
    pub x: i64,
    pub y: i64,
    pub ent: Bolsa,
    pub sai: Bolsa,
    pub comb: Bolsa,
    pub queima: i64,
    pub prog: i64,
    pub rec: Option<Id>,
    pub ativo: i64,
    pub parado: String,
}

/// As ações que o jogador sabe fazer. O núcleo não sabe o que é árvore ou
/// minério: as ações são genéricas e os nomes vêm do pacote.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Acao {
    Ir,
    Colher,
    Minerar,
    Plantar,
    Construir,
    Desmontar,
    Recolher,
    Abastecer,
    Fornecer,
}

impl Acao {
    pub const TODAS: [Acao; 9] = [
        Acao::Ir,
        Acao::Colher,
        Acao::Minerar,
        Acao::Plantar,
        Acao::Construir,
        Acao::Desmontar,
        Acao::Recolher,
        Acao::Abastecer,
        Acao::Fornecer,
    ];
    pub fn nome(self) -> &'static str {
        match self {
            Acao::Ir => "ir",
            Acao::Colher => "colher",
            Acao::Minerar => "minerar",
            Acao::Plantar => "plantar",
            Acao::Construir => "construir",
            Acao::Desmontar => "desmontar",
            Acao::Recolher => "recolher",
            Acao::Abastecer => "abastecer",
            Acao::Fornecer => "fornecer",
        }
    }
    pub fn de(nome: &str) -> Option<Acao> {
        Acao::TODAS.into_iter().find(|a| a.nome() == nome)
    }
}

/// Uma ordem na fila do jogador: "vá até perto desta célula e faça isto".
/// Item e elemento ficam como texto, do jeito que vieram no comando.
#[derive(Clone, Debug)]
pub struct Ordem {
    pub x: i64,
    pub y: i64,
    pub acao: Acao,
    pub item: Option<String>,
    pub elemento: Option<String>,
    pub vezes: Option<i64>,
    pub feitas: Option<i64>,
}

#[derive(Clone, Debug, Default)]
pub struct Jog {
    pub x: i64,
    pub y: i64,
    pub fila: Vec<Ordem>,
    pub cam: Vec<i64>,
    pub passo: i64,
    pub tarefa: i64,
}

#[derive(Clone, Debug)]
pub struct Registro {
    pub t: i64,
    pub txt: String,
    pub cls: String,
}

#[derive(Clone, Debug)]
pub struct Missao {
    pub estado: String,
    /// as metas como vieram da configuração (texto livre do pacote)
    pub metas: Vec<Value>,
    /// referência de cada marca no começo (para metas de cobertura)
    pub refs: Vec<(String, i64)>,
}

/// O mundo inteiro num tick.
#[derive(Clone, Debug)]
pub struct Estado {
    pub tick: i64,
    pub rng: i64,
    pub l: i64,
    pub a: i64,
    pub cl: i64,
    pub ca: i64,
    pub sub: Vec<Id>,
    pub dep: Vec<Option<Id>>,
    pub dep_qtd: Vec<i64>,
    pub cov: Vec<Option<Id>>,
    pub cres: Vec<i64>,
    pub occ: Vec<u32>,
    pub ents: Vec<Ent>,
    pub prox: u32,
    pub inv: Bolsa,
    pub prod: Bolsa,
    pub campos: Vec<Vec<i64>>,
    pub techs: Vec<Id>,
    pub era: i64,
    pub liberados: Vec<Id>,
    pub log: VecDeque<Registro>,
    pub ultimo_dano: i64,
    pub missao: Missao,
    pub jog: Jog,
    /// transitório: avisos para a interface, fora do hash e do save
    pub eventos: Vec<Value>,
}

pub const LOG_MAX: usize = 80;

impl Estado {
    pub fn idx(&self, x: i64, y: i64) -> usize {
        (x + y * self.l) as usize
    }
    pub fn dentro(&self, x: i64, y: i64) -> bool {
        x >= 0 && y >= 0 && x < self.l && y < self.a
    }
    pub fn ent_em(&self, x: i64, y: i64) -> Option<usize> {
        if !self.dentro(x, y) {
            return None;
        }
        let id = self.occ[self.idx(x, y)];
        if id == 0 {
            return None;
        }
        // ids crescem na ordem de construção, e a lista mantém essa ordem
        self.ents.binary_search_by_key(&id, |e| e.id).ok()
    }
    pub fn registrar(&mut self, txt: impl Into<String>, cls: &str) {
        self.log.push_front(Registro { t: self.tick, txt: txt.into(), cls: cls.to_string() });
        if self.log.len() > LOG_MAX {
            self.log.pop_back();
        }
    }

    /// O estado em JSON, como o protótipo em JavaScript (sem os eventos).
    pub fn para_json(&self, p: &Pacote) -> Value {
        fn nomes<'a>(v: &[Id], f: &dyn Fn(Id) -> &'a str) -> Value {
            Value::Array(v.iter().map(|i| Value::from(f(*i))).collect())
        }
        fn opc<'a>(v: &[Option<Id>], f: &dyn Fn(Id) -> &'a str) -> Value {
            Value::Array(v.iter().map(|i| i.map_or(Value::Null, |i| Value::from(f(i)))).collect())
        }
        let bolsa = |b: &Bolsa| {
            Value::Object(b.0.iter().map(|(k, n)| (p.itens[*k as usize].id.clone(), Value::from(*n))).collect())
        };
        let el = |i: Id| p.elementos[i as usize].id.as_str();
        let mut m = Map::new();
        m.insert("tick".into(), self.tick.into());
        m.insert("rng".into(), self.rng.into());
        m.insert("L".into(), self.l.into());
        m.insert("A".into(), self.a.into());
        m.insert("CL".into(), self.cl.into());
        m.insert("CA".into(), self.ca.into());
        m.insert("sub".into(), nomes(&self.sub, &|i| p.substratos[i as usize].id.as_str()));
        m.insert("dep".into(), opc(&self.dep, &|i| p.depositos[i as usize].id.as_str()));
        m.insert("depQtd".into(), self.dep_qtd.clone().into());
        m.insert("cov".into(), opc(&self.cov, &|i| p.coberturas[i as usize].id.as_str()));
        m.insert("cres".into(), self.cres.clone().into());
        m.insert("occ".into(), self.occ.clone().into());
        let ents = self
            .ents
            .iter()
            .map(|e| {
                json!({
                    "id": e.id, "p": el(e.p), "x": e.x, "y": e.y,
                    "ent": bolsa(&e.ent), "sai": bolsa(&e.sai), "comb": bolsa(&e.comb),
                    "queima": e.queima, "prog": e.prog,
                    "rec": e.rec.map(|r| p.receitas[r as usize].id.as_str()),
                    "ativo": e.ativo, "parado": e.parado,
                })
            })
            .collect();
        m.insert("ents".into(), Value::Array(ents));
        m.insert("prox".into(), self.prox.into());
        m.insert("inv".into(), bolsa(&self.inv));
        m.insert("prod".into(), bolsa(&self.prod));
        m.insert(
            "campos".into(),
            Value::Object(p.campos.iter().zip(&self.campos).map(|(c, v)| (c.id.clone(), v.clone().into())).collect()),
        );
        m.insert("techs".into(), nomes(&self.techs, &|i| p.tecnologias[i as usize].id.as_str()));
        m.insert("era".into(), self.era.into());
        m.insert("liberados".into(), nomes(&self.liberados, &el));
        let log = self.log.iter().map(|r| json!({ "t": r.t, "txt": r.txt, "cls": r.cls })).collect();
        m.insert("log".into(), Value::Array(log));
        m.insert("ultimoDano".into(), self.ultimo_dano.into());
        let refs: Map<String, Value> = self.missao.refs.iter().map(|(k, n)| (k.clone(), Value::from(*n))).collect();
        m.insert("missao".into(), json!({ "estado": self.missao.estado, "metas": self.missao.metas, "ref": refs }));
        let fila: Vec<Value> = self.jog.fila.iter().map(|o| o.para_json()).collect();
        m.insert(
            "jog".into(),
            json!({ "x": self.jog.x, "y": self.jog.y, "fila": fila, "cam": self.jog.cam, "passo": self.jog.passo, "tarefa": self.jog.tarefa }),
        );
        Value::Object(m)
    }

    /// Hash do mundo: FNV-1a do JSON, igual ao do protótipo.
    pub fn hash(&self, p: &Pacote) -> String {
        fnv_utf16(&self.para_json(p).to_string())
    }

    /// Recria o estado a partir do JSON (de um checkpoint, novo ou antigo).
    pub fn de_json(v: &Value, p: &Pacote) -> Result<Estado, String> {
        Leitor { p }.estado(v)
    }
}

impl Ordem {
    pub fn para_json(&self) -> Value {
        let mut m = Map::new();
        m.insert("x".into(), self.x.into());
        m.insert("y".into(), self.y.into());
        m.insert("acao".into(), self.acao.nome().into());
        if let Some(i) = &self.item {
            m.insert("item".into(), i.clone().into());
        }
        if let Some(e) = &self.elemento {
            m.insert("elemento".into(), e.clone().into());
        }
        if let Some(v) = self.vezes {
            m.insert("vezes".into(), v.into());
        }
        if let Some(f) = self.feitas {
            m.insert("feitas".into(), f.into());
        }
        Value::Object(m)
    }
}

struct Leitor<'a> {
    p: &'a Pacote,
}

fn campo<'v>(v: &'v Value, k: &str) -> Result<&'v Value, String> {
    v.get(k).ok_or_else(|| format!("falta \"{k}\" no estado"))
}
fn int(v: &Value, k: &str) -> Result<i64, String> {
    let x = campo(v, k)?;
    x.as_i64()
        .or_else(|| x.as_f64().filter(|f| f.fract() == 0.0).map(|f| f as i64))
        .ok_or_else(|| format!("\"{k}\" precisa ser inteiro"))
}
fn arr<'v>(v: &'v Value, k: &str) -> Result<&'v Vec<Value>, String> {
    campo(v, k)?.as_array().ok_or_else(|| format!("\"{k}\" precisa ser uma lista"))
}
fn ints(v: &Value, k: &str) -> Result<Vec<i64>, String> {
    arr(v, k)?.iter().map(|x| x.as_i64().ok_or_else(|| format!("\"{k}\" precisa ter só inteiros"))).collect()
}

impl Leitor<'_> {
    fn id(&self, v: &Value, achar: impl Fn(&str) -> Option<Id>, o_que: &str) -> Result<Id, String> {
        let s = v.as_str().ok_or_else(|| format!("{o_que} precisa ser texto"))?;
        achar(s).ok_or_else(|| format!("{o_que} \"{s}\" não existe neste pacote"))
    }
    fn opc(&self, v: &Value, achar: impl Fn(&str) -> Option<Id>, o_que: &str) -> Result<Option<Id>, String> {
        if v.is_null() {
            Ok(None)
        } else {
            self.id(v, achar, o_que).map(Some)
        }
    }
    fn bolsa(&self, v: &Value) -> Result<Bolsa, String> {
        let m = v.as_object().ok_or("inventário precisa ser um objeto")?;
        let mut b = Bolsa::default();
        for (k, n) in m {
            let it = self.p.item(k).ok_or_else(|| format!("item \"{k}\" não existe neste pacote"))?;
            b.0.push((it, n.as_i64().ok_or("quantidade precisa ser inteira")?));
        }
        Ok(b)
    }
    fn estado(&self, v: &Value) -> Result<Estado, String> {
        let p = self.p;
        let (l, a) = (int(v, "L")?, int(v, "A")?);
        let n = (l * a) as usize;
        let sub = arr(v, "sub")?
            .iter()
            .map(|x| self.id(x, |s| p.substrato(s), "substrato"))
            .collect::<Result<Vec<_>, _>>()?;
        let dep =
            arr(v, "dep")?.iter().map(|x| self.opc(x, |s| p.deposito(s), "depósito")).collect::<Result<Vec<_>, _>>()?;
        let cov = arr(v, "cov")?
            .iter()
            .map(|x| self.opc(x, |s| p.cobertura(s), "cobertura"))
            .collect::<Result<Vec<_>, _>>()?;
        let dep_qtd = ints(v, "depQtd")?;
        let cres = ints(v, "cres")?;
        let occ = ints(v, "occ")?.into_iter().map(|x| x as u32).collect::<Vec<_>>();
        if [sub.len(), dep.len(), cov.len(), dep_qtd.len(), cres.len(), occ.len()].iter().any(|k| *k != n) {
            return Err("as camadas do mapa não têm o tamanho L×A".into());
        }
        let mut ents = Vec::new();
        for e in arr(v, "ents")? {
            ents.push(Ent {
                id: int(e, "id")? as u32,
                p: self.id(campo(e, "p")?, |s| p.elemento(s), "elemento")?,
                x: int(e, "x")?,
                y: int(e, "y")?,
                ent: self.bolsa(campo(e, "ent")?)?,
                sai: self.bolsa(campo(e, "sai")?)?,
                comb: self.bolsa(campo(e, "comb")?)?,
                queima: int(e, "queima")?,
                prog: int(e, "prog")?,
                rec: self.opc(campo(e, "rec")?, |s| p.receita(s), "receita")?,
                ativo: int(e, "ativo")?,
                parado: campo(e, "parado")?.as_str().unwrap_or("").to_string(),
            });
        }
        if ents.windows(2).any(|w| w[0].id >= w[1].id) {
            return Err("os elementos precisam estar em ordem de construção".into());
        }
        let campos_v = campo(v, "campos")?;
        let mut campos = Vec::new();
        for c in &p.campos {
            campos.push(ints(campos_v, &c.id)?);
        }
        let ids = |k: &str, achar: &dyn Fn(&str) -> Option<Id>, o_que: &str| -> Result<Vec<Id>, String> {
            arr(v, k)?.iter().map(|x| self.id(x, achar, o_que)).collect()
        };
        let mut log = VecDeque::new();
        for r in arr(v, "log")? {
            log.push_back(Registro {
                t: int(r, "t")?,
                txt: campo(r, "txt")?.as_str().unwrap_or("").to_string(),
                cls: campo(r, "cls")?.as_str().unwrap_or("").to_string(),
            });
        }
        let mv = campo(v, "missao")?;
        let missao = Missao {
            estado: campo(mv, "estado")?.as_str().unwrap_or("andamento").to_string(),
            metas: arr(mv, "metas")?.clone(),
            refs: campo(mv, "ref")?
                .as_object()
                .ok_or("ref precisa ser um objeto")?
                .iter()
                .map(|(k, n)| (k.clone(), n.as_i64().unwrap_or(0)))
                .collect(),
        };
        let jv = campo(v, "jog")?;
        let mut fila = Vec::new();
        for o in arr(jv, "fila")? {
            fila.push(Ordem {
                x: int(o, "x")?,
                y: int(o, "y")?,
                acao: Acao::de(campo(o, "acao")?.as_str().unwrap_or("")).ok_or("ação desconhecida na fila")?,
                item: o.get("item").and_then(Value::as_str).map(str::to_string),
                elemento: o.get("elemento").and_then(Value::as_str).map(str::to_string),
                vezes: o.get("vezes").and_then(Value::as_i64),
                feitas: o.get("feitas").and_then(Value::as_i64),
            });
        }
        let jog = Jog {
            x: int(jv, "x")?,
            y: int(jv, "y")?,
            fila,
            cam: ints(jv, "cam")?,
            passo: int(jv, "passo")?,
            tarefa: int(jv, "tarefa")?,
        };
        Ok(Estado {
            tick: int(v, "tick")?,
            rng: int(v, "rng")?,
            l,
            a,
            cl: int(v, "CL")?,
            ca: int(v, "CA")?,
            sub,
            dep,
            dep_qtd,
            cov,
            cres,
            occ,
            ents,
            prox: int(v, "prox")? as u32,
            inv: self.bolsa(campo(v, "inv")?)?,
            prod: self.bolsa(campo(v, "prod")?)?,
            campos,
            techs: ids("techs", &|s| p.tecnologia(s), "tecnologia")?,
            era: int(v, "era")?,
            liberados: ids("liberados", &|s| p.elemento(s), "elemento")?,
            log,
            ultimo_dano: int(v, "ultimoDano")?,
            missao,
            jog,
            eventos: Vec::new(),
        })
    }
}
