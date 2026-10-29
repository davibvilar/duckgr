//! Compilação do pacote do mundo: valida as referências e converte os dados
//! (que chegam em JSON, na ordem em que foram escritos) em tabelas densas,
//! indexadas por número. O núcleo não conhece nenhum nome de item, máquina
//! ou cobertura: tudo vem daqui.

use std::collections::BTreeMap;

use serde_json::{Map, Value};

use crate::js::{num, round, texto, truthy};
use crate::TPS;

/// Índice numa tabela do pacote (item, cobertura, elemento...).
pub type Id = u16;

/// Lista de (item, quantidade) na ordem do pacote.
pub type Itens = Vec<(Id, i64)>;

/// Quanto custa entrar numa célula, para a classe de passagem do jogador.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Passagem {
    Livre,
    Bloqueia,
    Custo(i64),
}

#[derive(Debug)]
pub struct Substrato {
    pub id: String,
    pub nome: String,
    pub construir: bool,
    pub plantar: bool,
    /// `plantar === false`: junto com `!construir`, marca a água para o gerador.
    pub plantar_falso: bool,
    pub marcas: Vec<Id>,
    pub passagem: Passagem,
}

#[derive(Debug)]
pub struct Deposito {
    pub id: String,
    pub nome: String,
    pub item: Id,
}

#[derive(Debug)]
pub struct Cresce {
    pub para: Option<Id>,
    pub segundos: f64,
    pub bonus_vizinho: Option<Id>,
}

#[derive(Debug)]
pub struct Colheita {
    pub verbo: Option<String>,
    pub da: Itens,
    pub vira: Option<Id>,
}

#[derive(Debug)]
pub struct Cobertura {
    pub id: String,
    pub nome: String,
    pub substituivel: bool,
    /// quanto absorve de cada campo por segundo (na ordem dos campos)
    pub absorve: Vec<i64>,
    /// `None`: não regride; `Some(None)`: regride para nada
    pub regride: Option<Option<Id>>,
    pub cresce: Option<Cresce>,
    pub colheita: Option<Colheita>,
    pub marcas: Vec<Id>,
    pub passagem: Passagem,
}

#[derive(Debug)]
pub struct Item {
    pub id: String,
    pub nome: String,
    pub combustivel: i64,
    pub planta: Option<Id>,
}

#[derive(Debug)]
pub struct Receita {
    pub id: String,
    pub nome: String,
    pub entra: Itens,
    pub sai: Itens,
    pub ticks: i64,
    pub mao: bool,
    pub exige_vizinho: Option<Id>,
    pub exige_aviso: Option<String>,
}

#[derive(Debug, Default)]
pub struct Inventario {
    pub entrada: Option<i64>,
    pub saida: Option<i64>,
    pub combustivel: Option<i64>,
    pub guarda: Option<i64>,
}

#[derive(Debug)]
pub struct Elemento {
    pub id: String,
    pub nome: String,
    pub custo: Itens,
    pub exige_deposito: bool,
    /// componente "energia": quanto queima por tick
    pub consumo: Option<i64>,
    /// componente "extrai": ticks por item
    pub extrai_ticks: Option<i64>,
    /// componente "processo": receitas que a máquina sabe fazer
    pub processo: Option<Vec<Id>>,
    pub entrega: bool,
    pub inv: Inventario,
    /// componente "emite": quanto emite em cada campo (0 = nada)
    pub emite: Vec<f64>,
}

#[derive(Debug)]
pub struct Dano {
    pub limiar: f64,
    pub chance: f64,
    pub aviso: String,
}

#[derive(Debug)]
pub struct Campo {
    pub id: String,
    pub difusao_limiar: f64,
    pub difusao_divisor: f64,
    /// acima disso, nada cresce no chunk (0 = não inibe)
    pub inibe: f64,
    pub dano: Option<Dano>,
}

