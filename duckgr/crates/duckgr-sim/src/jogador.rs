//! O jogador: um agente com posição, velocidade e alcance. Ele só age no que
//! está ao alcance; para o resto, anda até lá. O jogador recebe ordens
//! (comando "ordem") que entram numa fila; cada ordem é "vá até perto desta
//! célula e faça esta ação". Andar, esperar e agir acontecem tick a tick,
//! então tudo continua determinístico e entra no replay.

use serde_json::{json, Map, Value};

use crate::estado::{Acao, Bolsa, Ent, Estado, Ordem};
use crate::js::{minusc, num, texto, to_int32, truthy};
use crate::pacote::{Id, Itens, Pacote, Passagem};
use crate::{faltam, registrar_evento, tem_itens, VIZINHOS};

/// Quanto custa (em múltiplos do passo) entrar numa célula; -1 bloqueia.
pub fn custo_passagem(s: &Estado, p: &Pacote, x: i64, y: i64) -> i64 {
    if !s.dentro(x, y) {
        return -1;
    }
    let i = s.idx(x, y);
    if s.occ[i] != 0 {
        return -1;
    }
    let mut custo = 1;
    let sub = Some(p.substratos[s.sub[i] as usize].passagem);
    let cov = s.cov[i].map(|c| p.coberturas[c as usize].passagem);
    for pass in [sub, cov].into_iter().flatten() {
        match pass {
            Passagem::Bloqueia => return -1,
            Passagem::Custo(n) => custo *= n,
            Passagem::Livre => {}
        }
    }
    custo
}

/// Posição inicial: a célula transitável mais perto do centro do mapa.
pub fn posicao_inicial(s: &Estado, p: &Pacote) -> (i64, i64) {
    let (cx, cy) = (s.l / 2, s.a / 2);
    for r in 0..s.l.max(s.a) {
        for dy in -r..=r {
            for dx in -r..=r {
                if dx.abs().max(dy.abs()) != r {
                    continue;
                }
                if custo_passagem(s, p, cx + dx, cy + dy) == 1 {
                    return (cx + dx, cy + dy);
                }
            }
        }
    }
    (0, 0)
}

/// A ordem pode ser feita de (x, y)?
pub fn alcanca(p: &Pacote, x: i64, y: i64, o: &Ordem) -> bool {
    match o.acao {
        Acao::Ir => x == o.x && y == o.y,
        Acao::Construir if x == o.x && y == o.y => false, // não constrói onde está pisando
        _ => (x - o.x).abs().max((y - o.y).abs()) <= p.jogador.alcance,
    }
}

/// Caminho mais barato (Dijkstra com baldes; vizinhos em ordem fixa =
/// determinístico) até qualquer célula de onde a ordem seja alcançável.
/// Devolve os índices a percorrer (sem a célula atual): vazio se já está lá,
/// `None` se não há caminho.
pub fn caminho(s: &Estado, p: &Pacote, jx: i64, jy: i64, o: &Ordem) -> Option<Vec<i64>> {
    let alvo_bloqueado = o.acao == Acao::Ir && custo_passagem(s, p, o.x, o.y) < 0;
    let ok = |x: i64, y: i64| alcanca(p, x, y, o) || (alvo_bloqueado && (x - o.x).abs().max((y - o.y).abs()) <= 1);
    if ok(jx, jy) {
        return Some(Vec::new());
    }
    let n = (s.l * s.a) as usize;
    let mut dist = vec![i64::MAX; n];
    let mut de = vec![usize::MAX; n];
    let ini = s.idx(jx, jy);
    dist[ini] = 0;
    let mut baldes: Vec<Vec<usize>> = vec![vec![ini]];
    let mut d = 0;
    while d < baldes.len() {
        let mut k = 0;
        while k < baldes[d].len() {
            let i = baldes[d][k];
            k += 1;
            if dist[i] != d as i64 {
                continue;
            }
            let (x, y) = (i as i64 % s.l, i as i64 / s.l);
            if i != ini && ok(x, y) {
                let mut rota = Vec::new();
                let mut c = i;
                while c != ini {
                    rota.push(c as i64);
                    c = de[c];
                }
                rota.reverse();
                return Some(rota);
            }
            for (dx, dy) in VIZINHOS {
                let c = custo_passagem(s, p, x + dx, y + dy);
                if c < 0 {
                    continue;
                }
                let j = s.idx(x + dx, y + dy);
                let nd = d as i64 + c;
                if nd < dist[j] {
                    dist[j] = nd;
                    de[j] = i;
                    if baldes.len() <= nd as usize {
                        baldes.resize(nd as usize + 1, Vec::new());
                    }
                    baldes[nd as usize].push(j);
                }
            }
        }
        d += 1;
    }
    None
}

