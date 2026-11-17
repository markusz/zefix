use super::{LangError, Span};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Tok {
    Ident(String),
    Int(i64),
    LParen,
    RParen,
    Comma,
    Dot,
    Pipe,
    Amp,
    Plus,
    Minus,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Eof,
}

impl Tok {
    pub fn describe(&self) -> String {
        let symbol = match self {
            Tok::Ident(s) => return format!("`{s}`"),
            Tok::Int(i) => return format!("`{i}`"),
            Tok::Eof => return "Ende des Ausdrucks".into(),
            Tok::LParen => "(",
            Tok::RParen => ")",
            Tok::Comma => ",",
            Tok::Dot => ".",
            Tok::Pipe => "|",
            Tok::Amp => "&",
            Tok::Plus => "+",
            Tok::Minus => "-",
            Tok::Eq => "==",
            Tok::Ne => "!=",
            Tok::Lt => "<",
            Tok::Le => "<=",
            Tok::Gt => ">",
            Tok::Ge => ">=",
        };
        format!("`{symbol}`")
    }
}

#[derive(Clone, Debug)]
pub struct Token {
    pub tok: Tok,
    pub span: Span,
}

pub fn lex(src: &str) -> Result<Vec<Token>, LangError> {
    let mut lexer = Lexer { src, pos: 0 };
    let mut out = Vec::new();
    while let Some(token) = lexer.next_token()? {
        out.push(token);
    }
    out.push(Token {
        tok: Tok::Eof,
        span: src.len()..src.len(),
    });
    Ok(out)
}

struct Lexer<'a> {
    src: &'a str,
    pos: usize,
}

impl<'a> Lexer<'a> {
    fn next_token(&mut self) -> Result<Option<Token>, LangError> {
        self.take_while(|b| b.is_ascii_whitespace());
        let start = self.pos;
        let Some(&c) = self.src.as_bytes().get(start) else {
            return Ok(None);
        };
        let tok = if c.is_ascii_alphabetic() || c == b'_' {
            Tok::Ident(
                self.take_while(|b| b.is_ascii_alphanumeric() || b == b'_')
                    .to_string(),
            )
        } else if c.is_ascii_digit() {
            let digits = self.take_while(|b| b.is_ascii_digit());
            let value = digits
                .parse()
                .map_err(|_| LangError::new("Zahl zu groß", start..self.pos))?;
            Tok::Int(value)
        } else {
            self.symbol()?
        };
        Ok(Some(Token {
            tok,
            span: start..self.pos,
        }))
    }

    fn take_while(&mut self, pred: impl Fn(u8) -> bool) -> &'a str {
        let start = self.pos;
        while self.src.as_bytes().get(self.pos).is_some_and(|&b| pred(b)) {
            self.pos += 1;
        }
        &self.src[start..self.pos]
    }

    fn symbol(&mut self) -> Result<Tok, LangError> {
        let start = self.pos;
        let rest = &self.src[start..];
        if let Some(tok) = two_char_token(rest) {
            self.pos += 2;
            return Ok(tok);
        }
        let ch = rest.chars().next().unwrap_or_default();
        let tok = match ch {
            '(' => Tok::LParen,
            ')' => Tok::RParen,
            ',' => Tok::Comma,
            '.' => Tok::Dot,
            '|' => Tok::Pipe,
            '&' => Tok::Amp,
            '+' => Tok::Plus,
            '-' => Tok::Minus,
            '<' => Tok::Lt,
            '>' => Tok::Gt,
            '=' => {
                return Err(LangError::new(
                    "`=` ist kein Operator, meintest du `==`?",
                    start..start + 1,
                ));
            }
            _ => {
                return Err(LangError::new(
                    format!("unerwartetes Zeichen `{ch}`"),
                    start..start + ch.len_utf8(),
                ));
            }
        };
        self.pos += 1;
        Ok(tok)
    }
}

fn two_char_token(rest: &str) -> Option<Tok> {
    match rest.get(..2)? {
        "==" => Some(Tok::Eq),
        "!=" => Some(Tok::Ne),
        "<=" => Some(Tok::Le),
        ">=" => Some(Tok::Ge),
        _ => None,
    }
}
