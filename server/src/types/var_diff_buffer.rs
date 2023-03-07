#[derive(Debug)]
pub struct VarDiffBuffer {
    pub(crate) pos: usize,
    pub(crate) used: usize,
    pub(crate) data: [u128; 90],
}

impl VarDiffBuffer {
    pub fn new() -> VarDiffBuffer {
        VarDiffBuffer {
            pos: 0,
            used: 0,
            data: [0; 90],
        }
    }

    pub(crate) fn append(&mut self, time: u128) {
        self.data[self.pos] = time;
        self.pos += 1;
        self.pos %= 90;

        if self.used < 90 {
            self.used += 1;
        }
    }

    pub(crate) fn reset(&mut self) {
        self.pos = 0;
        self.used = 0;
    }

    pub(crate) fn avg(&self) -> f64 {
        let mut count = 90;

        if self.used < 90 {
            count = self.pos;
        }

        let mut total: u128 = 0;
        for i in 0..count {
            total += self.data[i];
        }

        (total as f64) / (count as f64)
    }
}
