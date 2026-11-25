//! Which sharks share the sea, and how big they are.

use crate::game;
use crate::math;
use crate::world::{SharkKind, SharkSpec, World};

pub const MIN_SIZE: f64 = 0.55;

/// All different, and none of them `except`, while there are enough kinds to go round.
pub fn random_kinds(n: usize, except: Option<SharkKind>) -> Vec<SharkKind> {
    let mut kinds = SharkKind::ALL.to_vec();
    kinds.retain(|&k| Some(k) != except);
    for i in (1..kinds.len()).rev() {
        kinds.swap(i, (math::random() * (i + 1) as f64) as usize);
    }
    (0..n)
        .map(|i| {
            kinds
                .get(i)
                .copied()
                .unwrap_or_else(|| kinds[(math::random() * kinds.len() as f64) as usize])
        })
        .collect()
}

/// One shark at full size, the rest smaller. Size doubles as depth: smaller
/// sharks swim deeper and dimmer.
pub fn random_sizes(kinds: &[SharkKind]) -> Vec<SharkSpec> {
    let biggest = (math::random() * kinds.len() as f64) as usize;
    kinds
        .iter()
        .enumerate()
        .map(|(i, &kind)| SharkSpec {
            kind,
            size: if i == biggest {
                1.0
            } else {
                math::rand(MIN_SIZE, 1.0)
            },
            player: false,
        })
        .collect()
}

pub struct Setup {
    /// Used while there are exactly as many as the rivals asked for.
    pub others: Option<Vec<SharkKind>>,
    pub fish: Option<usize>,
}

impl Setup {
    /// A sea with `rivals` other sharks, and the player's shark if there is one.
    pub fn sea(&self, player: Option<SharkKind>, rivals: usize) -> World {
        let others = match &self.others {
            Some(kinds) if kinds.len() == rivals => kinds.clone(),
            _ => random_kinds(rivals, player),
        };
        let mut sharks = random_sizes(&others);
        sharks.extend(player.map(|kind| SharkSpec {
            kind,
            size: game::START_SIZE,
            player: true,
        }));
        World::new(sharks, self.fish)
    }
}
