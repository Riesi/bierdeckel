use std::thread;
use std::time::Duration;

use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::gpio::PinDriver;

use smart_leds::hsv::{hsv2rgb, Hsv};
use smart_leds::SmartLedsWrite;
use ws2812_esp32_rmt_driver::Ws2812Esp32Rmt;

#[link_section = ".rtc.data"]
static mut BOOT_COUNTER: u32 = 0;

fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    unsafe {
        BOOT_COUNTER = BOOT_COUNTER+1; 
    }

    let peripherals = Peripherals::take().unwrap();
    let mut led_pin = PinDriver::output(peripherals.pins.gpio2).unwrap();
    
    log::info!("Hello, world!");
    unsafe {
        log::info!("BootCnt: {}, RNG: {}",BOOT_COUNTER, esp_idf_sys::esp_random());
    }

    unsafe {
        log::info!("About to get to sleep now. Will wake up automatically in 2 seconds");
        esp_idf_sys::esp_deep_sleep(Duration::from_secs(2).as_micros() as u64);
    }

    let ws2812_pin = peripherals.pins.gpio15;
    let channel = peripherals.rmt.channel0;
    //let mut ws2812 = LedPixelEsp32Rmt::<RGBW8, LedPixelColorGrbw32>::new(channel, ws2812_pin).unwrap();

    let mut ws2812 = Ws2812Esp32Rmt::new(channel, ws2812_pin).unwrap();

    let mut hue = 20;
    loop {
        let pixels = std::iter::repeat(hsv2rgb(Hsv {
            hue,
            sat: 255,
            val: 255,
        }))
        .take(5);
        ws2812.write(pixels).unwrap();

        thread::sleep(Duration::from_millis(50));

        hue = hue.wrapping_add(3);

        /*led_pin.set_high().unwrap();
        thread::sleep(Duration::from_millis(200));

        led_pin.set_low().unwrap();
        thread::sleep(Duration::from_millis(200));*/
    }
}
