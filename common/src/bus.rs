//! A bus layout, as edited by the UI and saved to disk.

use std::{fs, io, path::Path};

use serde::{Deserialize, Serialize};

/// Belts in a default bus group.
pub const GROUP_BELTS: usize = 6;
/// Empty tiles between two default bus groups.
pub const GROUP_GAP: usize = 2;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bus {
    /// Tile-wide columns of the bus, as drawn from left to right.
    pub slots: Vec<Slot>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Slot {
    Empty,
    /// `left` and `right` are the belt sides as drawn, regardless of `direction`.
    Belt {
        left: Option<String>,
        right: Option<String>,
        direction: Direction,
    },
    Pipe {
        fluid: Option<String>,
        direction: Direction,
    },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    #[default]
    Up,
    Down,
}

impl Direction {
    pub fn reversed(self) -> Self {
        match self {
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
        }
    }
}

impl Slot {
    pub fn empty_belt() -> Self {
        Slot::Belt {
            left: None,
            right: None,
            direction: Direction::default(),
        }
    }

    pub fn direction(&self) -> Option<Direction> {
        match self {
            Slot::Empty => None,
            Slot::Belt { direction, .. } | Slot::Pipe { direction, .. } => Some(*direction),
        }
    }
}

impl Default for Bus {
    /// Two groups of belts separated by a gap, like a typical Nullius bus.
    fn default() -> Self {
        let mut bus = Bus { slots: Vec::new() };
        bus.push_group();
        bus.push_group();
        bus
    }
}

impl Bus {
    /// Appends a group of empty belts, preceded by a gap if the bus isn't empty.
    pub fn push_group(&mut self) {
        if !self.slots.is_empty() {
            self.slots.extend(std::iter::repeat_n(Slot::Empty, GROUP_GAP));
        }
        self.slots
            .extend(std::iter::repeat_n(Slot::empty_belt(), GROUP_BELTS));
    }

    pub fn load(path: &Path) -> io::Result<Self> {
        let text = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&text)?)
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        let text = serde_json::to_string_pretty(self)?;
        fs::write(path, text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_bus_is_two_groups_with_a_gap() {
        let bus = Bus::default();
        assert_eq!(bus.slots.len(), 2 * GROUP_BELTS + GROUP_GAP);
        assert_eq!(bus.slots[GROUP_BELTS], Slot::Empty);
        assert_eq!(bus.slots[0], Slot::empty_belt());
    }

    #[test]
    fn round_trips_through_json() {
        let bus = Bus {
            slots: vec![
                Slot::Empty,
                Slot::Belt {
                    left: Some("iron-plate".into()),
                    right: None,
                    direction: Direction::Down,
                },
                Slot::Pipe {
                    fluid: Some("water".into()),
                    direction: Direction::Up,
                },
            ],
        };
        let json = serde_json::to_string(&bus).unwrap();
        assert_eq!(serde_json::from_str::<Bus>(&json).unwrap(), bus);
    }
}
