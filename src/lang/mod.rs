//! The query language: `any(p in others, p.has(HO, SO) and p.count(trumpf) >= 4)`.

pub mod ast;
pub mod compile;
pub mod eval;
pub mod ir;
pub mod lexer;
pub mod parser;

pub type Span = std::ops::Range<usize>;

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct LangError {
    pub message: String,
    pub span: Span,
}

impl LangError {
    pub fn new(message: impl Into<String>, span: Span) -> LangError {
        LangError {
            message: message.into(),
            span,
        }
    }

    /// Error message plus the offending source line with a caret marker.
    pub fn render(&self, src: &str) -> String {
        let start = self.span.start.min(src.len());
        let line_start = src[..start].rfind('\n').map_or(0, |i| i + 1);
        let line_end = src[start..].find('\n').map_or(src.len(), |i| start + i);
        let line = &src[line_start..line_end];
        let col = src[line_start..start].chars().count();
        let end = self.span.end.clamp(start, line_end);
        let width = src[start..end].chars().count().max(1);
        format!(
            "{}\n    {}\n    {}{}",
            self.message,
            line,
            " ".repeat(col),
            "^".repeat(width)
        )
    }
}
