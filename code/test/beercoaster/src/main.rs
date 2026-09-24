#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]
use esp_hal::analog::adc;
use esp_hal::clock::CpuClock;
use esp_hal::timer::timg::TimerGroup;

use bt_hci::controller::ExternalController;
use esp_radio::ble::controller::BleConnector;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};

use log::error;
use log::info;

use core::option::Option::Some;

mod utils;
use utils::led_animation;

// use num_derive::FromPrimitive;
// use num_traits::FromPrimitive;

use embassy_sync::signal::Signal;

use crate::utils::adc_readout;


#[panic_handler]
fn panic(panic_info: &core::panic::PanicInfo) -> ! {
    error!("{}", panic_info);
    loop {}
}

extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
// const MTU_UUID: Uuid128     = Uuid128(0xBBBBBBBB_21C0_46A4_B722_270E3AE3D830.to_be_bytes().);
// const NOTIFY_UUID: Uuid128  = uuid128!("BBD671AA-21C0-46A4-B722-270E3AE3D830");
// const CONTROL_UUID: Uuid128 = uuid128!("7AD671AA-21C0-46A4-B722-270E3AE3D830");
// const WRITE_UUID: Uuid128   = uuid128!("23408888-1F40-4CD8-9B89-CA8D45F8A5B0");
// const COM_UUID: Uuid128     = uuid128!("23408877-1F40-4FD8-9B89-CA9D45F8B5B0");

// const BIER_SERVICE_UUID: Uuid128 = uuid128!("fafafafa-fafa-fafa-fafa-fafafafafafa");
// #[derive(Debug, PartialEq, FromPrimitive)]
// enum COMState {
//     Version = 0x00,
//     ADCValue = 0x01,
// }

// #[derive(Debug, PartialEq, Eq, Hash)]
// enum LedState {
//     BtWait,
//     BtFlashing,
//     BtVerified,
//     DefaultPattern,
//     ActivePattern,
//     ErrorPattern,
// }

const LEDS: usize = 5;

static ADC_VALUE_SIGNAL: Signal<embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, u16>  = Signal::new();

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    // generator version: 1.3.0
    // generator parameters: --chip esp32c3 -o unstable-hal -o embassy -o alloc -o ble-trouble -o wifi -o log -o ci -o vscode -o nightly-x86_64-unknown-linux-gnu -o esp32c3-mini-1

    esp_println::logger::init_logger_from_env();
    let speed = CpuClock::_80MHz;
    let config = esp_hal::Config::default().with_cpu_clock(speed);
    let peripherals = esp_hal::init(config);

    // The following pins are used to bootstrap the chip. They are available
    // for use, but check the datasheet of the module for more information on them.
    // - GPIO2
    // - GPIO8
    // - GPIO9
    // These GPIO pins are in use by some feature of the module and should not be used.
    let _ = peripherals.GPIO11;
    let _ = peripherals.GPIO12;
    let _ = peripherals.GPIO13;
    let _ = peripherals.GPIO14;
    let _ = peripherals.GPIO15;
    let _ = peripherals.GPIO16;
    let _ = peripherals.GPIO17;

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 66320);
    // COEX needs more RAM - so we've added some more
    esp_alloc::heap_allocator!(size: 64 * 1024);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, peripherals.FROM_CPU_INTR0);

    info!("Embassy initialized!");

    if let Some(timestamp) = option_env!("VERGEN_BUILD_TIMESTAMP") {
        info!("Build Timestamp: {timestamp}");
    }
    let _git_desc = if let Some(describe) = option_env!("VERGEN_GIT_DESCRIBE") {
        info!("git describe: {describe}");
        describe
    } else {
        "NAK"
    }
    .as_bytes();

    // let (mut _wifi_controller, _interfaces) =
    //     esp_radio::wifi::new(peripherals.WIFI, Default::default())
    //         .expect("Failed to initialize Wi-Fi controller");

    // find more examples https://github.com/embassy-rs/trouble/tree/main/examples/esp32
    let bluetooth = peripherals.BT;
    let connector = BleConnector::new(bluetooth, Default::default()).unwrap();
    let controller: ExternalController<_, 20> = ExternalController::new(connector);

    utils::ble_bas_peripheral::run(controller).await;
    // spawner.spawn(ble_task());

    info!("init WS2812 RMT hardware");
    let led_pin = peripherals.GPIO8;
    type LedColor = smart_leds::RGB8;
    let led = {
        let freq = esp_hal::time::Rate::from_mhz(80);
        let rmt = esp_hal::rmt::Rmt::new(peripherals.RMT, freq)
            .expect("Failed to initialize RMT0")
            .into_async();
        // Configure color order and timing implementation as needed.
        esp_hal_smartled::RmtSmartLeds::<
            { esp_hal_smartled::buffer_size::<LedColor>(LEDS) },
            _,
            LedColor,
            esp_hal_smartled::color_order::Grb,
        >::new_with_memsize(
            esp_hal_smartled::WS2812B_TIMING,
            rmt.channel0,
            led_pin,
            2,
            freq,
        )
        .unwrap()
    };
    spawner.spawn(led_animation::smart_led_task(led).unwrap());

    // // Asynchronously drive the pin signals
    // strip.async_send_color(colors).await;
    //   let rainbow = [
    //     led_animation::RED,
    //     led_animation::GREEN,
    //     led_animation::BLUE,
    //     led_animation::CYAN,
    //     led_animation::PINK,
    // ];
    // let rainbow_pat = LedPattern::new(200, rainbow.clone());
    // let default_pattern = LedAnimation::new_rotation(4, rainbow_pat);

    info!("init adc task");
    let mut adc1_config = adc::AdcConfig::new();
    let pin = adc1_config.enable_pin(peripherals.GPIO4, adc::Attenuation::_11dB);
    let adc1 = adc::Adc::new(peripherals.ADC1, adc1_config).into_async();
    spawner.spawn(adc_readout::adc_task(adc1, pin).unwrap());

    loop {
        if let Some(adc_val) = ADC_VALUE_SIGNAL.try_take(){
            log::info!("FROG ADC value: {}mV", adc_val);
        }
        Timer::after(Duration::from_secs(1)).await;
    }
    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.1.0/examples
}
