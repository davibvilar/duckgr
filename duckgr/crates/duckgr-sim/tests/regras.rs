//! Regras da Terra Viva reproduzidas a partir de sequências de comandos, e a
//! compatibilidade com os saves gravados pelo núcleo em JavaScript.

use duckgr_sim::estado::Acao;
use duckgr_sim::{aplicar, avancar, compilar, contar_marca, novo_estado, passo, rodar, Estado, Pacote, TPS};
use serde_json::{json, Value};

fn dados(nome: &str) -> Value {
    let caminho = format!("{}/../../testes/dados/{nome}", env!("CARGO_MANIFEST_DIR"));
    let txt = std::fs::read_to_string(&caminho).unwrap_or_else(|e| panic!("{caminho}: {e} (rode `npm run pacotes`)"));
    serde_json::from_str(&txt).expect("JSON válido")
}

fn terra_viva() -> Pacote {
    compilar(&dados("terra-viva.json")).expect("o pacote compila").0
}

/// Um jogador de teste: aplica comandos guardando o registro, como a interface faz.
struct Partida {
    p: Pacote,
    s: Estado,
    cfg: Value,
    cmds: Vec<Value>,
}

impl Partida {
    fn nova(cfg: Value) -> Partida {
        let p = terra_viva();
        let s = novo_estado(&p, &cfg).expect("estado inicial");
        Partida { p, s, cfg, cmds: Vec::new() }
    }
    fn cmd(&mut self, mut c: Value) -> Value {
        c["t"] = self.s.tick.into();
        c["autor"] = "teste".into();
        let r = aplicar(&mut self.s, &self.p, &c);
        if r["ok"] == true {
            self.cmds.push(c);
        }
        r
    }
    fn segundos(&mut self, s: i64) {
        for _ in 0..s * TPS {
            passo(&mut self.s, &self.p);
        }
    }
    /// Dá uma ordem e roda o mundo até a fila esvaziar; true se deu tudo certo.
    fn fazer(&mut self, ordem: Value) -> bool {
        self.s.eventos.clear();
        let mut c = json!({ "tipo": "ordem" });
        for (k, v) in ordem.as_object().unwrap() {
            c[k] = v.clone();
        }
        if self.cmd(c)["ok"] != true {
            return false;
        }
        for _ in 0..120 * TPS {
            if self.s.jog.fila.is_empty() {
                break;
            }
            passo(&mut self.s, &self.p);
        }
        self.s.jog.fila.is_empty() && !self.s.eventos.iter().any(|e| e["tipo"] == "aviso")
    }
    fn achar(&self, teste: impl Fn(usize, i64, i64) -> bool) -> Option<(i64, i64)> {
        for y in 0..self.s.a {
            for x in 0..self.s.l {
                if teste(self.s.idx(x, y), x, y) {
                    return Some((x, y));
                }
            }
        }
        None
    }
    fn cov(&self, i: usize) -> Option<&str> {
        self.s.cov[i].map(|c| self.p.coberturas[c as usize].id.as_str())
    }
    fn item(&self, nome: &str) -> i64 {
        self.s.inv.get(self.p.item(nome).unwrap())
    }
    fn hash(&self) -> String {
        self.s.hash(&self.p)
    }
    /// Depósito de um tipo com uma célula livre à direita (para encostar uma máquina).
    fn deposito_com_vizinho_livre(&self, tipo: &str) -> (i64, i64) {
        let d = self.p.deposito(tipo).unwrap();
        let solo = self.p.substrato("solo").unwrap();
        self.achar(|i, x, _| {
            self.s.dep[i] == Some(d)
                && x + 1 < self.s.l
                && self.s.dep[i + 1].is_none()
                && self.s.sub[i + 1] == solo
                && self.s.cov[i + 1].is_none_or(|c| self.p.coberturas[c as usize].substituivel)
        })
        .expect("há depósito com vizinho livre")
    }
}

#[test]
fn o_pacote_compila_e_converte_segundos_em_ticks() {
    let p = terra_viva();
    assert_eq!(p.id, "terra-viva");
    assert_eq!(p.receitas[p.receita("placa-ferro").unwrap() as usize].ticks, 96);
    // 3,2 s × 30 ticks
}

#[test]
fn pacote_com_referencia_quebrada_e_recusado() {
    let mut v = dados("terra-viva.json");
    v["receitas"]["placa-ferro"]["sai"] = json!({ "placa-de-ouro": 1 });
    let erro = compilar(&v).unwrap_err();
    assert!(erro.contains("placa-de-ouro"), "{erro}");
}

#[test]
fn a_mesma_semente_gera_o_mesmo_mapa() {
    let p = terra_viva();
    let h = |s: i64| novo_estado(&p, &json!({ "semente": s })).unwrap().hash(&p);
    assert_eq!(h(7), h(7));
    assert_ne!(h(7), h(8));
}

