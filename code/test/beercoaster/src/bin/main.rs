#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]
use esp_hal::clock::CpuClock;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::analog::adc;

use esp_radio::ble::controller::BleConnector;
use bt_hci::controller::ExternalController;
use trouble_host::prelude::*;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};

use nb;

use log::info;
use log::error;

use core::option::Option;
use core::option::Option::{None, Some};
use core::result::Result;
use core::result::Result::{Err, Ok};


// mod libs;
// use libs::led_animation;
// use libs::led_animation::{LedAnimation, LedPattern};

use trouble_host::types::uuid::Uuid::Uuid128;
use num_derive::FromPrimitive;
use num_derive::ToPrimitive;
use num_traits::FromPrimitive;
use num_traits::ToPrimitive;

use ws2812_rs;


#[panic_handler]
fn panic(panic_info: &core::panic::PanicInfo) -> ! {
    error!("{}", panic_info);
    loop {}
}

extern crate alloc;

const CONNECTIONS_MAX: usize = 1;
const L2CAP_CHANNELS_MAX: usize = 1;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]


const WEIGHT_EMPTY: u16 = 380;
const WEIGHT_FULL: u16 = 720;
const WEIGHT_TARGET1: u16 = 500;
const LIGHT_LIMIT: f32 = 0.35;

// const MTU_UUID: Uuid128     = Uuid128(0xBBBBBBBB_21C0_46A4_B722_270E3AE3D830.to_be_bytes().);
// const NOTIFY_UUID: Uuid128  = uuid128!("BBD671AA-21C0-46A4-B722-270E3AE3D830");
// const CONTROL_UUID: Uuid128 = uuid128!("7AD671AA-21C0-46A4-B722-270E3AE3D830");
// const WRITE_UUID: Uuid128   = uuid128!("23408888-1F40-4CD8-9B89-CA8D45F8A5B0");
// const COM_UUID: Uuid128     = uuid128!("23408877-1F40-4FD8-9B89-CA9D45F8B5B0");

// const BIER_SERVICE_UUID: Uuid128 = uuid128!("fafafafa-fafa-fafa-fafa-fafafafafafa");


#[derive(Debug, PartialEq, FromPrimitive)]
enum COMState {
    Version = 0x00,
    ADCValue = 0x01,
}

#[derive(Debug, PartialEq, Eq, Hash)]
enum LedState {
    BtWait,
    BtFlashing,
    BtVerified,
    DefaultPattern,
    ActivePattern,
    ErrorPattern,
}

#[derive(ToPrimitive)]
enum OTAControlResponse {
    FlashAck = 0x00,
    FlashNak = 0x01,
    DoneAck = 0x02,
    DoneNak = 0x03,
}

#[derive(FromPrimitive)]
enum OTAControl {
    NOP = 0x00,
    REQUEST = 0x01,
    DONE = 0x02,
    VERIFY = 0x03,
    FLASH = 0x04,
    ABORT = 0x05,
}

struct OTAStateHandle {
    state: OTAState,
}
// from
// https://play.rust-lang.org/?version=stable&mode=debug&edition=2015&gist=ee3e4df093c136ced7b394dc7ffb78e1
#[derive(Debug, PartialEq)]
enum OTAState {
    Initial,
    WaitFlash,
    Failure,
}

#[derive(Debug, Clone)]
enum OTAEvent {
    FlashData,
    DoneFlash,
    Nop,
    Verify,
    Abort,
}

impl OTAStateHandle {
    fn next(&mut self, event: OTAEvent) -> &OTAState {
        match (&self.state, event) {
            (OTAState::Initial, OTAEvent::Abort) => self.state = OTAState::Initial,
            (OTAState::Initial, OTAEvent::Verify) => self.state = OTAState::Initial,
            (OTAState::Initial, OTAEvent::FlashData) => self.state = OTAState::WaitFlash,
            (OTAState::WaitFlash, OTAEvent::DoneFlash) => self.state = OTAState::Initial,
            (OTAState::WaitFlash, OTAEvent::Abort) => self.state = OTAState::Initial,
            (OTAState::WaitFlash, OTAEvent::Nop) => self.state = OTAState::WaitFlash,
            (_s, _e) => self.state = OTAState::Failure,
        }
        &self.state
    }
}

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    // generator version: 1.3.0
    // generator parameters: --chip esp32c3 -o unstable-hal -o embassy -o alloc -o ble-trouble -o wifi -o log -o ci -o vscode -o nightly-x86_64-unknown-linux-gnu -o esp32c3-mini-1

    esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
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
    let sw_interrupt =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);

    info!("Embassy initialized!");

    if let Some(timestamp) = option_env!("VERGEN_BUILD_TIMESTAMP") {
        info!("Build Timestamp: {timestamp}");
    }
    let git_desc = if let Some(describe) = option_env!("VERGEN_GIT_DESCRIBE") {
        info!("git describe: {describe}");
        describe
    } else {
        "NAK"
    }.as_bytes();

    //let (mut brightness_rx, brightness_tx) = single_value_channel::channel_starting_with(1f32);


    let (mut _wifi_controller, _interfaces) =
        esp_radio::wifi::new(peripherals.WIFI, Default::default())
            .expect("Failed to initialize Wi-Fi controller");

    // find more examples https://github.com/embassy-rs/trouble/tree/main/examples/esp32
    let transport = BleConnector::new(peripherals.BT, Default::default()).unwrap();
    let ble_controller = ExternalController::<_, 1>::new(transport);
    let mut resources: HostResources<DefaultPacketPool, CONNECTIONS_MAX, L2CAP_CHANNELS_MAX> =
        HostResources::new();

    let address: Address = Address::random([0xff, 0x8f, 0x1a, 0x05, 0xe4, 0xff]);
    info!("Our address = {:?}", address);
    let _stack = trouble_host::new(ble_controller, &mut resources).set_random_address(address);

    // TODO: Spawn some tasks
    let _ = spawner;


    //slet mut ws2812 = ws2812_rs::WS2812SPI::new(peripherals.GPIO10);

    //ws2812.send_color_w_embassy([ws2812_rs::Color::red(), ws2812_rs::Color::blue()]);

    //   let rainbow = [
    //     led_animation::RED,
    //     led_animation::GREEN,
    //     led_animation::BLUE,
    //     led_animation::CYAN,
    //     led_animation::PINK,
    // ];
    // let rainbow_pat = LedPattern::new(200, rainbow.clone());
    // let default_pattern = LedAnimation::new_rotation(4, rainbow_pat);


    let mut adc1_config = adc::AdcConfig::new();
    let mut pin = adc1_config.enable_pin(peripherals.GPIO4, adc::Attenuation::_11dB);
    let mut adc1 = adc::Adc::new(peripherals.ADC1, adc1_config);

    let mut factor = 1f32;
    loop {
        Timer::after(Duration::from_secs(1)).await;
        let adc_val = nb::block!(adc1.read_oneshot(&mut pin)).unwrap();
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


    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.1.0/examples
}
