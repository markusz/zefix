//! Name resolution and type checking: AST → IR.

use super::ast::{BinOp, KEYWORDS, MethodNode, Node, NodeKind, QuantKind, QuantNode};
use super::ir::{BoolExpr, BoolQuant, Domain, IntExpr, MAX_VARS, PlayerRef, Quant, SetExpr};
use super::parser::parse;
use super::{LangError, Span};
use crate::cards::{Card, Cards};
use crate::game::{Game, Seat};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum VarKind {
    Player,
    Suit,
}

struct Var {
    name: String,
    kind: VarKind,
}

enum Typed {
    Bool(BoolExpr),
    Int(IntExpr),
    Set(SetExpr),
    Player(PlayerRef),
}

impl Typed {
    fn type_name(&self) -> &'static str {
        match self {
            Typed::Bool(_) => "Wahrheitswert",
            Typed::Int(_) => "Zahl",
            Typed::Set(_) => "Kartenmenge",
            Typed::Player(_) => "Spieler",
        }
    }
}

const GROUPS: [&str; 5] = ["others", "opponents", "players", "farben", "suits"];
const PLAYERS: [&str; 6] = ["me", "left", "across", "right", "partner", "vorhand"];
const METHODS: [&str; 4] = ["count", "has", "has_any", "frei"];

fn parse_domain(name: &str) -> Option<Domain> {
    Some(match name {
        "others" => Domain::Others,
        "opponents" => Domain::Opponents,
        "players" => Domain::Players,
        "farben" => Domain::Farben,
        "suits" => Domain::Suits,
        _ => return None,
    })
}

fn var_kind(domain: Domain) -> VarKind {
    match domain {
        Domain::Others | Domain::Opponents | Domain::Players => VarKind::Player,
        Domain::Farben | Domain::Suits => VarKind::Suit,
    }
}

fn type_error(expected: &str, got: &Typed, span: Span) -> LangError {
    LangError::new(
        format!("erwartet: {expected}, gefunden: {}", got.type_name()),
        span,
    )
}

fn fold_set(
    a: SetExpr,
    b: SetExpr,
    op: fn(Cards, Cards) -> Cards,
    node: fn(Box<SetExpr>, Box<SetExpr>) -> SetExpr,
) -> SetExpr {
    match (a, b) {
        (SetExpr::Const(x), SetExpr::Const(y)) => SetExpr::Const(op(x, y)),
        (a, b) => node(Box::new(a), Box::new(b)),
    }
}

fn union(a: SetExpr, b: SetExpr) -> SetExpr {
    fold_set(a, b, |x, y| x | y, SetExpr::Union)
}

fn call_error(name: &str, span: Span) -> LangError {
    let msg = if METHODS.contains(&name) {
        format!("`{name}` ist eine Methode eines Spielers, z. B. `me.{name}(…)`")
    } else {
        format!("unbekannte Funktion `{name}`")
    };
    LangError::new(msg, span)
}

fn unknown_name(name: &str, span: Span) -> LangError {
    let upper = name.to_uppercase();
    let hint = if Card::parse(&upper).is_some() {
        format!(" (Karten groß schreiben: `{upper}`)")
    } else {
        String::new()
    };
    LangError::new(format!("unbekannter Name `{name}`{hint}"), span)
}

fn typed_quant(kind: QuantKind, quant: Box<Quant>) -> Typed {
    let bool_quant = match kind {
        QuantKind::Num => return Typed::Int(IntExpr::Num(quant)),
        QuantKind::Any => BoolQuant::Any,
        QuantKind::All => BoolQuant::All,
        QuantKind::NoneOf => BoolQuant::NoneOf,
    };
    Typed::Bool(BoolExpr::Quant(bool_quant, quant))
}

type R<T> = Result<T, LangError>;

pub struct Compiler<'g> {
    game: &'g Game,
    vorhand: Option<Seat>,
    vars: Vec<Var>,
}

