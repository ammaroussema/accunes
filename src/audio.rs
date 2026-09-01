use std::collections::VecDeque;

pub const UNDERRUN_RAMP_FRAMES: f32 = 1280.0;
pub struct AudioRingBuffer {
    buf: VecDeque<f32>,
}

impl AudioRingBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            buf: VecDeque::with_capacity(capacity),
        }
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn sample_at(&self, i: usize) -> f32 {
        self.buf[i]
    }

    pub fn push(&mut self, v: f32) {
        self.buf.push_back(v);
    }

    pub fn consume(&mut self, n: usize) {
        let n = n.min(self.buf.len());
        self.buf.drain(..n);
    }

    pub fn clear_and_reset(&mut self) {
        self.buf.clear();
    }

    pub fn fill_silence(&mut self, n: usize) {
        self.buf.extend(std::iter::repeat(0.0f32).take(n));
    }
}
