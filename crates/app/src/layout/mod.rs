//! Screen layouts. `Layout` is the only place that looks at the screen width; each layout then
//! composes the shared views (seats, queries, results) in its own way.

pub mod controls;
pub mod desktop;
pub mod phone;

use crate::theme::Density;

/// Below this width (phones) the phone layout takes over.
const NARROW_WIDTH: f32 = 700.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    /// Seats beside queries and results.
    Desktop,
    /// One section at a time, results inside each query, no query-language text.
    Phone,
}

impl Layout {
    pub fn for_width(width: f32) -> Layout {
        if width < NARROW_WIDTH {
            Layout::Phone
        } else {
            Layout::Desktop
        }
    }

    pub fn density(self) -> Density {
        match self {
            Layout::Desktop => Density::Regular,
            Layout::Phone => Density::Compact,
        }
    }
}
