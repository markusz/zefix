//! Recursive-descent parser.
//!
//! Precedence, lowest first: `or`, `and`, `not`, comparison, `+ - |`, `&`, `.method()`.

use super::ast::{BinOp, CmpOp, KEYWORDS, MethodNode, Node, NodeKind, QuantKind, QuantNode};
use super::lexer::{Tok, Token, lex};
use super::{LangError, Span};

type R = Result<Node, LangError>;

pub fn parse(src: &str) -> R {
    let mut p = Parser {
        toks: lex(src)?,
        pos: 0,
        eof: Token {
            tok: Tok::Eof,
            span: src.len()..src.len(),
        },
    };
    let node = p.expr()?;
    match p.peek() {
        Tok::Eof => Ok(node),
        _ => Err(p.unexpected("Operator oder Ende des Ausdrucks")),
    }
}

struct Parser {
    toks: Vec<Token>,
    pos: usize,
    eof: Token,
}

struct Ident {
    name: String,
    span: Span,
}

/// Call arguments and the end offset of the closing paren.
struct Args {
    nodes: Vec<Node>,
    end: usize,
}

fn binary(op: BinOp, lhs: Node, rhs: Node) -> Node {
    Node {
        span: lhs.span.start..rhs.span.end,
        kind: NodeKind::Binary {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        },
    }
}

impl Parser {
    fn current(&self) -> &Token {
        self.toks.get(self.pos).unwrap_or(&self.eof)
    }

    fn peek(&self) -> &Tok {
        &self.current().tok
    }

    fn bump(&mut self) -> Token {
        let t = self.current().clone();
        if t.tok != Tok::Eof {
            self.pos += 1;
        }
        t
    }

    fn is_kw(&self, kw: &str) -> bool {
        matches!(self.peek(), Tok::Ident(s) if s == kw)
    }

    fn unexpected(&self, expected: &str) -> LangError {
        LangError::new(
            format!("erwartet: {expected}, gefunden: {}", self.peek().describe()),
            self.current().span.clone(),
        )
    }

    fn expect(&mut self, tok: Tok) -> Result<Token, LangError> {
        if *self.peek() == tok {
            Ok(self.bump())
        } else {
            Err(self.unexpected(&tok.describe()))
        }
    }

    fn ident(&mut self, what: &str) -> Result<Ident, LangError> {
        match self.peek().clone() {
            Tok::Ident(name) => Ok(Ident {
                name,
                span: self.bump().span,
            }),
            _ => Err(self.unexpected(what)),
        }
    }

    fn expr(&mut self) -> R {
        self.or()
    }

    fn or(&mut self) -> R {
        let mut lhs = self.and()?;
        while self.is_kw("or") {
            self.bump();
            let rhs = self.and()?;
            lhs = binary(BinOp::Or, lhs, rhs);
        }
        Ok(lhs)
    }

    fn and(&mut self) -> R {
        let mut lhs = self.not()?;
        while self.is_kw("and") {
            self.bump();
            let rhs = self.not()?;
            lhs = binary(BinOp::And, lhs, rhs);
        }
        Ok(lhs)
    }

    fn not(&mut self) -> R {
        if !self.is_kw("not") {
            return self.cmp();
        }
        let start = self.bump().span.start;
        let inner = self.not()?;
        Ok(Node {
            span: start..inner.span.end,
            kind: NodeKind::Not(Box::new(inner)),
        })
    }

    fn cmp_op(&self) -> Option<CmpOp> {
        Some(match self.peek() {
            Tok::Eq => CmpOp::Eq,
            Tok::Ne => CmpOp::Ne,
            Tok::Lt => CmpOp::Lt,
            Tok::Le => CmpOp::Le,
            Tok::Gt => CmpOp::Gt,
            Tok::Ge => CmpOp::Ge,
            _ => return None,
        })
    }

    fn cmp(&mut self) -> R {
        let lhs = self.additive()?;
        let Some(op) = self.cmp_op() else {
            return Ok(lhs);
        };
        self.bump();
        let rhs = self.additive()?;
        if self.cmp_op().is_some() {
            return Err(LangError::new(
                "Vergleiche lassen sich nicht verketten, nutze `and`",
                self.current().span.clone(),
            ));
        }
        Ok(binary(BinOp::Cmp(op), lhs, rhs))
    }

    fn additive(&mut self) -> R {
        let mut lhs = self.inter()?;
        loop {
            let op = match self.peek() {
                Tok::Plus => BinOp::Add,
                Tok::Minus => BinOp::Sub,
                Tok::Pipe => BinOp::Union,
                _ => return Ok(lhs),
            };
            self.bump();
            let rhs = self.inter()?;
            lhs = binary(op, lhs, rhs);
        }
    }

    fn inter(&mut self) -> R {
        let mut lhs = self.postfix()?;
        while *self.peek() == Tok::Amp {
            self.bump();
            let rhs = self.postfix()?;
            lhs = binary(BinOp::Inter, lhs, rhs);
        }
        Ok(lhs)
    }

    fn postfix(&mut self) -> R {
        let mut node = self.primary()?;
        while *self.peek() == Tok::Dot {
            self.bump();
            let method = self.ident("Methodenname")?;
            self.expect(Tok::LParen)?;
            let args = self.args()?;
            node = Node {
                span: node.span.start..args.end,
                kind: NodeKind::Method(MethodNode {
                    recv: Box::new(node),
                    name: method.name,
                    name_span: method.span,
                    args: args.nodes,
                }),
            };
        }
        Ok(node)
    }