impl<'g> Compiler<'g> {
    pub fn new(game: &'g Game, vorhand: Option<Seat>) -> Self {
        Compiler {
            game,
            vorhand,
            vars: Vec::new(),
        }
    }

    pub fn compile_bool(&mut self, src: &str) -> R<BoolExpr> {
        let ast = parse(src)?;
        self.bool(&ast)
    }

    /// Card set that does not depend on the deal, e.g. for hand slots.
    pub fn compile_const_set(&mut self, src: &str) -> R<Cards> {
        let ast = parse(src)?;
        match self.set(&ast)? {
            SetExpr::Const(c) => Ok(c),
            _ => Err(LangError::new("feste Kartenmenge erwartet", ast.span)),
        }
    }

    fn bool(&mut self, n: &Node) -> R<BoolExpr> {
        match self.node(n)? {
            Typed::Bool(b) => Ok(b),
            other => Err(type_error("Wahrheitswert", &other, n.span.clone())),
        }
    }

    fn boxed_bool(&mut self, n: &Node) -> R<Box<BoolExpr>> {
        self.bool(n).map(Box::new)
    }

    fn int(&mut self, n: &Node) -> R<IntExpr> {
        match self.node(n)? {
            Typed::Int(i) => Ok(i),
            other => Err(type_error("Zahl", &other, n.span.clone())),
        }
    }

    fn set(&mut self, n: &Node) -> R<SetExpr> {
        match self.node(n)? {
            Typed::Set(s) => Ok(s),
            other => Err(type_error("Kartenmenge", &other, n.span.clone())),
        }
    }

    fn player(&mut self, n: &Node) -> R<PlayerRef> {
        match self.node(n)? {
            Typed::Player(p) => Ok(p),
            other => Err(type_error("Spieler", &other, n.span.clone())),
        }
    }

    fn node(&mut self, n: &Node) -> R<Typed> {
        match &n.kind {
            NodeKind::Int(v) => Ok(Typed::Int(IntExpr::Const(*v))),
            NodeKind::Name(name) => self.name(name, n.span.clone()),
            NodeKind::Not(inner) => Ok(Typed::Bool(BoolExpr::Not(self.boxed_bool(inner)?))),
            NodeKind::Binary { op, lhs, rhs } => self.binary(*op, lhs, rhs),
            NodeKind::Call { name } => Err(call_error(name, n.span.clone())),
            NodeKind::Method(m) => self.method(m, n.span.clone()),
            NodeKind::Quant(q) => self.quant(q),
        }
    }

    fn binary(&mut self, op: BinOp, lhs: &Node, rhs: &Node) -> R<Typed> {
        Ok(match op {
            BinOp::Or => Typed::Bool(BoolExpr::Or(self.boxed_bool(lhs)?, self.boxed_bool(rhs)?)),
            BinOp::And => Typed::Bool(BoolExpr::And(self.boxed_bool(lhs)?, self.boxed_bool(rhs)?)),
            BinOp::Cmp(c) => Typed::Bool(BoolExpr::Cmp(c, self.int(lhs)?, self.int(rhs)?)),
            BinOp::Add => {
                let (a, b) = (self.int(lhs)?, self.int(rhs)?);
                Typed::Int(IntExpr::Add(Box::new(a), Box::new(b)))
            }
            BinOp::Union => Typed::Set(union(self.set(lhs)?, self.set(rhs)?)),
            BinOp::Inter => Typed::Set(fold_set(
                self.set(lhs)?,
                self.set(rhs)?,
                |x, y| x & y,
                SetExpr::Inter,
            )),
            BinOp::Sub => self.minus(lhs, rhs)?,
        })
    }