#[derive(Debug)]
pub struct Tecnologia {
    pub id: String,
    pub nome: String,
    pub custo: Itens,
    pub libera: Vec<Id>,
    pub requer: Vec<Id>,
    pub abre_era: i64,
}

#[derive(Debug)]
pub struct Jogador {
    pub ticks_por_celula: i64,
    pub alcance: i64,
    pub minerar_vezes: i64,
    /// ticks de cada ação, pelo nome da ação
    pub duracoes: Vec<(String, i64)>,
}

#[derive(Debug)]
pub struct Rio {
    pub substrato: Id,
    pub vau: Option<Id>,
    pub vaus: i64,
    pub posicao: f64,
    pub largura: i64,
}

#[derive(Debug)]
pub enum Mancha {
    Cobertura { tipos: Vec<(Id, f64)>, por_area: f64, escala_por: Option<String>, raio: (i64, i64), preenche: f64 },
    Deposito { tipo: Id, em: (f64, f64), raio: i64, qtd: (i64, i64) },
}

#[derive(Debug)]
pub struct Geracao {
    pub substrato: Id,
    pub cobertura: Option<Id>,
    pub rios: Vec<Rio>,
    pub manchas: Vec<Mancha>,
}

/// O pacote compilado.
#[derive(Debug)]
pub struct Pacote {
    pub id: String,
    pub versao: String,
    pub chunk: i64,
    pub padrao: Map<String, Value>,
    pub geracao: Geracao,
    pub jogador: Jogador,
    pub substratos: Vec<Substrato>,
    pub depositos: Vec<Deposito>,
    pub coberturas: Vec<Cobertura>,
    pub itens: Vec<Item>,
    pub receitas: Vec<Receita>,
    pub elementos: Vec<Elemento>,
    pub campos: Vec<Campo>,
    pub eras: Vec<String>,
    pub tecnologias: Vec<Tecnologia>,
    pub inicio_inventario: Vec<(Id, f64)>,
    pub inicio_liberados: Vec<Id>,
    pub missao_nome: String,
    pub abertura: Option<String>,
    pub metas: Value,
    /// marcas ("agua", "floresta"...) citadas no pacote
    pub marcas: Vec<String>,
    indices: BTreeMap<(u8, String), Id>,
}

// tabelas, para o índice de nomes
const T_SUB: u8 = 0;
const T_DEP: u8 = 1;
const T_COV: u8 = 2;
const T_ITEM: u8 = 3;
const T_REC: u8 = 4;
const T_EL: u8 = 5;
const T_CAMPO: u8 = 6;
const T_TEC: u8 = 7;
const T_MARCA: u8 = 8;

impl Pacote {
    fn achar(&self, t: u8, nome: &str) -> Option<Id> {
        self.indices.get(&(t, nome.to_string())).copied()
    }
    pub fn substrato(&self, nome: &str) -> Option<Id> {
        self.achar(T_SUB, nome)
    }
    pub fn deposito(&self, nome: &str) -> Option<Id> {
        self.achar(T_DEP, nome)
    }
    pub fn cobertura(&self, nome: &str) -> Option<Id> {
        self.achar(T_COV, nome)
    }
    pub fn item(&self, nome: &str) -> Option<Id> {
        self.achar(T_ITEM, nome)
    }
    pub fn receita(&self, nome: &str) -> Option<Id> {
        self.achar(T_REC, nome)
    }
    pub fn elemento(&self, nome: &str) -> Option<Id> {
        self.achar(T_EL, nome)
    }
    pub fn campo(&self, nome: &str) -> Option<Id> {
        self.achar(T_CAMPO, nome)
    }
    pub fn tecnologia(&self, nome: &str) -> Option<Id> {
        self.achar(T_TEC, nome)
    }
    pub fn marca(&self, nome: &str) -> Option<Id> {
        self.achar(T_MARCA, nome)
    }
    /// Nome do item para mensagens; se não existir, o próprio id.
    pub fn nome_item(&self, id: &str) -> String {
        match self.item(id) {
            Some(i) => self.itens[i as usize].nome.clone(),
            None => id.to_string(),
        }
    }
    pub fn duracao(&self, acao: &str) -> i64 {
        self.jogador.duracoes.iter().find(|(a, _)| a == acao).map(|(_, t)| *t).filter(|t| *t != 0).unwrap_or(1)
    }
}

