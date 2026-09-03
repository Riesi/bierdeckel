#[warn(dead_code)]
use ws2812_rs::Color;
use core::derive;
use core::option::Option::{None, Some};
use core::option::Option;
use core::clone::Clone;
use core::cmp::Ord;
extern crate alloc;
use alloc::vec::Vec;

pub const RED: Color = Color([0xFF, 0, 0]);
pub const GREEN: Color = Color([0, 0xFF, 0]);
pub const BLUE: Color = Color([0, 0, 0xFF]);

pub const WHITE: Color = Color([0xFF, 0xFF, 0xFF]);
pub const BLACK: Color = Color([0, 0, 0]);

pub const YELLOW: Color = Color([0xFF, 0xFF, 0]);
pub const PINK: Color = Color([0xFF, 0, 0xFF]);
pub const CYAN: Color = Color([0, 0xFF, 0xFF]);

pub const RED_H: Color = Color([0x0F, 0, 0]);
pub const GREEN_H: Color = Color([0, 0x0F, 0]);
pub const BLUE_H: Color = Color([0, 0, 0x0F]);
pub const WHITE_H: Color = Color([0x0F, 0x0F, 0x0F]);

pub const YELLOW_H: Color = Color([0, 0, 0]);
pub const PINK_H: Color = Color([0x0F, 0, 0x0F]);
pub const CYAN_H: Color = Color([0, 0x0F, 0x0F]);

const LED_COUNT: usize = 5;

#[derive(Clone)]
pub struct LedPattern {
    time_step: u8,
    pub led_data: [Color; LED_COUNT],
}

impl LedPattern {
    pub fn new(time: u64, led_data: [Color; LED_COUNT]) -> Self {
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
    pub min_repeats: u8,
    index: usize,
}

impl LedAnimation {
    pub fn new(min_repeats: u8) -> Self {
        Self {
            entries: Vec::new(),
            min_repeats,
            index: 0,
        }
    }
    pub fn new_rotation(min_repeats: u8, mut pat: LedPattern) -> Self {
        let mut entries = Vec::new();
        for _ in 0..LED_COUNT {
            entries.push(pat.clone());
            pat.led_data.rotate_right(1);
        }
        Self {
            entries,
            min_repeats,
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
    pub fn get_min_repeats(&self) -> u8 {
        self.min_repeats * (self.entries.len() as u8)
    }
}