    /// `-` is subtraction on numbers and set difference on card sets.
    fn minus(&mut self, lhs: &Node, rhs: &Node) -> R<Typed> {
        match (self.node(lhs)?, self.node(rhs)?) {
            (Typed::Int(a), Typed::Int(b)) => {
                Ok(Typed::Int(IntExpr::Sub(Box::new(a), Box::new(b))))
            }
            (Typed::Set(a), Typed::Set(b)) => {
                Ok(Typed::Set(fold_set(a, b, |x, y| x - y, SetExpr::Diff)))
            }
            (a, b) => Err(LangError::new(
                format!(
                    "`-` braucht zwei Zahlen oder zwei Kartenmengen, gefunden: {} und {}",
                    a.type_name(),
                    b.type_name()
                ),
                lhs.span.start..rhs.span.end,
            )),
        }
    }

    fn method(&mut self, m: &MethodNode, span: Span) -> R<Typed> {
        let player = self.player(&m.recv)?;
        match m.name.as_str() {
            "count" => Ok(Typed::Int(IntExpr::Count(
                player,
                self.single_set(m, span)?,
            ))),
            "has" => Ok(Typed::Bool(BoolExpr::HasAll(
                player,
                self.union_of(m, span)?,
            ))),
            "has_any" => Ok(Typed::Bool(BoolExpr::HasAny(
                player,
                self.single_set(m, span)?,
            ))),
            "frei" => {
                let any = BoolExpr::HasAny(player, self.single_set(m, span)?);
                Ok(Typed::Bool(BoolExpr::Not(Box::new(any))))
            }
            _ => Err(LangError::new(
                format!(
                    "unbekannte Methode `{}` (verfügbar: {})",
                    m.name,
                    METHODS.join(", ")
                ),
                m.name_span.clone(),
            )),
        }
    }

    fn single_set(&mut self, m: &MethodNode, span: Span) -> R<SetExpr> {
        match m.args.as_slice() {
            [arg] => self.set(arg),
            args => Err(LangError::new(
                format!(
                    "`{}` erwartet genau ein Argument, gefunden: {}",
                    m.name,
                    args.len()
                ),
                span,
            )),
        }
    }

    fn union_of(&mut self, m: &MethodNode, span: Span) -> R<SetExpr> {
        if m.args.is_empty() {
            return Err(LangError::new(
                format!("`{}` braucht mindestens eine Karte", m.name),
                span,
            ));
        }
        m.args
            .iter()
            .try_fold(SetExpr::Const(Cards::EMPTY), |acc, arg| {
                Ok(union(acc, self.set(arg)?))
            })
    }

    fn quant(&mut self, q: &QuantNode) -> R<Typed> {
        let domain = parse_domain(&q.domain).ok_or_else(|| {
            LangError::new(
                format!(
                    "unbekannte Gruppe `{}` (verfügbar: {})",
                    q.domain,
                    GROUPS.join(", ")
                ),
                q.domain_span.clone(),
            )
        })?;
        self.check_var_name(&q.var, &q.var_span)?;
        let slot = self.vars.len() as u8;
        self.vars.push(Var {
            name: q.var.clone(),
            kind: var_kind(domain),
        });
        let body = self.bool(&q.body);
        self.vars.pop();
        let quant = Box::new(Quant {
            slot,
            domain,
            body: body?,
        });
        Ok(typed_quant(q.kind, quant))
    }

    fn check_var_name(&self, name: &str, span: &Span) -> R<()> {
        let taken = KEYWORDS.contains(&name)
            || GROUPS.contains(&name)
            || PLAYERS.contains(&name)
            || self.card_set(name).is_some()
            || self.vars.iter().any(|v| v.name == name);
        if taken {
            return Err(LangError::new(
                format!("`{name}` ist schon vergeben, wähle einen anderen Variablennamen"),
                span.clone(),
            ));
        }
        if self.vars.len() >= MAX_VARS {
            return Err(LangError::new("zu tief verschachtelt", span.clone()));
        }
        Ok(())
    }

