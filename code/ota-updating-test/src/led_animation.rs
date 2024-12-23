// use smart_leds::hsv::{hsv2rgb, Hsv};
use smart_leds::RGB8;


pub const RED: RGB8 = RGB8 { r: 0xff, g: 0, b: 0, };
pub const GREEN: RGB8 = RGB8 { r: 0, g: 0xff, b: 0, };
pub const BLUE: RGB8 = RGB8 { r: 0, g: 0, b: 0xff, };
pub const WHITE: RGB8 = RGB8 { r: 0xff, g: 0xff, b: 0xff, };

pub const PINK: RGB8 = RGB8 { r: 0xff, g: 0, b: 0xff, };
pub const CYAN: RGB8 = RGB8 { r: 0, g: 0xff, b: 0xff, };

pub const RED_H: RGB8 = RGB8 { r: 0x0f, g: 0, b: 0, };
pub const GREEN_H: RGB8 = RGB8 { r: 0, g: 0x0f, b: 0, };
pub const BLUE_H: RGB8 = RGB8 { r: 0, g: 0, b: 0x0f, };
pub const WHITE_H: RGB8 = RGB8 { r: 0x0f, g: 0x0f, b: 0x0f, };
pub const BLACK: RGB8 = RGB8 { r: 0, g: 0, b: 0, };

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
    // TODO add conversion checks on input range
    fn convert_ms_to_time_step(time: u64) -> u8 {
        let conv = (time - 10) / 10;
        conv.clamp(0, u8::MAX as u64) as u8
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
    pub fn new_rotation(mut pat: LedPattern) -> Self {
        let mut entries = Vec::new();
        for _ in 0..LED_COUNT {
            entries.push(pat.clone());
            pat.led_data.rotate_right(1);
        }
        Self {
            entries,
            index: 0,
        }
    }
    // TODO fix patterns with only 1 state hanging
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
