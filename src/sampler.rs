//! Draws my hand from the slots and deals the remaining 24 cards.

use crate::cards::{Card, Cards, DECK_SIZE};
use crate::game::Seat;
use crate::lang::eval::Deal;
use crate::scenario::{HAND_SIZE, Scenario};
use rand_xoshiro::rand_core::Rng;

const HAND: usize = HAND_SIZE as usize;
const REST: usize = DECK_SIZE - HAND;

/// Uniform integer in `0..n` (multiply-shift; bias < 2^-58 for n ≤ 32).
fn below(rng: &mut impl Rng, n: usize) -> usize {
    ((rng.next_u64() as u128 * n as u128) >> 64) as usize
}

struct SlotCards {
    cards: Vec<u8>,
    count: usize,
}

#[derive(Clone, Copy)]
struct CalledSau {
    sau: Card,
    suit: Cards,
}

pub struct Sampler {
    fixed: Cards,
    slots: Vec<SlotCards>,
    called: Option<CalledSau>,
}

impl Sampler {
    pub fn new(scn: &Scenario) -> Sampler {
        Sampler {
            fixed: scn.fixed,
            slots: scn
                .slots
                .iter()
                .map(|s| SlotCards {
                    cards: s.cards.iter().map(Card::index).collect(),
                    count: s.count as usize,
                })
                .collect(),
            called: scn.game.called_sau().map(|sau| CalledSau {
                sau,
                suit: scn.game.suit_set(sau.suit()),
            }),
        }
    }

    pub fn my_hand(&self, rng: &mut impl Rng) -> Cards {
        let mut hand = self.fixed;
        let mut buf = [0u8; DECK_SIZE];
        for slot in &self.slots {
            let n = slot.cards.len();
            buf[..n].copy_from_slice(&slot.cards);
            for i in 0..slot.count {
                let j = i + below(rng, n - i);
                buf.swap(i, j);
                hand |= Card::from_index(buf[i]).mask();
            }
        }
        hand
    }

    /// Rufregel: I may not hold the called Sau but need a card of its suit.
    pub fn may_call(&self, me: Cards) -> bool {
        self.called
            .is_none_or(|c| !me.contains(c.sau) && me.intersects(c.suit))
    }

    pub fn deal(&self, rng: &mut impl Rng, me: Cards) -> Deal {
        let mut rest = [0u8; REST];
        for (slot, card) in rest.iter_mut().zip((Cards::ALL - me).iter()) {
            *slot = card.index();
        }
        // The last hand is whatever remains, so only two hands need shuffling.
        for i in 0..2 * HAND {
            let j = i + below(rng, REST - i);
            rest.swap(i, j);
        }
        let hand = |cards: &[u8]| {
            cards
                .iter()
                .map(|&i| Card::from_index(i))
                .collect::<Cards>()
        };
        let (left, others) = rest.split_at(HAND);
        let (across, right) = others.split_at(HAND);
        let hands = [me, hand(left), hand(across), hand(right)];
        let partner = self.called.and_then(|c| {
            Seat::ALL
                .into_iter()
                .find(|s| hands[s.index()].contains(c.sau))
        });
        Deal { hands, partner }
    }
}
