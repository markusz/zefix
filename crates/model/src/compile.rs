//! The model as query-language text, and whether core accepts it. The UI shows both before a
//! run starts: previews on the desktop, errors next to what caused them.

use crate::model::Model;
use serde::Serialize;
use zefix_core::scenario::{ExprSite, Scenario, ScenarioError};

#[derive(Serialize)]
pub struct Compiled {
    /// Condition on the deal; `None` if nothing is constrained.
    pub filter: Option<String>,
    /// Cards dealt to me directly.
    pub fixed: Vec<String>,
    /// One per query in model order; `expr` is `None` while the query is empty.
    pub queries: Vec<CompiledQuery>,
    pub problem: Option<Problem>,
}

#[derive(Serialize)]
pub struct CompiledQuery {
    pub name: String,
    pub expr: Option<String>,
}

/// Why the scenario cannot run, located so the UI can show it in place.
#[derive(Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Problem {
    /// The free condition (`[me] where`) does not compile; `rendered` has a caret.
    Filter {
        rendered: String,
    },
    Query {
        name: String,
        rendered: String,
    },
    NoQueries,
    InvalidGame {
        message: String,
    },
    Other {
        message: String,
    },
}

impl Problem {
    pub fn of(error: &ScenarioError) -> Problem {
        match error {
            ScenarioError::Expr {
                site: ExprSite::MeWhere,
                src,
                error,
            } => Problem::Filter {
                rendered: error.render(src),
            },
            ScenarioError::Expr {
                site: ExprSite::Query { name },
                src,
                error,
            } => Problem::Query {
                name: name.clone(),
                rendered: error.render(src),
            },
            ScenarioError::NoQueries => Problem::NoQueries,
            other => Problem::Other {
                message: other.to_string(),
            },
        }
    }
}

pub fn compile(model: &Model) -> Compiled {
    let Some(game) = model.game.game() else {
        return Compiled {
            filter: None,
            fixed: Vec::new(),
            queries: Vec::new(),
            problem: Some(Problem::InvalidGame {
                message: "Ungültiges Spiel.".into(),
            }),
        };
    };
    let spec = model.spec(&game);
    let problem = Scenario::from_config(spec.to_config())
        .err()
        .map(|e| Problem::of(&e));
    Compiled {
        filter: spec.filter,
        fixed: spec.my_cards.iter().map(|c| c.to_string()).collect(),
        queries: model
            .queries
            .iter()
            .map(|q| crate::compile::CompiledQuery {
                name: q.name.clone(),
                expr: q.expr(&game),
            })
            .collect(),
        problem,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{QueryMode, Scope};

    #[test]
    fn default_model_runs() {
        let compiled = compile(&Model::default());
        assert_eq!(compiled.problem, None);
        assert_eq!(
            compiled.queries[0].expr.as_deref(),
            Some("any(p in opponents, p.count(trumpf) >= 4)")
        );
    }

    #[test]
    fn errors_name_their_query() {
        let mut model = Model::default();
        model.queries[0].mode = QueryMode::Text;
        model.queries[0].text = "me.count(trumf) > 1".into();
        let problem = compile(&model).problem;
        assert!(
            matches!(&problem, Some(Problem::Query { name, rendered })
                if name == "Ein Gegner hat ≥ 4 Trumpf" && rendered.contains("trumf")),
            "{problem:?}"
        );
        model.queries[0].scope = Scope::AnyOther;
        model.queries.clear();
        assert_eq!(compile(&model).problem, Some(Problem::NoQueries));
    }
}
