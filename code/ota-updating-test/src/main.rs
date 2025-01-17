#![feature(array_repeat)]
use std::array;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::collections::{HashMap, VecDeque};
use log::LevelFilter;

use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::adc::attenuation::DB_11;
use esp_idf_hal::adc::Resolution::Resolution12Bit;
use esp_idf_hal::adc::oneshot::config::AdcChannelConfig;
use esp_idf_hal::adc::oneshot::*;

use ws2812_esp32_rmt_driver::Ws2812Esp32Rmt;

use esp32_nimble::{uuid128, BLEAdvertisementData, BLEDevice, NimbleProperties};
use esp32_nimble::enums::{PowerType, PowerLevel};
use esp32_nimble::utilities::BleUuid;

use num_derive::FromPrimitive;    
use num_traits::FromPrimitive;
use num_derive::ToPrimitive;    
use num_traits::ToPrimitive;

use esp_ota;

pub mod led_animation;
use crate::led_animation::{LedAnimation, LedPattern};



const MTU_UUID: BleUuid     = uuid128!("BBBBBBBB-21C0-46A4-B722-270E3AE3D830");
const NOTIFY_UUID: BleUuid  = uuid128!("BBD671AA-21C0-46A4-B722-270E3AE3D830");
const CONTROL_UUID: BleUuid = uuid128!("7AD671AA-21C0-46A4-B722-270E3AE3D830");
const WRITE_UUID: BleUuid   = uuid128!("23408888-1F40-4CD8-9B89-CA8D45F8A5B0");

const BIER_SERVICE_UUID: BleUuid  = uuid128!("fafafafa-fafa-fafa-fafa-fafafafafafa");

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
enum OTAControl{
  NOP = 0x00,
  REQUEST = 0x01,
  DONE = 0x02,
  VERIFY = 0x03,
  FLASH = 0x04,
  ABORT = 0x05,
}


struct OTAStateHandle{
  state: OTAState
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
    fn next(&mut self, event: OTAEvent) -> &OTAState{
        match (&self.state, event) {
            (OTAState::Initial, OTAEvent::Abort) => {
              self.state = OTAState::Initial
            },
            (OTAState::Initial, OTAEvent::Verify) => {
              self.state = OTAState::Initial
            },
            (OTAState::Initial, OTAEvent::FlashData) => {
              self.state = OTAState::WaitFlash
            },
            (OTAState::WaitFlash, OTAEvent::DoneFlash) => {
              self.state = OTAState::Initial
            },
            (OTAState::WaitFlash, OTAEvent::Abort) => {
              self.state = OTAState::Initial
            },
            (OTAState::WaitFlash, OTAEvent::Nop) => {
              self.state = OTAState::WaitFlash
            },
            (_s, _e) => {
              self.state = OTAState::Failure
            }
        }
        &self.state
    }
}

