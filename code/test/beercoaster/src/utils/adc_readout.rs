use esp_hal::{
    Async,
    analog::adc::{Adc, AdcPin},
    peripherals::{ADC1, GPIO4},
};

use embassy_time::{Duration, Timer};

#[embassy_executor::task]
pub async fn adc_task(
    mut adc1: Adc<'static, ADC1<'static>, Async>,
    mut pin: AdcPin<GPIO4<'static>, esp_hal::peripherals::ADC1<'static>>,
) {
    log::info!("ADC thread!");
    let mut factor = 1f32;

    let mut adc_val;
    let mut adc_val_prev = 0u16;
    let mut adc_val_diff;
    let mut adc_val_ma = 0f32;
    let mut cnt = 0;
    let mut taps = 0u16;
    const TAP_STEP: f32 = 20.0; 
    const STEP_SIZE_MS: u64 = 10;
    const MA_WINDOW: f32 = 10.0;
    loop {
        Timer::after(Duration::from_millis(STEP_SIZE_MS)).await;
        adc_val = adc1.read_oneshot(&mut pin).await;

        adc_val_ma = adc_val_ma + ((adc_val as f32 - adc_val_ma) )/(MA_WINDOW+1.0);
        adc_val_diff = (adc_val as f32 - adc_val_prev as f32).abs() / STEP_SIZE_MS as f32; // backwards differential
        adc_val_prev = adc_val;

        crate::ADC_VALUE_SIGNAL.signal(adc_val);
        if adc_val_diff > TAP_STEP {
            taps += 1;
        }
        if cnt >10 {
            // log::info!("ADC value: {}mV", adc_val);
            // log::info!("ADC value avg: {}mV", adc_val_ma);
            // log::info!("ADC value diff: {}mV/s", adc_val_diff);

            crate::TAP_VALUE_SIGNAL.signal(taps);
            taps = taps.saturating_sub(2);

            cnt=0;
        }
        cnt = cnt +1;
    }


// const WEIGHT_EMPTY: u16 = 380;
// const WEIGHT_FULL: u16 = 720;
// const WEIGHT_TARGET1: u16 = 500;
// const LIGHT_LIMIT: f32 = 0.35;
    // let f = if adc_val > WEIGHT_FULL {
    //         1f32
    //     } else {
    //         if adc_val > WEIGHT_TARGET1 {
    //             1f32 - (WEIGHT_FULL - adc_val) as f32
    //                 / ((WEIGHT_FULL - WEIGHT_TARGET1) as f32 / (1f32 - LIGHT_LIMIT))
    //         } else {
    //             if adc_val > WEIGHT_EMPTY {
    //                 LIGHT_LIMIT
    //                     - (WEIGHT_TARGET1 - adc_val) as f32
    //                         / ((WEIGHT_TARGET1 - WEIGHT_EMPTY) as f32 / LIGHT_LIMIT)
    //             } else {
    //                 0f32
    //             }
    //         }
    //     };
    //     if factor != f {
    //         factor = f;
    //         //brightness_tx.update(factor).unwrap();
    //     }
}