/// Texto de uma chave como o JavaScript faria em `${id}`.
fn chave(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        outro => outro.to_string(),
    }
}

fn obj(v: Option<&Value>) -> impl Iterator<Item = (&String, &Value)> {
    v.and_then(Value::as_object).into_iter().flat_map(|m| m.iter())
}

fn lista(v: Option<&Value>) -> impl Iterator<Item = &Value> {
    v.and_then(Value::as_array).into_iter().flat_map(|a| a.iter())
}

fn existe(c: &Value, erros: &mut Vec<String>, tabela: &str, id: Option<&Value>, onde: &str) {
    let Some(id) = id.filter(|v| !v.is_null()) else { return };
    let k = chave(id);
    if !truthy(c.get(tabela).and_then(|t| t.get(&k))) {
        erros.push(format!("{onde}: \"{k}\" não existe em {tabela}"));
    }
}

/// Validação com as mesmas mensagens do protótipo em JavaScript.
fn validar(c: &Value) -> Vec<String> {
    let mut erros = Vec::new();
    let e = &mut erros;
    let s = |k: &String| Value::String(k.clone());
    for (id, d) in obj(c.get("depositos")) {
        existe(c, e, "itens", d.get("item"), &format!("depósito {id}"));
    }
    for (id, cv) in obj(c.get("coberturas")) {
        let onde = format!("cobertura {id}");
        if let Some(cr) = cv.get("cresce").filter(|v| truthy(Some(v))) {
            existe(c, e, "coberturas", cr.get("para"), &onde);
        }
        if let Some(r) = cv.get("regride") {
            existe(c, e, "coberturas", Some(r), &onde);
        }
        if let Some(col) = cv.get("colheita").filter(|v| truthy(Some(v))) {
            existe(c, e, "coberturas", col.get("vira"), &onde);
            for (it, _) in obj(col.get("da")) {
                existe(c, e, "itens", Some(&s(it)), &format!("colheita de {id}"));
            }
        }
    }
    for (id, it) in obj(c.get("itens")) {
        if truthy(it.get("planta")) {
            existe(c, e, "coberturas", it.get("planta"), &format!("item {id}"));
        }
    }
    for (id, r) in obj(c.get("receitas")) {
        for (it, _) in obj(r.get("entra")).chain(obj(r.get("sai"))) {
            existe(c, e, "itens", Some(&s(it)), &format!("receita {id}"));
        }
    }
    for (id, el) in obj(c.get("elementos")) {
        for (it, _) in obj(el.get("custo")) {
            existe(c, e, "itens", Some(&s(it)), &format!("custo de {id}"));
        }
        for r in lista(el.get("processo")) {
            existe(c, e, "receitas", Some(r), &format!("elemento {id}"));
        }
        for (f, _) in obj(el.get("emite")) {
            existe(c, e, "campos", Some(&s(f)), &format!("elemento {id}"));
        }
    }
    for (id, t) in obj(c.get("tecnologias")) {
        let onde = format!("tecnologia {id}");
        for (it, _) in obj(t.get("custo")) {
            existe(c, e, "itens", Some(&s(it)), &onde);
        }
        for x in lista(t.get("libera")) {
            existe(c, e, "elementos", Some(x), &onde);
        }
        for r in lista(t.get("requer")) {
            existe(c, e, "tecnologias", Some(r), &onde);
        }
    }
    let inicio = c.get("inicio");
    for x in lista(inicio.and_then(|i| i.get("liberados"))) {
        existe(c, e, "elementos", Some(x), "início");
    }
    if !truthy(c.get("jogador")) {
        e.push("falta a definição do jogador".to_string());
    }
    for (it, _) in obj(inicio.and_then(|i| i.get("inventario"))) {
        existe(c, e, "itens", Some(&s(it)), "início");
    }
    erros
}

