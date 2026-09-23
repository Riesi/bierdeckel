use core::result::Result::{Err, Ok};

use esp_hal::{Async, analog::adc::{Adc, AdcPin}, peripherals::{ADC1, GPIO4}};
use esp_hal::Blocking;

use embassy_time::{Duration, Timer};




const WEIGHT_EMPTY: u16 = 380;
const WEIGHT_FULL: u16 = 720;
const WEIGHT_TARGET1: u16 = 500;
const LIGHT_LIMIT: f32 = 0.35;

#[embassy_executor::task]
pub async fn adc_task(mut adc1: Adc<'static, ADC1<'static>, Async> , mut pin: AdcPin<GPIO4<'static>, esp_hal::peripherals::ADC1<'static>>) {
    let mut factor = 1f32;
    loop {
        Timer::after(Duration::from_secs(1)).await;
        let adc_val = adc1.read_oneshot(&mut pin).await;
        let f = if adc_val > WEIGHT_FULL {
            1f32
        } else {
            if adc_val > WEIGHT_TARGET1 {
                1f32 - (WEIGHT_FULL - adc_val) as f32
                    / ((WEIGHT_FULL - WEIGHT_TARGET1) as f32 / (1f32 - LIGHT_LIMIT))
            } else {
                if adc_val > WEIGHT_EMPTY {
                    LIGHT_LIMIT
                        - (WEIGHT_TARGET1 - adc_val) as f32
                            / ((WEIGHT_TARGET1 - WEIGHT_EMPTY) as f32 / LIGHT_LIMIT)
                } else {
                    0f32
                }
            }
        };
        if factor != f {
            factor = f;
            //brightness_tx.update(factor).unwrap();
        }
        log::info!("ADC value: {}mV, scale {}", adc_val, factor);
    }
}