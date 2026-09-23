#![allow(unused)]
#[warn(dead_code)]
use esp_hal::Async;
use esp_hal_smartled::{RmtSmartLeds, color_order};
use log::info;
use smart_leds::RGB;
use smart_leds::SmartLedsWriteAsync;

use core::clone::Clone;
use core::cmp::Ord;
use core::derive;
use core::option::Option;
use core::option::Option::{None, Some};
use smart_leds::RGB8;
extern crate alloc;
use alloc::vec::Vec;

pub const RED: RGB8 = RGB8::new(0xFF, 0, 0);
pub const GREEN: RGB8 = RGB8::new(0, 0xFF, 0);
pub const BLUE: RGB8 = RGB8::new(0, 0, 0xFF);

pub const WHITE: RGB8 = RGB8::new(0xFF, 0xFF, 0xFF);
pub const BLACK: RGB8 = RGB8::new(0, 0, 0);

pub const YELLOW: RGB8 = RGB8::new(0xFF, 0xFF, 0);
pub const PINK: RGB8 = RGB8::new(0xFF, 0, 0xFF);
pub const CYAN: RGB8 = RGB8::new(0, 0xFF, 0xFF);

pub const RED_H: RGB8 = RGB8::new(0x0F, 0, 0);
pub const GREEN_H: RGB8 = RGB8::new(0, 0x0F, 0);
pub const BLUE_H: RGB8 = RGB8::new(0, 0, 0x0F);
pub const WHITE_H: RGB8 = RGB8::new(0x0F, 0x0F, 0x0F);

pub const YELLOW_H: RGB8 = RGB8::new(0, 0, 0);
pub const PINK_H: RGB8 = RGB8::new(0x0F, 0, 0x0F);
pub const CYAN_H: RGB8 = RGB8::new(0, 0x0F, 0x0F);

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
        let ret = self.entries.get(self.index).map(|pat| pat.clone());
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

// Declare async tasks
#[embassy_executor::task]
pub async fn smart_led_task(mut led: RmtSmartLeds<'static, 122, Async, RGB<u8>, color_order::Grb>) {
    info!("LED thread!");
    let delay = esp_hal::delay::Delay::new();
    let mut color = smart_leds::hsv::Hsv {
        hue: 0,
        sat: 255,
        val: 255,
    };
    let mut data;
    loop {
        // Iterate over the rainbow!
        for hue in 0..=255 {
            color.hue = hue;
            // Convert from the HSV color space (where we can easily transition from one
            // color to the other) to the RGB color space that we can then send to the LED
            data = [smart_leds::hsv::hsv2rgb(color); crate::LEDS]; // smart_leds::hsv::hsv2rgb(color)
            // When sending to the LED, we do a gamma correction first (see smart_leds
            // documentation for details) and then limit the brightness to 10 out of 255 so
            // that the output it's not too bright.
            let ret = led
                .write(smart_leds::brightness(
                    smart_leds::gamma(data.iter().cloned()),
                    10,
                ))
                .await;
            if let Err(e) = ret {
                info!("{:#?}", e);
            }
            delay.delay_millis(20);
        }
    }
}