fn ticks_de(segundos: Option<f64>) -> i64 {
    (round(segundos.unwrap_or(0.0) * TPS as f64) as i64).max(1)
}

/// As mudanças que a compilação faz nos dados (como o protótipo fazia): cada
/// receita ganha o próprio id e a duração em ticks; extração e jogador também.
fn converter(c: &mut Value) {
    if let Some(rs) = c.get_mut("receitas").and_then(Value::as_object_mut) {
        for (id, r) in rs.iter_mut() {
            if let Some(r) = r.as_object_mut() {
                let ticks = ticks_de(num(r.get("segundos")).filter(|s| *s != 0.0));
                r.insert("id".into(), Value::String(id.clone()));
                r.insert("ticks".into(), ticks.into());
            }
        }
    }
    if let Some(es) = c.get_mut("elementos").and_then(Value::as_object_mut) {
        for (_, e) in es.iter_mut() {
            if let Some(x) = e.get_mut("extrai").and_then(Value::as_object_mut) {
                let ticks = ticks_de(num(x.get("segundos")));
                x.insert("ticks".into(), ticks.into());
            }
        }
    }
    if let Some(j) = c.get_mut("jogador").and_then(Value::as_object_mut) {
        let cps = num(j.get("celulasPorSegundo")).unwrap_or(f64::NAN);
        let tpc = (round(TPS as f64 / cps) as i64).max(1);
        j.insert("ticksPorCelula".into(), tpc.into());
        if let Some(d) = j.get_mut("duracoes").and_then(Value::as_object_mut) {
            for (_, s) in d.iter_mut() {
                *s = ticks_de(s.as_f64()).into();
            }
        }
    }
}

/// Ajuda a montar as tabelas densas, juntando os erros que o protótipo não
/// pegava (números quebrados onde o núcleo exige inteiros, nomes que faltam).
struct Montador<'a> {
    c: &'a Value,
    indices: BTreeMap<(u8, String), Id>,
    marcas: Vec<String>,
    erros: Vec<String>,
}