#[test]
fn o_mapa_padrao_tem_agua_floresta_e_depositos() {
    let pt = Partida::nova(json!({}));
    let agua = pt.p.substrato("agua").unwrap();
    assert!(pt.s.sub.contains(&agua));
    assert!(contar_marca(&pt.s, &pt.p, "floresta") > 100);
    for d in ["ferro", "cobre", "carvao", "pedra"] {
        assert!(pt.s.dep.contains(&pt.p.deposito(d)), "falta {d}");
    }
}

#[test]
fn determinismo_refazer_pelos_comandos_da_o_mesmo_hash() {
    let mut pt = Partida::nova(json!({}));
    let (x, y) = pt.achar(|i, _, _| pt.cov(i) == Some("adulta")).unwrap();
    pt.cmd(json!({ "tipo": "ordem", "x": x, "y": y, "acao": "colher" }));
    pt.segundos(20);
    let (x, y) = pt.achar(|i, _, _| pt.cov(i) == Some("grama") && pt.s.dep[i].is_none()).unwrap();
    pt.cmd(json!({ "tipo": "ordem", "x": x, "y": y, "acao": "plantar", "item": "muda", "adicionar": true }));
    pt.cmd(json!({ "tipo": "ordem", "x": 0, "y": 0, "acao": "ir", "adicionar": true }));
    pt.segundos(40);
    let refeito = rodar(&pt.p, &pt.cfg, &pt.cmds, pt.s.tick).unwrap();
    assert_eq!(refeito.hash(&pt.p), pt.hash());
}

#[test]
fn comando_que_falha_nao_muda_o_mundo() {
    let mut pt = Partida::nova(json!({}));
    let antes = pt.hash();
    let agua = pt.p.substrato("agua").unwrap();
    let (x, y) = pt.achar(|i, _, _| pt.s.sub[i] == agua).unwrap();
    let r = pt.cmd(json!({ "tipo": "ordem", "x": x, "y": y, "acao": "construir", "elemento": "fornalha" }));
    assert_eq!(r["ok"], false);
    assert_eq!(pt.hash(), antes);
}

#[test]
fn cortar_arvore_da_madeira_e_muda_e_deixa_toco() {
    let mut pt = Partida::nova(json!({}));
    let (x, y) = pt.achar(|i, _, _| pt.cov(i) == Some("adulta")).unwrap();
    let madeira = pt.item("madeira");
    assert!(pt.fazer(json!({ "x": x, "y": y, "acao": "colher" })));
    assert_eq!(pt.item("madeira"), madeira + 4);
    assert_eq!(pt.item("muda"), 5);
    assert_eq!(pt.cov(pt.s.idx(x, y)), Some("toco"));
}

#[test]
fn o_jogador_anda_ate_o_alvo_antes_de_agir() {
    let mut pt = Partida::nova(json!({}));
    let (jx, jy) = (pt.s.jog.x, pt.s.jog.y);
    let (x, y) = pt.achar(|i, x, y| pt.cov(i) == Some("adulta") && (x - jx).abs().max((y - jy).abs()) > 3).unwrap();
    pt.cmd(json!({ "tipo": "ordem", "x": x, "y": y, "acao": "colher" }));
    assert_eq!(pt.s.jog.fila[0].acao, Acao::Colher);
    pt.segundos(30);
    assert!((pt.s.jog.x - x).abs().max((pt.s.jog.y - y).abs()) <= pt.p.jogador.alcance);
    assert_eq!(pt.cov(pt.s.idx(x, y)), Some("toco"));
}

#[test]
fn mineradora_encostada_na_fornalha_produz_placas_sozinha() {
    let mut pt = Partida::nova(json!({}));
    let (x, y) = pt.deposito_com_vizinho_livre("ferro");
    assert!(pt.fazer(json!({ "x": x, "y": y, "acao": "construir", "elemento": "mineradora" })));
    assert!(pt.fazer(json!({ "x": x + 1, "y": y, "acao": "construir", "elemento": "fornalha" })));
    assert!(pt.fazer(json!({ "x": x, "y": y, "acao": "abastecer", "item": "carvao" })));
    assert!(pt.fazer(json!({ "x": x + 1, "y": y, "acao": "abastecer", "item": "madeira" })));
    pt.segundos(30);
    let placas = pt.s.prod.get(pt.p.item("placa-ferro").unwrap());
    assert!(placas >= 5, "produziu {placas}");
}