// ---------- ações: validar (sem olhar o alcance) e executar

/// O que a ação rendeu ou gastou (para os textos "+1 madeira").
#[derive(Default)]
struct Feito {
    ganho: Option<Itens>,
    perda: Option<Itens>,
}

fn item_de(p: &Pacote, o: &Ordem) -> Option<Id> {
    o.item.as_deref().and_then(|i| p.item(i))
}

fn lista_itens(p: &Pacote, itens: &[(Id, i64)]) -> String {
    itens.iter().map(|(it, n)| format!("{n} {}", minusc(&p.itens[*it as usize].nome))).collect::<Vec<_>>().join(", ")
}

fn validar(s: &Estado, p: &Pacote, o: &Ordem) -> Option<String> {
    let i = s.idx(o.x, o.y);
    match o.acao {
        Acao::Ir => None,
        Acao::Colher => match s.cov[i] {
            Some(c) if p.coberturas[c as usize].colheita.is_some() => None,
            _ => Some("Não há o que colher aqui.".into()),
        },
        Acao::Minerar => {
            if s.dep[i].is_some() && s.occ[i] == 0 {
                None
            } else if s.occ[i] != 0 {
                Some("Tem uma máquina em cima.".into())
            } else {
                Some("Não há depósito aqui.".into())
            }
        }
        Acao::Plantar => {
            let it = item_de(p, o).map(|k| &p.itens[k as usize]);
            let Some(it) = it.filter(|it| it.planta.is_some()) else {
                return Some("Isso não se planta.".into());
            };
            if s.inv.get(item_de(p, o)?) == 0 {
                return Some(format!("Você não tem {}.", minusc(&it.nome)));
            }
            if s.occ[i] != 0 {
                return Some("Tem uma máquina aqui.".into());
            }
            let sub = &p.substratos[s.sub[i] as usize];
            if !sub.plantar {
                return Some(format!("Não dá para plantar em {}.", minusc(&sub.nome)));
            }
            match s.cov[i] {
                Some(c) if !p.coberturas[c as usize].substituivel => Some("Já tem algo crescendo aqui.".into()),
                _ => None,
            }
        }
        Acao::Construir => {
            let el = o.elemento.as_deref().and_then(|e| p.elemento(e));
            let Some(el) = el.filter(|e| s.liberados.contains(e)) else {
                return Some("Ainda não pesquisado.".into());
            };
            let pr = &p.elementos[el as usize];
            if s.occ[i] != 0 {
                return Some("Já tem uma máquina aqui.".into());
            }
            let sub = &p.substratos[s.sub[i] as usize];
            if !sub.construir {
                return Some(format!("Não dá para construir em {}.", minusc(&sub.nome)));
            }
            if let Some(c) = s.cov[i] {
                let c = &p.coberturas[c as usize];
                if !c.substituivel {
                    return Some(format!("Primeiro tire {} daqui.", minusc(&c.nome)));
                }
            }
            if pr.exige_deposito && s.dep[i].is_none() {
                return Some(format!("{} precisa ficar em cima de um depósito.", pr.nome));
            }
            if !tem_itens(s, &pr.custo) {
                return Some(format!("Faltam {}.", faltam(s, p, &pr.custo)));
            }
            None
        }
        Acao::Desmontar => s.ent_em(o.x, o.y).is_none().then(|| "Não há máquina aqui.".into()),
        Acao::Recolher => match s.ent_em(o.x, o.y) {
            None => Some("Não há máquina aqui.".into()),
            Some(k) if s.ents[k].sai.total() != 0 => None,
            Some(_) => Some("Nada para recolher.".into()),
        },
        Acao::Abastecer | Acao::Fornecer => {
            let Some(k) = s.ent_em(o.x, o.y) else {
                return Some("Não há máquina aqui.".into());
            };
            let nome = o.item.as_deref().map(|i| p.nome_item(i)).unwrap_or_else(|| "undefined".into());
            let it = item_de(p, o).filter(|it| s.inv.get(*it) != 0);
            let Some(it) = it else {
                return Some(format!("Você não tem {}.", minusc(&nome)));
            };
            let e = &s.ents[k];
            if aceita(p, e, it) != Some(slot_de(o.acao)) {
                return Some(format!("{} não aceita {} agora.", p.elementos[e.p as usize].nome, minusc(&nome)));
            }
            None
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Slot {
    Ent,
    Comb,
    Sai,
}

fn slot_de(a: Acao) -> Slot {
    if a == Acao::Abastecer {
        Slot::Comb
    } else {
        Slot::Ent
    }
}

pub fn bolsa_mut(e: &mut Ent, slot: Slot) -> &mut Bolsa {
    match slot {
        Slot::Ent => &mut e.ent,
        Slot::Comb => &mut e.comb,
        Slot::Sai => &mut e.sai,
    }
}

/// Em que compartimento o elemento aceita receber este item?
pub fn aceita(p: &Pacote, e: &Ent, item: Id) -> Option<Slot> {
    let pr = &p.elementos[e.p as usize];
    let menor = |total: i64, cap: Option<i64>| cap.is_some_and(|c| total < c);
    if let Some(procs) = &pr.processo {
        let usa = procs.iter().any(|r| p.receitas[*r as usize].entra.iter().any(|(it, n)| *it == item && *n != 0));
        if usa && menor(e.ent.total(), pr.inv.entrada) {
            return Some(Slot::Ent);
        }
    }
    if pr.consumo.is_some() && p.itens[item as usize].combustivel != 0 && menor(e.comb.total(), pr.inv.combustivel) {
        return Some(Slot::Comb);
    }
    if pr.inv.guarda.is_some_and(|g| g != 0) && menor(e.sai.total(), pr.inv.guarda) {
        return Some(Slot::Sai);
    }
    None
}

fn executar(s: &mut Estado, p: &Pacote, o: &Ordem) -> Feito {
    let i = s.idx(o.x, o.y);
    match o.acao {
        Acao::Ir => Feito::default(),
        Acao::Colher => {
            let col = p.coberturas[s.cov[i].expect("validado") as usize].colheita.as_ref().expect("validado");
            for (it, n) in &col.da {
                s.inv.por(*it, *n);
            }
            s.cov[i] = col.vira;
            s.cres[i] = 0;
            Feito { ganho: Some(col.da.clone()), perda: None }
        }
        Acao::Minerar => {
            let it = p.depositos[s.dep[i].expect("validado") as usize].item;
            s.inv.por(it, 1);
            s.prod.por(it, 1);
            s.dep_qtd[i] -= 1;
            if s.dep_qtd[i] <= 0 {
                s.dep[i] = None;
                s.dep_qtd[i] = 0;
            }
            Feito { ganho: Some(vec![(it, 1)]), perda: None }
        }
        Acao::Plantar => {
            let it = item_de(p, o).expect("validado");
            s.inv.por(it, -1);
            s.cov[i] = p.itens[it as usize].planta;
            s.cres[i] = 0;
            Feito::default()
        }
        Acao::Construir => {
            let el = o.elemento.as_deref().and_then(|e| p.elemento(e)).expect("validado");
            let pr = &p.elementos[el as usize];
            for (it, n) in &pr.custo {
                s.inv.por(*it, -n);
            }
            let e = Ent {
                id: s.prox,
                p: el,
                x: o.x,
                y: o.y,
                ent: Bolsa::default(),
                sai: Bolsa::default(),
                comb: Bolsa::default(),
                queima: 0,
                prog: 0,
                rec: None,
                ativo: 0,
                parado: String::new(),
            };
            s.prox += 1;
            s.occ[i] = e.id;
            s.ents.push(e);
            if s.cov[i].is_some() {
                s.cov[i] = None;
            }
            s.registrar(format!("Construiu {} em ({},{}).", minusc(&pr.nome), o.x, o.y), "");
            Feito::default()
        }
        Acao::Desmontar => {
            let k = s.ent_em(o.x, o.y).expect("validado");
            let e = s.ents.remove(k);
            let pr = &p.elementos[e.p as usize];
            let mut ganho = Bolsa::default();
            for (it, n) in pr.custo.iter().chain(&e.ent.0).chain(&e.sai.0).chain(&e.comb.0) {
                s.inv.por(*it, *n);
                ganho.por(*it, *n);
            }
            s.occ[i] = 0;
            s.registrar(format!("Desmontou {} em ({},{}).", minusc(&pr.nome), o.x, o.y), "");
            Feito { ganho: Some(ganho.0), perda: None }
        }
        Acao::Recolher => {
            let k = s.ent_em(o.x, o.y).expect("validado");
            let sai = std::mem::take(&mut s.ents[k].sai);
            for (it, n) in &sai.0 {
                s.inv.por(*it, *n);
            }
            Feito { ganho: Some(sai.0), perda: None }
        }
        Acao::Abastecer | Acao::Fornecer => {
            let k = s.ent_em(o.x, o.y).expect("validado");
            let it = item_de(p, o).expect("validado");
            let slot = slot_de(o.acao);
            let pr = &p.elementos[s.ents[k].p as usize];
            let cap = if slot == Slot::Comb { pr.inv.combustivel } else { pr.inv.entrada }.unwrap_or(0);
            let n = s.inv.get(it).min(cap - bolsa_mut(&mut s.ents[k], slot).total());
            s.inv.por(it, -n);
            bolsa_mut(&mut s.ents[k], slot).por(it, n);
            Feito { ganho: None, perda: Some(vec![(it, n)]) }
        }
    }
}

fn itens_json(p: &Pacote, itens: &[(Id, i64)]) -> Value {
    let mut m = Map::new();
    for (it, n) in itens {
        m.insert(p.itens[*it as usize].id.clone(), (*n).into());
    }
    Value::Object(m)
}

// ---------- o que dá para fazer numa célula, já com rótulos (para o menu do tabuleiro)
pub fn acoes_possiveis(s: &Estado, p: &Pacote, x: i64, y: i64) -> Value {
    if !s.dentro(x, y) {
        return json!({ "acoes": [], "notas": [] });
    }
    let i = s.idx(x, y);
    let mut acoes = Vec::new();
    let mut notas = Vec::new();
    let nota = |tipo: &str, txt: String| json!({ "tipo": tipo, "txt": txt });
    let mut tenta = |o: Ordem, rotulo: String, detalhe: String| {
        if validar(s, p, &o).is_none() {
            let mut v = o.para_json();
            let m = v.as_object_mut().expect("objeto");
            m.insert("rotulo".into(), rotulo.into());
            m.insert("detalhe".into(), detalhe.into());
            acoes.push(v);
        }
    };
    let ordem = |acao: Acao| Ordem { x, y, acao, item: None, elemento: None, vezes: None, feitas: None };
    let item_ordem = |acao: Acao, it: Id| Ordem { item: Some(p.itens[it as usize].id.clone()), ..ordem(acao) };
    let nome_it = |it: Id| minusc(&p.itens[it as usize].nome);
    let titulo;
    if let Some(k) = s.ent_em(x, y) {
        let e = &s.ents[k];
        let pr = &p.elementos[e.p as usize];
        if e.sai.total() != 0 {
            tenta(ordem(Acao::Recolher), format!("Recolher {}", lista_itens(p, &e.sai.0)), String::new());
        }
        if pr.consumo.is_some() {
            let livre = pr.inv.combustivel.unwrap_or(0) - e.comb.total();
            // o combustível mais forte que o jogador tem (empate: o primeiro do pacote)
            let mut comb: Option<Id> = None;
            for (k, it) in p.itens.iter().enumerate() {
                if it.combustivel != 0
                    && s.inv.get(k as Id) != 0
                    && comb.is_none_or(|c| it.combustivel > p.itens[c as usize].combustivel)
                {
                    comb = Some(k as Id);
                }
            }
            if e.comb.total() == 0 && e.queima == 0 {
                notas.push(nota("ruim", "Sem combustível".into()));
            } else if livre > 0 {
                notas.push(nota("", format!("Cabe mais {livre} de combustível")));
            }
            if let Some(c) = comb.filter(|_| livre > 0) {
                tenta(
                    item_ordem(Acao::Abastecer, c),
                    format!("Abastecer com {} {}", livre.min(s.inv.get(c)), nome_it(c)),
                    String::new(),
                );
            }
        }
        if let Some(procs) = &pr.processo {
            let livre = pr.inv.entrada.unwrap_or(0) - e.ent.total();
            // o que já está na máquina; senão, o insumo que o jogador tem em maior quantidade
            let usa =
                |it: Id| procs.iter().any(|r| p.receitas[*r as usize].entra.iter().any(|(x, n)| *x == it && *n != 0));
            let mut ent = e.ent.primeiro().filter(|it| s.inv.get(*it) != 0);
            if ent.is_none() {
                for k in 0..p.itens.len() as Id {
                    if s.inv.get(k) != 0 && usa(k) && ent.is_none_or(|c| s.inv.get(k) > s.inv.get(c)) {
                        ent = Some(k);
                    }
                }
            }
            if e.ent.total() == 0 && e.rec.is_none() {
                notas.push(nota("ruim", "Sem entrada".into()));
            }
            if let Some(it) = ent.filter(|_| livre > 0) {
                tenta(
                    item_ordem(Acao::Fornecer, it),
                    format!("Fornecer {} {}", livre.min(s.inv.get(it)), nome_it(it)),
                    String::new(),
                );
            }
        }
        if !e.parado.is_empty() && e.parado != "Sem combustível" && e.parado != "Sem entrada" {
            notas.push(nota("ruim", e.parado.clone()));
        }
        tenta(ordem(Acao::Desmontar), format!("Desmontar {}", minusc(&pr.nome)), "devolve o custo e o conteúdo".into());
        titulo = pr.nome.clone();
    } else {
        let c = s.cov[i];
        let d = s.dep[i];
        if let Some(cv) = c.map(|c| &p.coberturas[c as usize]) {
            if let Some(col) = &cv.colheita {
                let verbo = col.verbo.clone().unwrap_or_else(|| "Colher".into());
                tenta(
                    ordem(Acao::Colher),
                    format!("{verbo} {}", minusc(&cv.nome)),
                    format!("+ {}", lista_itens(p, &col.da)),
                );
            }
        }
        if let Some(d) = d {
            let vezes = p.jogador.minerar_vezes.min(s.dep_qtd[i]);
            let o = Ordem { vezes: Some(vezes), ..ordem(Acao::Minerar) };
            tenta(
                o,
                format!("Minerar {} ×{vezes}", minusc(&p.depositos[d as usize].nome)),
                format!("restam {}", s.dep_qtd[i]),
            );
        }
        let plantavel = (0..p.itens.len() as Id).find(|k| p.itens[*k as usize].planta.is_some() && s.inv.get(*k) != 0);
        if let Some(it) = plantavel {
            tenta(
                item_ordem(Acao::Plantar, it),
                format!("Plantar {}", nome_it(it)),
                format!("você tem {}", s.inv.get(it)),
            );
        }
        for (k, el) in p.elementos.iter().enumerate() {
            if s.liberados.contains(&(k as Id)) {
                let o = Ordem { elemento: Some(el.id.clone()), ..ordem(Acao::Construir) };
                tenta(o, format!("Construir {}", minusc(&el.nome)), lista_itens(p, &el.custo));
            }
        }
        if custo_passagem(s, p, x, y) > 0 {
            tenta(ordem(Acao::Ir), "Ir até aqui".into(), String::new());
        }
        titulo = match (c, d) {
            (Some(c), _) => p.coberturas[c as usize].nome.clone(),
            (None, Some(d)) => p.depositos[d as usize].nome.clone(),
            _ => p.substratos[s.sub[i] as usize].nome.clone(),
        };
    }
    json!({ "acoes": acoes, "notas": notas, "titulo": titulo })
}

// ---------- um tick do jogador
pub fn passo_jogador(s: &mut Estado, p: &Pacote) {
    let Some(o) = s.jog.fila.first().cloned() else { return };
    if !alcanca(p, s.jog.x, s.jog.y, &o) {
        s.jog.tarefa = 0;
        let mut prox = s.jog.cam.first().copied();
        let custo_de = |s: &Estado, c: i64| custo_passagem(s, p, c % s.l, c / s.l);
        let mut custo = prox.map_or(-1, |c| custo_de(s, c));
        if custo < 0 {
            // sem caminho ainda, ou o caminho foi bloqueado: calcula de novo
            match caminho(s, p, s.jog.x, s.jog.y, &o) {
                Some(rota) if !rota.is_empty() => {
                    prox = Some(rota[0]);
                    custo = custo_de(s, rota[0]);
                    s.jog.cam = rota;
                    s.jog.passo = 0;
                }
                _ => return falhar(s, "Não há caminho até lá.".into()),
            }
        }
        let prox = prox.expect("há um próximo passo");
        s.jog.passo += 1;
        if s.jog.passo >= p.jogador.ticks_por_celula * custo {
            s.jog.passo = 0;
            s.jog.cam.remove(0);
            s.jog.x = prox % s.l;
            s.jog.y = prox / s.l;
        }
        return;
    }
    s.jog.cam.clear();
    s.jog.passo = 0;
    if let Some(erro) = validar(s, p, &o) {
        return falhar(s, erro);
    }
    s.jog.tarefa += 1;
    if s.jog.tarefa < p.duracao(o.acao.nome()) {
        return;
    }
    s.jog.tarefa = 0;
    let r = executar(s, p, &o);
    if r.ganho.is_some() || r.perda.is_some() {
        let ev = json!({
            "tipo": "ganho", "x": s.jog.x, "y": s.jog.y,
            "ganho": itens_json(p, r.ganho.as_deref().unwrap_or(&[])),
            "perda": itens_json(p, r.perda.as_deref().unwrap_or(&[])),
        });
        registrar_evento(s, ev);
    }
    let feitas = o.feitas.unwrap_or(0) + 1;
    s.jog.fila[0].feitas = Some(feitas);
    if feitas >= o.vezes.filter(|v| *v != 0).unwrap_or(1) {
        s.jog.fila.remove(0);
    }
}

fn limpar_fila(s: &mut Estado) {
    s.jog.fila.clear();
    s.jog.cam.clear();
    s.jog.passo = 0;
    s.jog.tarefa = 0;
}

fn falhar(s: &mut Estado, msg: String) {
    s.jog.fila.remove(0);
    s.jog.cam.clear();
    s.jog.passo = 0;
    s.jog.tarefa = 0;
    registrar_evento(s, json!({ "tipo": "aviso", "msg": msg }));
}

/// Resposta de um comando: deu certo? e a mensagem para o jogador, se houver.
pub fn resposta(ok: bool, msg: Option<String>) -> Value {
    match msg {
        Some(m) => json!({ "ok": ok, "msg": m }),
        None => json!({ "ok": ok }),
    }
}

fn inteiro(v: Option<&Value>) -> Option<i64> {
    num(v).filter(|f| f.fract() == 0.0 && f.abs() < 9e15).map(|f| f as i64)
}

/// Comando "ordem": dá uma ordem ao jogador. Com `adicionar`, entra no fim da
/// fila; sem, substitui a fila.
pub fn ordem(s: &mut Estado, p: &Pacote, cmd: &Value) -> Value {
    let acao = cmd.get("acao").and_then(Value::as_str).and_then(Acao::de);
    let (x, y) = (inteiro(cmd.get("x")), inteiro(cmd.get("y")));
    let (Some(acao), Some(x), Some(y)) = (acao, x, y) else { return resposta(false, None) };
    if !s.dentro(x, y) {
        return resposta(false, None);
    }
    let vezes = num(cmd.get("vezes")).filter(|v| *v > 1.0).map(|v| (to_int32(v) as i64).min(99));
    let o = Ordem {
        x,
        y,
        acao,
        item: texto(cmd.get("item")).map(str::to_string),
        elemento: texto(cmd.get("elemento")).map(str::to_string),
        vezes,
        feitas: None,
    };
    if let Some(erro) = validar(s, p, &o) {
        return resposta(false, Some(erro));
    }
    if acao == Acao::Ir && custo_passagem(s, p, x, y) < 0 {
        return resposta(false, Some("Não dá para pisar aí.".into()));
    }
    if truthy(cmd.get("adicionar")) {
        if s.jog.fila.len() >= 20 {
            return resposta(false, Some("A fila de tarefas está cheia.".into()));
        }
        s.jog.fila.push(o);
    } else {
        limpar_fila(s);
        s.jog.fila.push(o);
    }
    resposta(true, None)
}

/// Comando "cancelar": esvazia a fila de tarefas.
pub fn cancelar(s: &mut Estado) -> Value {
    if s.jog.fila.is_empty() {
        return resposta(false, None);
    }
    limpar_fila(s);
    resposta(true, Some("Tarefas canceladas.".into()))
}