impl<'a> Montador<'a> {
    fn indexar(&mut self, t: u8, tabela: &str) {
        for (k, (id, _)) in obj(self.c.get(tabela)).enumerate() {
            self.indices.insert((t, id.clone()), k as Id);
        }
    }
    fn id(&mut self, t: u8, v: Option<&Value>, onde: &str) -> Option<Id> {
        let k = chave(v?);
        let r = self.indices.get(&(t, k.clone())).copied();
        if r.is_none() {
            self.erros.push(format!("{onde}: \"{k}\" não existe"));
        }
        r
    }
    fn marca(&mut self, nome: &str) -> Id {
        if let Some(i) = self.indices.get(&(T_MARCA, nome.to_string())) {
            return *i;
        }
        self.marcas.push(nome.to_string());
        let i = (self.marcas.len() - 1) as Id;
        self.indices.insert((T_MARCA, nome.to_string()), i);
        i
    }
    fn marcas_de(&mut self, v: Option<&Value>) -> Vec<Id> {
        lista(v).filter_map(Value::as_str).map(|m| self.marca(m)).collect::<Vec<_>>()
    }
    fn int(&mut self, v: Option<&Value>, onde: &str) -> Option<i64> {
        match v {
            None | Some(Value::Null) => None,
            Some(Value::Number(n)) => match n.as_i64() {
                Some(i) => Some(i),
                None => {
                    let f = n.as_f64().unwrap_or(f64::NAN);
                    if f.fract() == 0.0 && f.abs() < 9e15 {
                        Some(f as i64)
                    } else {
                        self.erros.push(format!("{onde}: {n} precisa ser um número inteiro"));
                        None
                    }
                }
            },
            Some(outro) => {
                self.erros.push(format!("{onde}: {outro} precisa ser um número"));
                None
            }
        }
    }
    fn itens(&mut self, v: Option<&Value>, onde: &str) -> Itens {
        let mut r = Vec::new();
        for (it, n) in obj(v) {
            let id = self.id(T_ITEM, Some(&Value::String(it.clone())), onde);
            let n = self.int(Some(n), &format!("{onde}, {it}")).unwrap_or(0);
            if let Some(id) = id {
                r.push((id, n));
            }
        }
        r
    }
    fn passagem(&mut self, def: &Value, cls: &str, onde: &str) -> Passagem {
        match def.get("passagem").and_then(|p| p.get(cls)) {
            Some(Value::String(s)) if s == "bloqueia" => Passagem::Bloqueia,
            Some(n @ Value::Number(_)) => Passagem::Custo(self.int(Some(n), onde).unwrap_or(1)),
            _ => Passagem::Livre,
        }
    }
    fn por_campo<T: Copy>(&mut self, v: Option<&Value>, zero: T, mut f: impl FnMut(&Value) -> Option<T>) -> Vec<T> {
        let n = obj(self.c.get("campos")).count();
        let mut r = vec![zero; n];
        for (fid, x) in obj(v) {
            if let Some(k) = self.indices.get(&(T_CAMPO, fid.clone())) {
                if let Some(val) = f(x) {
                    r[*k as usize] = val;
                }
            }
        }
        r
    }
}

fn str_de(v: Option<&Value>) -> String {
    v.and_then(Value::as_str).unwrap_or("").to_string()
}

