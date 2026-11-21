use super::ir::{BoolExpr, BoolQuant, Domain, IntExpr, MAX_VARS, PlayerRef, Quant, SetExpr};
use crate::cards::{Cards, Suit};
use crate::game::{Game, PLAYER_COUNT, Seat};

#[derive(Clone, Copy, Debug)]
pub struct Deal {
    pub hands: [Cards; PLAYER_COUNT],
    /// Holder of the called Sau (Sauspiel only).
    pub partner: Option<Seat>,
}

pub struct Evaluator<'a> {
    game: &'a Game,
    deal: &'a Deal,
    vars: [u8; MAX_VARS],
}

/// Seats and suits both come in fours.
const MAX_DOMAIN_SIZE: usize = 4;

/// Seat or suit indices a quantifier iterates over.
#[derive(Default)]
struct Values {
    items: [u8; MAX_DOMAIN_SIZE],
    len: usize,
}

impl Values {
    fn push(&mut self, index: usize) {
        if let Some(slot) = self.items.get_mut(self.len) {
            *slot = index as u8;
            self.len += 1;
        }
    }

    fn as_slice(&self) -> &[u8] {
        &self.items[..self.len]
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BodyIs {
    True,
    False,
}

impl<'a> Evaluator<'a> {
    pub fn new(game: &'a Game, deal: &'a Deal) -> Self {
        Evaluator {
            game,
            deal,
            vars: [0; MAX_VARS],
        }
    }

    pub fn eval(&mut self, e: &BoolExpr) -> bool {
        match e {
            BoolExpr::And(a, b) => self.eval(a) && self.eval(b),
            BoolExpr::Or(a, b) => self.eval(a) || self.eval(b),
            BoolExpr::Not(a) => !self.eval(a),
            BoolExpr::Cmp(op, a, b) => {
                let (a, b) = (self.int(a), self.int(b));
                op.apply(a, b)
            }
            BoolExpr::HasAll(p, s) => self.set(s).is_subset_of(self.hand(*p)),
            BoolExpr::HasAny(p, s) => self.hand(*p).intersects(self.set(s)),
            BoolExpr::Quant(kind, q) => match kind {
                BoolQuant::Any => self.exists(q, BodyIs::True),
                BoolQuant::NoneOf => !self.exists(q, BodyIs::True),
                BoolQuant::All => !self.exists(q, BodyIs::False),
            },
        }
    }

    fn int(&mut self, e: &IntExpr) -> i64 {
        match e {
            IntExpr::Const(v) => *v,
            IntExpr::Count(p, s) => (self.hand(*p) & self.set(s)).len() as i64,
            IntExpr::Add(a, b) => self.int(a) + self.int(b),
            IntExpr::Sub(a, b) => self.int(a) - self.int(b),
            IntExpr::Num(q) => self.count(q) as i64,
        }
    }

    fn exists(&mut self, q: &Quant, want: BodyIs) -> bool {
        let values = self.domain(q.domain);
        values.as_slice().iter().any(|&v| {
            self.vars[q.slot as usize] = v;
            let got = if self.eval(&q.body) {
                BodyIs::True
            } else {
                BodyIs::False
            };
            got == want
        })
    }

    fn count(&mut self, q: &Quant) -> usize {
        let values = self.domain(q.domain);
        values
            .as_slice()
            .iter()
            .filter(|&&v| {
                self.vars[q.slot as usize] = v;
                self.eval(&q.body)
            })
            .count()
    }

    fn domain(&self, d: Domain) -> Values {
        let mut values = Values::default();
        let seats: &[Seat] = match d {
            Domain::Players => &Seat::ALL,
            Domain::Others | Domain::Opponents => &Seat::OTHERS,
            Domain::Farben | Domain::Suits => &[],
        };
        seats
            .iter()
            .filter(|&&s| d != Domain::Opponents || self.deal.partner != Some(s))
            .for_each(|s| values.push(s.index()));
        let suits: &[Suit] = match d {
            Domain::Suits => &Suit::ALL,
            Domain::Farben => &self.game.plain_suits,
            _ => &[],
        };
        suits.iter().for_each(|s| values.push(s.index()));
        values
    }

    fn hand(&self, p: PlayerRef) -> Cards {
        let seat = match p {
            PlayerRef::Seat(s) => s.index(),
            // The compiler only emits `Partner` for Sauspiel, where it is always set.
            PlayerRef::Partner => match self.deal.partner {
                Some(seat) => seat.index(),
                None => return Cards::EMPTY,
            },
            PlayerRef::Var(slot) => self.vars[slot as usize] as usize,
        };
        self.deal.hands[seat]
    }

    fn set(&self, e: &SetExpr) -> Cards {
        match e {
            SetExpr::Const(c) => *c,
            SetExpr::SuitVar(slot) => self
                .game
                .suit_set(Suit::ALL[self.vars[*slot as usize] as usize]),
            SetExpr::Union(a, b) => self.set(a) | self.set(b),
            SetExpr::Inter(a, b) => self.set(a) & self.set(b),
            SetExpr::Diff(a, b) => self.set(a) - self.set(b),
        }
    }
}