    /// Parses arguments after an already consumed `(`.
    fn args(&mut self) -> Result<Args, LangError> {
        let mut nodes = Vec::new();
        if *self.peek() == Tok::RParen {
            let end = self.bump().span.end;
            return Ok(Args { nodes, end });
        }
        loop {
            nodes.push(self.expr()?);
            match self.peek() {
                Tok::Comma => {
                    self.bump();
                }
                Tok::RParen => {
                    let end = self.bump().span.end;
                    return Ok(Args { nodes, end });
                }
                _ => return Err(self.unexpected("`,` oder `)`")),
            }
        }
    }

    fn primary(&mut self) -> R {
        match self.peek().clone() {
            Tok::Int(value) => Ok(Node {
                kind: NodeKind::Int(value),
                span: self.bump().span,
            }),
            Tok::LParen => {
                let start = self.bump().span.start;
                let inner = self.expr()?;
                let end = self.expect(Tok::RParen)?.span.end;
                Ok(Node {
                    kind: inner.kind,
                    span: start..end,
                })
            }
            Tok::Ident(name) => {
                let span = self.bump().span;
                self.ident_expr(name, span)
            }
            _ => Err(self.unexpected("Wert, Name oder `(`")),
        }
    }

    /// Name, function call or quantifier, after the identifier was consumed.
    fn ident_expr(&mut self, name: String, span: Span) -> R {
        let followed_by_paren = *self.peek() == Tok::LParen;
        if let Some(kind) = QuantKind::from_keyword(&name)
            && followed_by_paren
        {
            return self.quant(kind, span.start);
        }
        if KEYWORDS.contains(&name.as_str()) {
            return Err(LangError::new(
                format!("`{name}` ist ein Schlüsselwort und hier nicht erlaubt"),
                span,
            ));
        }
        if !followed_by_paren {
            return Ok(Node {
                kind: NodeKind::Name(name),
                span,
            });
        }
        self.bump();
        let args = self.args()?;
        Ok(Node {
            kind: NodeKind::Call { name },
            span: span.start..args.end,
        })
    }

    fn quant(&mut self, kind: QuantKind, start: usize) -> R {
        self.expect(Tok::LParen)?;
        let var = self.ident("Variablenname")?;
        if !self.is_kw("in") {
            return Err(self.unexpected("`in`"));
        }
        self.bump();
        let domain = self.ident("Gruppe (others, opponents, players, farben, suits)")?;
        self.expect(Tok::Comma)?;
        let body = self.expr()?;
        let end = self.expect(Tok::RParen)?.span.end;
        Ok(Node {
            kind: NodeKind::Quant(QuantNode {
                kind,
                var: var.name,
                var_span: var.span,
                domain: domain.name,
                domain_span: domain.span,
                body: Box::new(body),
            }),
            span: start..end,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// S-expression rendering to assert tree shape.
    fn sexp(n: &Node) -> String {
        match &n.kind {
            NodeKind::Int(v) => v.to_string(),
            NodeKind::Name(s) => s.clone(),
            NodeKind::Not(i) => format!("(not {})", sexp(i)),
            NodeKind::Binary { op, lhs, rhs } => format!("({op:?} {} {})", sexp(lhs), sexp(rhs)),
            NodeKind::Call { name } => format!("(call {name})"),
            NodeKind::Method(m) => {
                let args: Vec<_> = m.args.iter().map(sexp).collect();
                format!("({}.{} {})", sexp(&m.recv), m.name, args.join(" "))
            }
            NodeKind::Quant(q) => {
                format!("({:?} {} {} {})", q.kind, q.var, q.domain, sexp(&q.body))
            }
        }
    }

    fn p(src: &str) -> String {
        parse(src)
            .map(|n| sexp(&n))
            .unwrap_or_else(|e| e.render(src))
    }

    #[test]
    fn precedence() {
        assert_eq!(p("a or b and not c"), "(Or a (And b (not c)))");
        assert_eq!(
            p("trumpf - ober | EA & x"),
            "(Union (Sub trumpf ober) (Inter EA x))"
        );
        assert_eq!(
            p("me.count(trumpf) + 1 >= 4"),
            "(Cmp(Ge) (Add (me.count trumpf) 1) 4)"
        );
    }

    #[test]
    fn quantifier() {
        assert_eq!(
            p("any(p in others, p.has(HO, SO))"),
            "(Any p others (p.has HO SO))"
        );
        assert_eq!(
            p("num(f in farben, me.frei(f)) >= 1"),
            "(Cmp(Ge) (Num f farben (me.frei f)) 1)"
        );
    }

    #[test]
    fn errors() {
        let err = |src| parse(src).err().unwrap_or_else(|| LangError::new("", 0..0));
        assert!(err("a == b == c").message.contains("verketten"));
        let e = err("me.count(trumpf) = 3");
        assert!(e.message.contains("=="));
        assert_eq!(e.span, 17..18);
        assert!(!err("any(p others, x)").message.is_empty());
        assert!(!err("(a").message.is_empty());
        assert!(!err("a b").message.is_empty());
    }
}
