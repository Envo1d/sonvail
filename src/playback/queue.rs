use std::sync::Arc;

use rand::seq::SliceRandom;

use crate::library::Track;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Repeat {
    #[default]
    Off,
    All,
    One,
}

impl Repeat {
    pub fn next(self) -> Self {
        match self {
            Self::Off => Self::All,
            Self::All => Self::One,
            Self::One => Self::Off,
        }
    }
}

#[derive(Default)]
pub(super) struct Queue {
    pub tracks: Arc<Vec<Track>>,
    pub order: Arc<Vec<usize>>,
    pub cursor: Option<usize>,
    pub repeat: Repeat,
    pub shuffle: bool,
}

impl Queue {
    pub fn current(&self) -> Option<usize> {
        self.cursor
            .and_then(|cursor| self.order.get(cursor).copied())
    }

    pub fn replace(&mut self, tracks: Arc<Vec<Track>>, selected: usize) -> bool {
        self.tracks = tracks;
        let mut order: Vec<_> = (0..self.tracks.len()).collect();
        self.cursor = (selected < order.len()).then_some(selected);
        if self.shuffle && self.cursor.is_some() {
            order.swap(0, selected);
            order[1..].shuffle(&mut rand::rng());
            self.cursor = Some(0);
        }
        self.order = Arc::new(order);
        self.cursor.is_some()
    }

    pub fn set_shuffle(&mut self, shuffle: bool) {
        let current = self.current();
        self.shuffle = shuffle;
        if shuffle {
            let cursor = self.cursor.map_or(0, |cursor| cursor + 1);
            Arc::make_mut(&mut self.order)[cursor..].shuffle(&mut rand::rng());
        } else {
            self.order = Arc::new((0..self.tracks.len()).collect());
            self.cursor = current;
        }
    }

    pub fn advance(&mut self, automatic: bool) -> Option<usize> {
        let cursor = self.cursor?;
        if automatic && self.repeat == Repeat::One {
            return self.current();
        }
        if cursor + 1 < self.order.len() {
            self.cursor = Some(cursor + 1);
        } else if self.repeat == Repeat::All && !self.order.is_empty() {
            if self.shuffle && self.order.len() > 1 {
                let previous = self.current();
                let order = Arc::make_mut(&mut self.order);
                order.shuffle(&mut rand::rng());
                if order.first().copied() == previous {
                    order.swap(0, 1);
                }
            }
            self.cursor = Some(0);
        } else {
            return None;
        }
        self.current()
    }

    pub fn previous(&mut self) -> Option<usize> {
        self.cursor = self.cursor.map(|cursor| {
            if cursor > 0 {
                cursor - 1
            } else if self.repeat == Repeat::All {
                self.order.len().saturating_sub(1)
            } else {
                0
            }
        });
        self.current()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn queue(count: usize) -> Queue {
        let mut queue = Queue::default();
        queue.replace(
            Arc::new(
                (0..count)
                    .map(|index| Track::from_path(format!("{index}.wav").into()))
                    .collect(),
            ),
            0,
        );
        queue
    }

    #[test]
    fn empty_queue_and_repeat_modes_have_finite_transitions() {
        assert_eq!(queue(0).advance(true), None);
        let mut q = queue(2);
        q.repeat = Repeat::One;
        assert_eq!(q.advance(true), Some(0));
        assert_eq!(q.advance(false), Some(1));
        q.repeat = Repeat::Off;
        assert_eq!(q.advance(true), None);
        q.repeat = Repeat::All;
        assert_eq!(q.advance(true), Some(0));
        assert_eq!(q.previous(), Some(1));
    }

    #[test]
    fn shuffle_preserves_current_and_visits_each_remaining_track_once() {
        let mut q = queue(12);
        q.advance(false);
        q.set_shuffle(true);
        assert_eq!(q.current(), Some(1));
        let mut visited = vec![0, 1];
        while let Some(index) = q.advance(false) {
            visited.push(index);
        }
        visited.sort_unstable();
        assert_eq!(visited, (0..12).collect::<Vec<_>>());
        let current = q.current();
        q.set_shuffle(false);
        assert_eq!(q.current(), current);
    }
}