fn main(){
  // It is necessary to call this function once. Otherwise some patches to the runtime
  // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
  esp_idf_svc::sys::link_patches();
  // Bind the log crate to the ESP Logging facilities
  esp_idf_svc::log::EspLogger::initialize_default();
  if let Err(e) = esp_idf_svc::log::EspLogger.set_target_level("NimBLE", LevelFilter::Error){
    println!("Failed to set log level: {:#?}", e);
  }

  let ota_state = Arc::new(Mutex::new(OTAStateHandle{state: OTAState::Initial}));

  let mut animation_queue = VecDeque::new();
  animation_queue.push_back(LedState::ActivePattern);
  let animation_queue = Arc::new(Mutex::new(animation_queue));

  let (mut brightness_rx, brightness_tx) = single_value_channel::channel_starting_with(-1f32);

  // unsafe{
  //   let mut mac_address = esp_idf_sys::esp_base_mac_addr_get(mac_address);
  // }

  // Finds the next suitable OTA partition and erases it
  let ota = match esp_ota::OtaUpdate::begin(){
    Ok(u) => {
      log::info!("Partition info: {:#?}",u);
      Arc::new(Mutex::new((Some(u), None)))
    },
    Err(e) => {
      log::error!("Failed to find OTA partition: {:#?}", e);
      Arc::new(Mutex::new((None, Some(e.kind()))))
    }
  };

  let ble_device = BLEDevice::take();
  let ble_advertising = ble_device.get_advertising();
  if let Err(e) = ble_device.set_power(PowerType::Default, PowerLevel::P9) {
    log::error!("Failed to set power level: {:#?}", e);
  }

  let server = ble_device.get_server();

  server.on_connect(|server, desc| {
    ::log::info!("Client connected: {:?}", desc.mtu());


    // TODO checkout intervals and power 
    server
      .update_conn_params(desc.conn_handle(), 6, 12, 0, 60)
      .unwrap();

    if server.connected_count() < (esp_idf_svc::sys::CONFIG_BT_NIMBLE_MAX_CONNECTIONS as _) {
      ::log::info!("Multi-connect support: start advertising");
      ble_advertising.lock().start().unwrap();
    }
  });

  server.on_disconnect(|_desc, reason| {
    ::log::info!("Client disconnected ({:?})", reason);
  });
  
  let service = server.create_service(uuid128!("fafafafa-fafa-fafa-fafa-fafafafafafa"));

  // A static characteristic.
  let static_characteristic = service.lock().create_characteristic(
    MTU_UUID,
    NimbleProperties::READ,
  );
  let _ = ble_device.set_preferred_mtu(esp_idf_svc::sys::BLE_ATT_MTU_MAX.try_into().unwrap_or(23u16));

  static_characteristic
    .lock()
    .set_value(&ble_device.get_preferred_mtu().to_le_bytes());

  // A characteristic that notifies every second.
  let notifying_characteristic = service.lock().create_characteristic(
    NOTIFY_UUID,
    NimbleProperties::READ | NimbleProperties::NOTIFY,
  );
  notifying_characteristic.lock().set_value(b"nak");
  
// A control characteristic.
let control_characteristic = service.lock().create_characteristic(
  CONTROL_UUID,
  NimbleProperties::READ | NimbleProperties::WRITE,
);

let ctrl_animation = animation_queue.clone();
let ctrl_state = ota_state.clone();
let notifier = notifying_characteristic.clone();
let ota_fin = ota.clone();
control_characteristic
  .lock()
  .on_read(move |_, _| {
    log::debug!("Read from control characteristic.");
  })
  .on_write(move |args| {
    log::debug!(
      "Wrote to control characteristic: {:?} -> {:?}",
      args.current_data(),
      args.recv_data()
    );

    if let Some(control_value) = args.recv_data().first(){
      if let Some(ctrl) = FromPrimitive::from_u8(*control_value){
        let event = match ctrl{
          OTAControl::REQUEST => {
            OTAEvent::Nop
          },
          OTAControl::VERIFY => {
            let val = match ota_fin.lock().unwrap().1.take(){
              Some(esp_ota::ErrorKind::InvalidRollbackState) => {
                ctrl_animation.lock().unwrap().push_back(LedState::BtVerified);
                esp_ota::mark_app_valid();
                log::debug!("Validated image!");
                // TODO reenable flashing or maybe reboot
                ToPrimitive::to_u8(&OTAControlResponse::DoneAck).unwrap()
              },
              Some(e) => {
                ctrl_animation.lock().unwrap().push_back(LedState::ErrorPattern);
                log::error!("{:#?}", e);
                ToPrimitive::to_u8(&OTAControlResponse::DoneNak).unwrap()
              },
              None => {
                log::debug!("Nothing to verify!");
                ToPrimitive::to_u8(&OTAControlResponse::DoneNak).unwrap()
              }

            };
            notifier.lock().set_value(&[val]).notify();
            //esp_ota::rollback_and_reboot().expect("Failed to roll back to working app");
            OTAEvent::Verify
          },
          OTAControl::FLASH => {
            ctrl_animation.lock().unwrap().push_back(LedState::BtFlashing);
            OTAEvent::FlashData
          },
          OTAControl::ABORT => {
            OTAEvent::Abort
          },
          OTAControl::DONE => {
            log::debug!("OTA flashing done!");
            // Performs validation of the newly written app image and completes the OTA update.
            let opt = ota_fin.lock().unwrap().0.take();
            if let Some(ot) = opt {
              if let Ok(mut completed_ota) = ot.finalize(){
                // Sets the newly written to partition as the next partition to boot from.
                if let Err(e) = completed_ota.set_as_boot_partition(){
                  log::error!("{:#?}",e);
                }else{
                  let val = ToPrimitive::to_u8(&OTAControlResponse::DoneAck).unwrap();
                  notifier.lock().set_value(&[val]).notify();
                  log::debug!("Rebooting!");

                  thread::sleep(Duration::from_millis(4000));
                  // Restarts the CPU, booting into the newly written app.
                  completed_ota.restart();
                }
              }else{
                let val = ToPrimitive::to_u8(&OTAControlResponse::DoneNak).unwrap();
                notifier.lock().set_value(&[val]).notify();
              }
            };
            OTAEvent::DoneFlash
          },
          _ => OTAEvent::Nop
        };
        let mut a = ctrl_state.lock().unwrap();
        log::info!("__ Transition from {:?}", a.state);
        if let OTAState::Failure = a.next(event) {
          log::error!("Failed state!");
          ctrl_animation.lock().unwrap().push_back(LedState::ErrorPattern);
          let val = ToPrimitive::to_u8(&OTAControlResponse::FlashNak).unwrap();
          notifier.lock().set_value(&[val]).notify();
        }else{
          let val = ToPrimitive::to_u8(&OTAControlResponse::FlashAck).unwrap();
          notifier.lock().set_value(&[val]).notify();
        }
        log::info!(" to {:?}", a.state);
      }else{
        log::warn!("Invalid control byte!");
      }
    }else {
      log::warn!("No control byte!");
    }
  });
  
  // A writable characteristic.
  let writable_characteristic = service.lock().create_characteristic(
    WRITE_UUID,
    NimbleProperties::READ | NimbleProperties::WRITE,
  );
  let mut chunk_count = 0;
  let wrt_state: Arc<Mutex<OTAStateHandle>> = ota_state.clone();
  let notifier = notifying_characteristic.clone();
  let ota_chunker = ota.clone();
  writable_characteristic
    .lock()
    .on_read(move |_, _| {
      log::info!("Read from writable characteristic.");
    })
    .on_write( move | args| {
      
      let a = &wrt_state.lock().unwrap();
      match &a.state{
        OTAState::WaitFlash => {

          let app_chunk = args.recv_data();
          if let Some(ref mut ot) = ota_chunker.lock().unwrap().0{
            if let Err(e) = ot.write(app_chunk){
              log::error!("{:#?}",e);
            }
          }else{
            log::error!("How did we end up here?\nTrying to flash after finishing OTA?");
          }
          if chunk_count%10 == 0 {
            let val = ToPrimitive::to_u8(&OTAControlResponse::FlashAck).unwrap();
            notifier.lock().set_value(&[val]).notify();
          }else{
            let val = ToPrimitive::to_u8(&OTAControlResponse::FlashAck).unwrap();
            notifier.lock().set_value(&[val]).notify();
          }
          chunk_count+=1;
          log::info!("Count: {chunk_count}");
        },
        _ => {
          log::info!("Nop");
          let val = ToPrimitive::to_u8(&OTAControlResponse::DoneNak).unwrap();
          notifier.lock().set_value(&[val]).notify();
        }
      }
    });

  let ad_res = ble_advertising.lock().set_data(
    BLEAdvertisementData::new()
      .name("Bierdeckel")
      .add_service_uuid(BIER_SERVICE_UUID),
  );
  if let Err(bleerr) = ad_res {
    loop{
      log::error!("Setting server name failed!\n{bleerr}");
      thread::sleep(Duration::from_millis(500));
    }
  }

  let ad_start_res = ble_advertising.lock().start();
  if let Err(bleerr) = ad_start_res {
    loop{
      log::error!("Failed starting ble!\n{bleerr}");
      thread::sleep(Duration::from_millis(500));
  }
  }

    let peripherals = Peripherals::take().unwrap();
    
    log::info!("Hello, world!");

    let ws2812_pin = peripherals.pins.gpio10;
    let channel = peripherals.rmt.channel0;
    //let mut ws2812 = LedPixelEsp32Rmt::<RGBW8, LedPixelColorGrbw32>::new(channel, ws2812_pin).unwrap();

    let mut ws2812 = Ws2812Esp32Rmt::new(channel, ws2812_pin).unwrap();

    let rainbow = [
      led_animation::RED,
      led_animation::GREEN,
      led_animation::BLUE,
      led_animation::CYAN,
      led_animation::PINK,
    ];
    let rainbow_pat = LedPattern::new(
        100,
        rainbow.clone(),
    );
    let default_pattern = LedAnimation::new_rotation(4, rainbow_pat);
    
    let mut bt_wait = LedAnimation::new(4);
    bt_wait.add_pattern(LedPattern::new(
        800,
        array::repeat(led_animation::BLUE_H),
    ));
    bt_wait.add_pattern(LedPattern::new(
        200,
        array::repeat(led_animation::BLACK),
    ));
    
    let mut active_pattern = LedAnimation::new(4);
    active_pattern.add_pattern(LedPattern::new(
        1500,
        array::repeat(led_animation::GREEN_H),
    ));
    active_pattern.add_pattern(LedPattern::new(
        400,
        array::repeat(led_animation::BLACK),
    ));
    
    let mut bt_flashing = LedAnimation::new(4);
    bt_flashing.add_pattern(LedPattern::new(
        300,
        array::repeat(led_animation::YELLOW_H),
    ));
    bt_flashing.add_pattern(LedPattern::new(
        200,
        array::repeat(led_animation::BLACK),
    ));
    let mut bt_verified = LedAnimation::new(4);
    bt_verified.add_pattern(LedPattern::new(
        300,
        array::repeat(led_animation::PINK_H),
    ));
    bt_verified.add_pattern(LedPattern::new(
        200,
        array::repeat(led_animation::BLACK),
    ));
    let mut error_pattern = LedAnimation::new(4);
    error_pattern.add_pattern(LedPattern::new(
        300,
        array::repeat(led_animation::RED),
    ));
    error_pattern.add_pattern(LedPattern::new(
        200,
        array::repeat(led_animation::BLACK),
    ));
    
    let mut led_map:  HashMap<LedState, LedAnimation> = HashMap::new();
    led_map.insert(LedState::BtWait, bt_wait);
    led_map.insert(LedState::BtFlashing, bt_flashing);
    led_map.insert(LedState::BtVerified, bt_verified);
    led_map.insert(LedState::DefaultPattern, default_pattern);
    led_map.insert(LedState::ActivePattern, active_pattern);
    led_map.insert(LedState::ErrorPattern, error_pattern);

    animation_queue.lock().unwrap().push_back(LedState::BtWait);
    animation_queue.lock().unwrap().push_back(LedState::DefaultPattern);

    let _thread_led = thread::spawn(move || {
      let mut state = LedState::DefaultPattern;
      let mut cycle = 0;
      let mut fact = 1f32;
        loop{
          if let Some(st) = animation_queue.lock().unwrap().pop_front(){
            state = st;
            cycle = 0;
          }

          let led_ani = led_map.get_mut(&state).unwrap();
          loop{
            led_ani.next_pattern().map(|p| {
              fact = *brightness_rx.latest();
              let mut data = p.led_data.clone();
              for da in data.iter_mut(){
                da.r = (da.r as f32* fact ) as u8;
                da.g = (da.g as f32* fact ) as u8;
                da.b = (da.b as f32* fact ) as u8;
              }
              let d = data.iter().copied();
              ws2812.write_nocopy(d).unwrap();
              thread::sleep(Duration::from_millis(p.time_step_ms()));
            });
            if cycle < led_ani.get_min_repeats() {
              cycle += 1;
            }else{
              break;
            }
          }
        }
    });

    let adc = AdcDriver::new(peripherals.adc1).unwrap();

    // configuring pin to analog read, you can regulate the adc input voltage range depending on your need
    // for this example we use the attenuation of 11db which sets the input voltage range to around 0-3.6V
    let config = AdcChannelConfig {
        attenuation: DB_11,
        resolution: Resolution12Bit,
        calibration: false,
    };

    let mut adc_pin = AdcChannelDriver::new(&adc, peripherals.pins.gpio4, &config).unwrap();
    let mut factor = 1f32;
    loop {
      thread::sleep(Duration::from_millis(100));
      let adc_val = adc.read(&mut adc_pin).unwrap();
      let f = (adc_val as f32 / 27.72f32).round() / 100f32; 
      if factor != f{
        factor = f;
        brightness_tx.update(factor).unwrap();
      }
      log::info!("ADC value: {}mV, scale {}", adc_val, factor);
    }
}