/// Compila o pacote. Devolve as tabelas densas e os dados convertidos (que a
/// interface usa para nomes, cores e descrições), ou a lista de erros.
pub fn compilar(pacote: &Value) -> Result<(Pacote, Value), String> {
    let id_pacote = pacote.get("id").map(chave).unwrap_or_else(|| "undefined".into());
    let falha = |erros: Vec<String>| format!("Pacote {id_pacote} inválido:\n- {}", erros.join("\n- "));
    let erros = validar(pacote);
    if !erros.is_empty() {
        return Err(falha(erros));
    }
    let mut convertido = pacote.clone();
    converter(&mut convertido);
    let c = &convertido;

    let mut m = Montador { c, indices: BTreeMap::new(), marcas: Vec::new(), erros: Vec::new() };
    m.indexar(T_SUB, "substratos");
    m.indexar(T_DEP, "depositos");
    m.indexar(T_COV, "coberturas");
    m.indexar(T_ITEM, "itens");
    m.indexar(T_REC, "receitas");
    m.indexar(T_EL, "elementos");
    m.indexar(T_CAMPO, "campos");
    m.indexar(T_TEC, "tecnologias");

    let jv = c.get("jogador").cloned().unwrap_or(Value::Null);
    let cls = str_de(jv.get("passagem"));
    let jogador = Jogador {
        ticks_por_celula: m.int(jv.get("ticksPorCelula"), "jogador").unwrap_or(1),
        alcance: m.int(jv.get("alcance"), "jogador.alcance").unwrap_or(0),
        minerar_vezes: m.int(jv.get("minerarVezes"), "jogador.minerarVezes").filter(|v| *v != 0).unwrap_or(1),
        duracoes: obj(jv.get("duracoes")).map(|(a, t)| (a.clone(), t.as_i64().unwrap_or(1))).collect(),
    };

    let mut substratos = Vec::new();
    for (id, s) in obj(c.get("substratos")) {
        let marcas = m.marcas_de(s.get("marcas"));
        let passagem = m.passagem(s, &cls, &format!("substrato {id}"));
        substratos.push(Substrato {
            id: id.clone(),
            nome: str_de(s.get("nome")),
            construir: truthy(s.get("construir")),
            plantar: truthy(s.get("plantar")),
            plantar_falso: s.get("plantar") == Some(&Value::Bool(false)),
            marcas,
            passagem,
        });
    }
    let mut depositos = Vec::new();
    for (id, d) in obj(c.get("depositos")) {
        let item = m.id(T_ITEM, d.get("item"), &format!("depósito {id}")).unwrap_or(0);
        depositos.push(Deposito { id: id.clone(), nome: str_de(d.get("nome")), item });
    }
    let mut coberturas = Vec::new();
    for (id, cv) in obj(c.get("coberturas")) {
        let onde = format!("cobertura {id}");
        let absorve = {
            let mut erros = Vec::new();
            let r = m.por_campo(cv.get("absorve"), 0i64, |x| match x.as_i64() {
                Some(i) => Some(i),
                None => {
                    erros.push(format!("{onde}: absorção {x} precisa ser um número inteiro"));
                    None
                }
            });
            m.erros.extend(erros);
            r
        };
        let regride = cv.get("regride").map(|r| if r.is_null() { None } else { m.id(T_COV, Some(r), &onde) });
        let cresce = cv.get("cresce").filter(|v| truthy(Some(v))).map(|cr| Cresce {
            para: cr.get("para").filter(|v| !v.is_null()).and_then(|p| m.id(T_COV, Some(p), &onde)),
            segundos: num(cr.get("segundos")).unwrap_or(f64::NAN),
            bonus_vizinho: texto(cr.get("bonusVizinho")).map(|s| s.to_string()).map(|s| m.marca(&s)),
        });
        let colheita = cv.get("colheita").filter(|v| truthy(Some(v))).map(|col| Colheita {
            verbo: texto(col.get("verbo")).map(str::to_string),
            da: m.itens(col.get("da"), &format!("colheita de {id}")),
            vira: col.get("vira").filter(|v| !v.is_null()).and_then(|p| m.id(T_COV, Some(p), &onde)),
        });
        let marcas = m.marcas_de(cv.get("marcas"));
        let passagem = m.passagem(cv, &cls, &onde);
        coberturas.push(Cobertura {
            id: id.clone(),
            nome: str_de(cv.get("nome")),
            substituivel: truthy(cv.get("substituivel")),
            absorve,
            regride,
            cresce,
            colheita,
            marcas,
            passagem,
        });
    }
    let mut itens = Vec::new();
    for (id, it) in obj(c.get("itens")) {
        let combustivel = m.int(it.get("combustivel"), &format!("item {id}")).unwrap_or(0);
        let planta = if truthy(it.get("planta")) { m.id(T_COV, it.get("planta"), &format!("item {id}")) } else { None };
        itens.push(Item { id: id.clone(), nome: str_de(it.get("nome")), combustivel, planta });
    }
    let mut receitas = Vec::new();
    for (id, r) in obj(c.get("receitas")) {
        let onde = format!("receita {id}");
        let exige = r.get("exige");
        receitas.push(Receita {
            id: id.clone(),
            nome: str_de(r.get("nome")),
            entra: m.itens(r.get("entra"), &onde),
            sai: m.itens(r.get("sai"), &onde),
            ticks: r.get("ticks").and_then(Value::as_i64).unwrap_or(1),
            mao: truthy(r.get("mao")),
            exige_vizinho: texto(exige.and_then(|e| e.get("vizinho"))).map(str::to_string).map(|s| m.marca(&s)),
            exige_aviso: texto(exige.and_then(|e| e.get("aviso"))).map(str::to_string),
        });
    }
    let mut elementos = Vec::new();
    for (id, e) in obj(c.get("elementos")) {
        let onde = format!("elemento {id}");
        let inv = e.get("inventario");
        let iv = |m: &mut Montador, k: &str| m.int(inv.and_then(|i| i.get(k)), &format!("{onde}, inventário {k}"));
        let inv = Inventario {
            entrada: iv(&mut m, "entrada"),
            saida: iv(&mut m, "saida"),
            combustivel: iv(&mut m, "combustivel"),
            guarda: iv(&mut m, "guarda"),
        };
        let consumo = e
            .get("energia")
            .filter(|v| truthy(Some(v)))
            .map(|en| m.int(en.get("consumo"), &format!("{onde}, consumo")).unwrap_or(0));
        let processo = if truthy(e.get("processo")) {
            Some(lista(e.get("processo")).filter_map(|r| m.id(T_REC, Some(r), &onde)).collect())
        } else {
            None
        };
        elementos.push(Elemento {
            id: id.clone(),
            nome: str_de(e.get("nome")),
            custo: m.itens(e.get("custo"), &format!("custo de {id}")),
            exige_deposito: e.get("exige").and_then(Value::as_str) == Some("deposito"),
            consumo,
            extrai_ticks: e
                .get("extrai")
                .filter(|v| truthy(Some(v)))
                .map(|x| x.get("ticks").and_then(Value::as_i64).unwrap_or(1)),
            processo,
            entrega: truthy(e.get("entrega")),
            inv,
            emite: m.por_campo(e.get("emite"), 0.0, Value::as_f64),
        });
    }
    let mut campos = Vec::new();
    for (id, f) in obj(c.get("campos")) {
        let dif = f.get("difusao");
        let divisor = num(dif.and_then(|d| d.get("divisor"))).unwrap_or(0.0);
        if divisor <= 0.0 {
            m.erros.push(format!("campo {id}: a difusão precisa de um divisor maior que zero"));
        }
        campos.push(Campo {
            id: id.clone(),
            difusao_limiar: num(dif.and_then(|d| d.get("limiar"))).unwrap_or(0.0),
            difusao_divisor: divisor,
            inibe: num(f.get("inibeCrescimento")).unwrap_or(0.0),
            dano: f.get("dano").filter(|v| truthy(Some(v))).map(|d| Dano {
                limiar: num(d.get("limiar")).unwrap_or(f64::NAN),
                chance: num(d.get("chance")).unwrap_or(f64::NAN),
                aviso: str_de(d.get("aviso")),
            }),
        });
    }
    let eras = lista(c.get("eras")).map(|e| str_de(e.get("nome"))).collect();
    let mut tecnologias = Vec::new();
    for (id, t) in obj(c.get("tecnologias")) {
        let onde = format!("tecnologia {id}");
        tecnologias.push(Tecnologia {
            id: id.clone(),
            nome: str_de(t.get("nome")),
            custo: m.itens(t.get("custo"), &onde),
            libera: lista(t.get("libera")).filter_map(|x| m.id(T_EL, Some(x), &onde)).collect(),
            requer: lista(t.get("requer")).filter_map(|x| m.id(T_TEC, Some(x), &onde)).collect(),
            abre_era: m.int(t.get("abreEra"), &onde).unwrap_or(0),
        });
    }

    let inicio = c.get("inicio");
    let mut inicio_inventario = Vec::new();
    for (it, n) in obj(inicio.and_then(|i| i.get("inventario"))) {
        if let Some(id) = m.id(T_ITEM, Some(&Value::String(it.clone())), "início") {
            inicio_inventario.push((id, n.as_f64().unwrap_or(f64::NAN)));
        }
    }
    let inicio_liberados =
        lista(inicio.and_then(|i| i.get("liberados"))).filter_map(|x| m.id(T_EL, Some(x), "início")).collect();

    let mapa = c.get("mapa");
    let chunk = m.int(mapa.and_then(|x| x.get("chunk")), "mapa.chunk").unwrap_or(0);
    if chunk <= 0 {
        m.erros.push("mapa.chunk precisa ser um inteiro maior que zero".into());
    }
    let g = mapa.and_then(|x| x.get("geracao"));
    let gget = |k: &str| g.and_then(|g| g.get(k));
    let substrato = m.id(T_SUB, gget("substrato"), "geração").unwrap_or(0);
    let cobertura = gget("cobertura").filter(|v| !v.is_null()).and_then(|v| m.id(T_COV, Some(v), "geração"));
    let mut rios = Vec::new();
    for r in lista(gget("rios")) {
        rios.push(Rio {
            substrato: m.id(T_SUB, r.get("substrato"), "rio").unwrap_or(0),
            vau: if truthy(r.get("vau")) { m.id(T_SUB, r.get("vau"), "rio") } else { None },
            vaus: m.int(r.get("vaus"), "rio.vaus").unwrap_or(0),
            posicao: num(r.get("posicao")).unwrap_or(f64::NAN),
            largura: m.int(r.get("largura"), "rio.largura").unwrap_or(0),
        });
    }
    let mut manchas = Vec::new();
    for x in lista(gget("manchas")) {
        let par_int = |m: &mut Montador, v: Option<&Value>, onde: &str| {
            let a = v.and_then(|v| v.get(0));
            let b = v.and_then(|v| v.get(1));
            (m.int(a, onde).unwrap_or(0), m.int(b, onde).unwrap_or(0))
        };
        match x.get("camada").and_then(Value::as_str) {
            Some("cobertura") => {
                let mut tipos = Vec::new();
                for (t, p) in obj(x.get("tipos")) {
                    if let Some(id) = m.id(T_COV, Some(&Value::String(t.clone())), "mancha") {
                        tipos.push((id, p.as_f64().unwrap_or(f64::NAN)));
                    }
                }
                let raio = par_int(&mut m, x.get("raio"), "mancha.raio");
                manchas.push(Mancha::Cobertura {
                    tipos,
                    por_area: num(x.get("porArea")).unwrap_or(f64::NAN),
                    escala_por: x.get("escalaPor").map(chave),
                    raio,
                    preenche: num(x.get("preenche")).unwrap_or(f64::NAN),
                });
            }
            Some("deposito") => {
                let tipo = m.id(T_DEP, x.get("tipo"), "mancha").unwrap_or(0);
                let em = x.get("em");
                let raio = m.int(x.get("raio"), "mancha.raio").unwrap_or(0);
                let qtd = par_int(&mut m, x.get("qtd"), "mancha.qtd");
                manchas.push(Mancha::Deposito {
                    tipo,
                    em: (
                        num(em.and_then(|e| e.get(0))).unwrap_or(f64::NAN),
                        num(em.and_then(|e| e.get(1))).unwrap_or(f64::NAN),
                    ),
                    raio,
                    qtd,
                });
            }
            _ => {}
        }
    }

    let missao = c.get("missao");
    let p = Pacote {
        id: id_pacote.clone(),
        versao: c.get("versao").map(chave).unwrap_or_default(),
        chunk,
        padrao: c.get("padrao").and_then(Value::as_object).cloned().unwrap_or_default(),
        geracao: Geracao { substrato, cobertura, rios, manchas },
        jogador,
        substratos,
        depositos,
        coberturas,
        itens,
        receitas,
        elementos,
        campos,
        eras,
        tecnologias,
        inicio_inventario,
        inicio_liberados,
        missao_nome: str_de(missao.and_then(|x| x.get("nome"))),
        abertura: texto(missao.and_then(|x| x.get("abertura"))).map(str::to_string),
        metas: missao.and_then(|x| x.get("metas")).cloned().unwrap_or(Value::Array(vec![])),
        marcas: m.marcas,
        indices: m.indices,
    };
    if !m.erros.is_empty() {
        return Err(falha(m.erros));
    }
    Ok((p, convertido))
}
