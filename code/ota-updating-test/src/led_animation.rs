// use smart_leds::hsv::{hsv2rgb, Hsv};
use smart_leds::RGB8;

const LED_COUNT: usize = 5;

#[derive(Clone)]
pub struct LedPattern {
    time_step: u8,
    pub led_data: [RGB8; LED_COUNT],
}

impl LedPattern {
    pub fn new(time: u64, led_data: [RGB8; LED_COUNT]) -> Self {
        LedPattern {
            time_step: Self::convert_ms_to_time_step(time),
            led_data,
        }
    }
    /*
     * time step is biased starting from 10ms in 10ms steps
     */
    pub fn time_step_ms(&self) -> u64 {
        self.time_step as u64 * 10 + 10
    }

    fn convert_ms_to_time_step(time: u64) -> u8 {
        ((time - 10) / 10) as u8
    }

}

pub struct LedAnimation {
    entries: Vec<LedPattern>,
    index: usize,
}

impl LedAnimation {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            index: 0,
        }
    }
    pub fn next_pattern(&mut self) -> Option<LedPattern> {
        let ret = if let Some(pat) = self.entries.get(self.index) {
            Some(pat.clone())
        } else {
            None
        };
        self.index = (self.index + 1) % self.entries.len();
        ret
    }
    pub fn add_pattern(&mut self, pattern: LedPattern) {
        self.entries.push(pattern);
    }
}