#[test]
fn maquinas_poluem_e_a_floresta_adoece() {
    let mut pt = Partida::nova(json!({ "recursosIniciais": 20 }));
    let minerio = pt.p.item("minerio-ferro").unwrap();
    pt.s.inv.por(minerio, 400);
    let mut n = 0;
    'fora: for y in 8..16 {
        for x in 16..24 {
            if n >= 10 {
                break 'fora;
            }
            if !pt.fazer(json!({ "x": x, "y": y, "acao": "construir", "elemento": "fornalha" })) {
                continue;
            }
            pt.fazer(json!({ "x": x, "y": y, "acao": "fornecer", "item": "minerio-ferro" }));
            pt.fazer(json!({ "x": x, "y": y, "acao": "abastecer", "item": "carvao" }));
            n += 1;
        }
    }
    assert!(n >= 6, "construiu {n} fornalhas");
    let floresta = contar_marca(&pt.s, &pt.p, "floresta");
    let mut pico = 0;
    for _ in 0..40 {
        pt.segundos(1);
        pico = pico.max(pt.s.campos[0].iter().copied().max().unwrap());
    }
    assert!(pico > 0, "o trecho ficou poluído");
    assert!(contar_marca(&pt.s, &pt.p, "floresta") < floresta, "árvores adoeceram");
}

#[test]
fn pesquisa_libera_o_viveiro_e_a_eletricidade_abre_a_era_2() {
    let mut pt = Partida::nova(json!({}));
    pt.s.inv.por(pt.p.item("ciencia").unwrap(), 25);
    assert_eq!(pt.cmd(json!({ "tipo": "pesquisar", "tecnologia": "eletricidade" }))["ok"], false);
    assert_eq!(pt.cmd(json!({ "tipo": "pesquisar", "tecnologia": "viveiro" }))["ok"], true);
    assert!(pt.s.liberados.contains(&pt.p.elemento("viveiro").unwrap()));
    assert_eq!(pt.cmd(json!({ "tipo": "pesquisar", "tecnologia": "eletricidade" }))["ok"], true);
    assert_eq!(pt.s.era, 2);
}

#[test]
fn o_estado_vai_e_volta_em_json_sem_perder_nada() {
    let mut pt = Partida::nova(json!({}));
    let (x, y) = pt.deposito_com_vizinho_livre("ferro");
    pt.fazer(json!({ "x": x, "y": y, "acao": "construir", "elemento": "mineradora" }));
    pt.fazer(json!({ "x": x, "y": y, "acao": "abastecer", "item": "carvao" }));
    pt.segundos(10);
    let json = pt.s.para_json(&pt.p);
    let mut volta = Estado::de_json(&json, &pt.p).unwrap();
    assert_eq!(volta.hash(&pt.p), pt.hash());
    for _ in 0..300 {
        passo(&mut volta, &pt.p);
        passo(&mut pt.s, &pt.p);
    }
    assert_eq!(volta.hash(&pt.p), pt.hash());
}

/// Um save gravado pelo núcleo em JavaScript (v0.4), com uma partida de 13
/// minutos: máquinas, poluição que adoece árvores, pesquisa e missão cumprida.
#[test]
fn save_gravado_pelo_nucleo_em_javascript_abre_com_o_mesmo_hash() {
    let p = terra_viva();
    let v = dados("save-v0.4.json");
    let save = &v["save"];
    let historico = save["historico"].as_array().unwrap();
    // pelo checkpoint, refazendo só os comandos posteriores
    let cp = &save["checkpoint"];
    let mut s = Estado::de_json(&cp["estado"], &p).unwrap();
    let depois: Vec<Value> = historico.iter().filter(|c| c["t"].as_i64() >= cp["tick"].as_i64()).cloned().collect();
    avancar(&mut s, &p, &depois, save["tick"].as_i64().unwrap());
    assert_eq!(s.hash(&p), save["hash"].as_str().unwrap());
    // do zero, conferindo o hash em vários momentos da partida
    let mut z = novo_estado(&p, &save["cfg"]).unwrap();
    for m in v["marcos"].as_array().unwrap() {
        let t = m["tick"].as_i64().unwrap();
        let antes: Vec<Value> = historico.iter().filter(|c| c["t"].as_i64().unwrap() < t).cloned().collect();
        avancar(&mut z, &p, &antes, t);
        assert_eq!(z.hash(&p), m["hash"].as_str().unwrap(), "tick {t}");
    }
}

/// Orçamento de desempenho: até 10 ms por tick no maior mapa da Terra Viva.
#[test]
fn orcamento_de_desempenho() {
    let mut pt = Partida::nova(json!({ "largura": 56, "altura": 32, "recursosIniciais": 20 }));
    for tipo in ["ferro", "cobre", "carvao"] {
        let (x, y) = pt.deposito_com_vizinho_livre(tipo);
        pt.fazer(json!({ "x": x, "y": y, "acao": "construir", "elemento": "mineradora" }));
        pt.fazer(json!({ "x": x + 1, "y": y, "acao": "construir", "elemento": "fornalha" }));
        pt.fazer(json!({ "x": x, "y": y, "acao": "abastecer", "item": "carvao" }));
    }
    let ticks = 60 * TPS;
    let inicio = std::time::Instant::now();
    pt.segundos(60);
    let ms = inicio.elapsed().as_secs_f64() * 1000.0 / ticks as f64;
    println!("{ms:.4} ms por tick");
    assert!(ms < 10.0, "{ms} ms por tick");
}
