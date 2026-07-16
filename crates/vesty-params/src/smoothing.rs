#[derive(Clone, Copy, Debug)]
pub struct LinearSmoother {
    current: f32,
    target: f32,
    step: f32,
    remaining: u32,
}

impl LinearSmoother {
    pub fn new(value: f32) -> Self {
        Self {
            current: value,
            target: value,
            step: 0.0,
            remaining: 0,
        }
    }

    pub fn reset(&mut self, value: f32) {
        self.current = value;
        self.target = value;
        self.step = 0.0;
        self.remaining = 0;
    }

    pub fn set_target(&mut self, target: f32, samples: u32) {
        self.target = target;
        self.remaining = samples;
        self.step = if samples == 0 {
            self.current = target;
            0.0
        } else {
            (target - self.current) / samples as f32
        };
    }

    pub fn next_value(&mut self) -> f32 {
        if self.remaining == 0 {
            self.current = self.target;
            return self.current;
        }

        self.current += self.step;
        self.remaining -= 1;
        if self.remaining == 0 {
            self.current = self.target;
        }
        self.current
    }
}
