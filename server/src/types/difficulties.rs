use crate::types::Difficulty;

#[derive(Clone, Debug)]
pub struct Difficulties {
    pub(crate) current: Difficulty,
    pub(crate) old: Difficulty,
    pub(crate) next: Difficulty,
}

impl Difficulties {
    pub fn new_only_current(current: Difficulty) -> Self {
        Difficulties {
            current,
            old: Difficulty::zero(),
            next: Difficulty::zero(),
        }
    }

    pub fn current(&self) -> Difficulty {
        self.current
    }

    pub fn old(&self) -> Difficulty {
        self.old
    }

    pub fn next(&self) -> Option<Difficulty> {
        if self.next.is_zero() {
            None
        } else {
            Some(self.next)
        }
    }

    pub fn update_next(&mut self, next: Difficulty) {
        self.next = next;
    }

    pub(crate) fn shift(&mut self) -> Option<Difficulty> {
        if self.next.is_zero() {
            None
        } else {
            self.old = self.current;
            self.current = self.next;
            self.next = Difficulty::zero();
            Some(self.current)
        }
    }

    pub(crate) fn set_and_shift(&mut self, current: Difficulty) {
        self.old = self.current;
        self.current = current;
        self.next = Difficulty::zero();
    }
}