    fn name(&self, name: &str, span: Span) -> R<Typed> {
        if let Some(typed) = self.variable(name) {
            return Ok(typed);
        }
        if let Some(player) = self.player_name(name, &span)? {
            return Ok(Typed::Player(player));
        }
        if GROUPS.contains(&name) {
            return Err(LangError::new(
                format!("`{name}` ist eine Gruppe, nur in Quantoren nutzbar: any(x in {name}, …)"),
                span,
            ));
        }
        self.card_set(name)
            .map(|set| Typed::Set(SetExpr::Const(set)))
            .ok_or_else(|| unknown_name(name, span))
    }

    fn variable(&self, name: &str) -> Option<Typed> {
        let (slot, var) = self
            .vars
            .iter()
            .enumerate()
            .rev()
            .find(|(_, v)| v.name == name)?;
        let slot_id = slot as u8;
        Some(match var.kind {
            VarKind::Player => Typed::Player(PlayerRef::Var(slot_id)),
            VarKind::Suit => Typed::Set(SetExpr::SuitVar(slot_id)),
        })
    }

    fn player_name(&self, name: &str, span: &Span) -> R<Option<PlayerRef>> {
        let seat = match name {
            "me" => Seat::Me,
            "left" => Seat::Left,
            "across" => Seat::Across,
            "right" => Seat::Right,
            "partner" if self.game.called_sau().is_some() => return Ok(Some(PlayerRef::Partner)),
            "partner" => {
                return Err(LangError::new(
                    "`partner` gibt es nur im Sauspiel",
                    span.clone(),
                ));
            }
            "vorhand" => self.vorhand.ok_or_else(|| {
                LangError::new(
                    "`vorhand` ist nicht gesetzt ([table] vorhand = \"…\")",
                    span.clone(),
                )
            })?,
            _ => return Ok(None),
        };
        Ok(Some(PlayerRef::Seat(seat)))
    }

    fn card_set(&self, name: &str) -> Option<Cards> {
        self.game
            .named_set(name)
            .or_else(|| Card::parse(name).map(Card::mask))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::Suit;
    use crate::game::GameKind;

    fn game(kind: GameKind) -> Game {
        Game::new(kind).unwrap_or_else(|e| panic!("{e}"))
    }

    fn sauspiel() -> Game {
        game(GameKind::Sauspiel {
            called: Suit::Schellen,
        })
    }

    fn err(game: &Game, src: &str) -> String {
        Compiler::new(game, None)
            .compile_bool(src)
            .err()
            .map(|e| e.message)
            .unwrap_or_default()
    }

    #[test]
    fn compiles_examples() {
        let g = sauspiel();
        let mut c = Compiler::new(&g, None);
        for src in [
            "any(p in others, p.has(HO, SO) and p.count(trumpf - ober) >= 2)",
            "partner.count(trumpf) >= 3",
            "any(f in farben, me.count(f) == 2)",
            "num(p in opponents, p.frei(eichel)) >= 1 or not me.has_any(sau)",
            "all(p in others, any(f in suits, p.count(f) > me.count(f)))",
        ] {
            if let Err(e) = c.compile_bool(src) {
                panic!("{}", e.render(src));
            }
        }
    }

    #[test]
    fn folds_constant_sets() {
        let g = sauspiel();
        let set = Compiler::new(&g, None).compile_const_set("farbe - sau");
        assert_eq!(set.map(Cards::len), Ok(15));
    }

    #[test]
    fn type_and_name_errors() {
        let g = sauspiel();
        assert!(err(&g, "me.count(trumpf)").contains("Wahrheitswert"));
        assert!(err(&g, "me.count(3) > 1").contains("Kartenmenge"));
        assert!(err(&g, "me.has(eo)").contains("`EO`"));
        assert!(err(&g, "count(trumpf) > 1").contains("me.count"));
        assert!(err(&g, "any(me in others, me.has(EO))").contains("vergeben"));
        assert!(err(&g, "vorhand.has(EO)").contains("table"));
        assert!(err(&g, "others.has(EO)").contains("Gruppe"));
        let solo = game(GameKind::Solo {
            trump: Suit::Eichel,
        });
        assert!(err(&solo, "partner.has(EO)").contains("Sauspiel"));
    }
}
