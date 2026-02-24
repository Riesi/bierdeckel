use btleplug::api::{Central, CentralEvent, Peripheral, ScanFilter, WriteType};
use btleplug::platform::Adapter;

use uuid::{uuid, Uuid};
use futures::stream::StreamExt;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use num_derive::FromPrimitive;    
use num_traits::FromPrimitive;
use num_derive::ToPrimitive;    
use num_traits::ToPrimitive;

#[derive(Debug, PartialEq, FromPrimitive, ToPrimitive)]
enum COMState {
    Version = 0x00,
    ADCValue = 0x01,
}

#[derive(FromPrimitive,Debug)]
pub enum OTAControlResponse {
  FLASH_ACK = 0x00,
  FLASH_NAK = 0x01,
  DONE_ACK = 0x02,
  DONE_NAK = 0x03,
}

#[derive(ToPrimitive)]
pub enum OTAControl{
  NOP = 0x00,
  REQUEST = 0x01,
  DONE = 0x02,
  VERIFY = 0x03,
  FLASH = 0x04,
  ABORT = 0x05,
}


pub const PERIPHERAL_NAME_MATCH_FILTER: &str = "Bierdeckel";

pub const MTU_UUID: Uuid     = uuid!("BBBBBBBB-21C0-46A4-B722-270E3AE3D830");
pub const NOTIFY_UUID: Uuid  = uuid!("BBD671AA-21C0-46A4-B722-270E3AE3D830");
pub const CONTROL_UUID: Uuid = uuid!("7AD671AA-21C0-46A4-B722-270E3AE3D830");
pub const WRITE_UUID: Uuid   = uuid!("23408888-1F40-4CD8-9B89-CA8D45F8A5B0");
pub const COM_UUID: Uuid     = uuid!("23408877-1F40-4FD8-9B89-CA9D45F8B5B0");

pub const BIER_SERVICE_UUID: Uuid  = uuid!("fafafafa-fafa-fafa-fafa-fafafafafafa");
/*
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let manager = Manager::new().await?;
    let adapter_list = manager.adapters().await?;
    if adapter_list.is_empty() {
        eprintln!("No Bluetooth adapters found");
    }
    // Flash
    scan(&adapter_list, false).await.unwrap();

    // Delay to prevent 
    time::sleep(Duration::from_millis(4000)).await;

    // Verify
    scan(&adapter_list, true).await.unwrap();

    Ok(())
}*/

pub async fn scan(adapter_list: &Vec<Adapter>, verify: bool, file: &rfd::FileHandle) -> Result<(), ()> {
    for adapter in adapter_list.iter() {
        println!("Starting scan...");

        if let Err(_) = adapter.stop_scan().await {
            println!("Stopping failed?!");
        }

        let mut event_stream = adapter.events().await.expect("Getting events failed!");

        let filter = ScanFilter{services: vec![BIER_SERVICE_UUID]};
        adapter
            .start_scan(filter)
            .await
            .expect("Can't scan BLE adapter for connected devices...");

        while let Some(event) = event_stream.next().await {
            match event {
                CentralEvent::DeviceDiscovered(id) => {
                    let peripheral = match adapter.peripheral(&id).await {
                        Err(e) => {
                            eprintln!("{:?}",e );
                            return Result::Err(());
                        }
                        Ok(res) => {res}
                    };
                    let properties = match peripheral.properties().await {
                        Err(e) => {
                            eprintln!("{:?}",e );
                            return Result::Err(());
                        }
                        Ok(res) => {res}
                    };
                    let name = properties
                        .and_then(|p| p.local_name)
                        .unwrap_or_default();
                    if name.eq(PERIPHERAL_NAME_MATCH_FILTER) {
                        println!("Connecting to Bierdeckel: {:?}", id);
                        let is_connected = match peripheral.is_connected().await {
                            Err(e) => {
                                eprintln!("{:?}",e );
                                return Result::Err(());
                            }
                            Ok(res) => {res}
                        };
                        if !is_connected {
                            // Connect if we aren't already connected.
                            if let Err(err) = peripheral.connect().await {
                                eprintln!("Error connecting to peripheral, skipping: {}", err);
                            }
                        }
                    }else{
                        println!("Ignoring Device: {:?}, Name: {}", id, name);
                    }
                }
                CentralEvent::StateUpdate(state) => {
                    println!("AdapterStatusUpdate {:?}", state);
                }
                CentralEvent::DeviceConnected(id) => {
                    println!("DeviceConnected: {:?}", id);
                    let peripheral = match adapter.peripheral(&id).await {
                        Err(e) => {
                            eprintln!("{:?}",e );
                            return Result::Err(());
                        }
                        Ok(res) => {res}
                    };
                    let is_connected = match peripheral.is_connected().await {
                        Err(e) => {
                            eprintln!("{:?}",e );
                            return Result::Err(());
                        }
                        Ok(res) => {res}
                    };
                    let properties = match peripheral.properties().await {
                        Err(e) => {
                            eprintln!("{:?}",e );
                            return Result::Err(());
                        }
                        Ok(res) => {res}
                    };
                    let local_name = properties
                        .unwrap()
                        .local_name
                        .unwrap_or(String::from("(peripheral name unknown)"));
                    println!(
                        "Peripheral {:?} is connected: {:?}",
                        &local_name, is_connected
                    );
                    if !is_connected {
                        return Err(());
                    }
                    if verify{
                        println!("Validating image");
                        validate_firmware(peripheral).await.unwrap();
                    }else{
                        println!("Flashing image");
                        flash_firmware(peripheral, &file).await.unwrap();
                    }

                    if let Err(e) = adapter.stop_scan().await{
                        println!("{:?}",e );
                    }
                    break;
                }
                CentralEvent::DeviceDisconnected(id) => {
                    println!("DeviceDisconnected: {:?}", id);
                }
                CentralEvent::ManufacturerDataAdvertisement {
                    id,
                    manufacturer_data,
                } => {
                    println!(
                        "ManufacturerDataAdvertisement: {:?}, {:?}",
                        id, manufacturer_data
                    );
                }
                CentralEvent::ServiceDataAdvertisement { id, service_data } => {
                    println!("ServiceDataAdvertisement: {:?}, {:?}", id, service_data);
                }
                CentralEvent::ServicesAdvertisement { id, services } => {
                    let services: Vec<String> =
                        services.into_iter().map(|s| s.to_string()).collect();
                    println!("ServicesAdvertisement: {:?}, {:?}", id, services);
                }
                _ => {}
            }
        }
    }
    Ok(())
}

pub async fn flash_firmware(peripheral: impl Peripheral, file: &rfd::FileHandle) -> Result<(), ()> {

    println!("Discover peripheral services...");
    peripheral.discover_services().await.unwrap();
    let chars = peripheral.characteristics();
    let control_characteristic = chars.iter().find(|c| c.uuid == CONTROL_UUID).unwrap();
    let data_characteristic = chars.iter().find(|c| c.uuid == WRITE_UUID).unwrap();
    let mtu_characteristic = chars.iter().find(|c| c.uuid == MTU_UUID).unwrap();
    let notify_characteristic = chars.iter().find(|c| c.uuid == NOTIFY_UUID).unwrap();
    let com_characteristic = chars.iter().find(|c| c.uuid == COM_UUID).unwrap();

    peripheral.subscribe(&notify_characteristic).await.unwrap();
    // Print the first 4 notifications received.
    let mut notification_stream =
        peripheral.notifications().await.unwrap();

    let cmd: u8 = ToPrimitive::to_u8(&COMState::Version).unwrap();
    peripheral.write(&com_characteristic, &[cmd], WriteType::WithoutResponse).await.expect("Version command failed!");
    let cmd: u8 = ToPrimitive::to_u8(&OTAControl::ABORT).unwrap();
    peripheral.write(&control_characteristic, &[cmd], WriteType::WithoutResponse).await.unwrap();



    if let Some(data) = notification_stream.next().await{
        let version_desc = str::from_utf8(&data.value).unwrap_or("NOPE!");
        println!("Version: {}", version_desc);
    }

    let mtu = peripheral.read(&mtu_characteristic).await.unwrap();
    let mtu = if let Some(&mt) = mtu.first_chunk::<2>(){
        u16::from_le_bytes(mt)
    }else{
        23
    };
    let mtu = 512;

    let cmd = &[ToPrimitive::to_u8(&OTAControl::FLASH).unwrap()];
    peripheral.write(&control_characteristic, cmd, WriteType::WithoutResponse).await.unwrap();

    if let Some(data) = notification_stream.next().await{
        if let Some(OTAControlResponse::FLASH_ACK) = FromPrimitive::from_u8(*data.value.first().unwrap()) {
            println!("Lets go!");
        }else{
            panic!("No ACK received! Failed flash!");
        }
    }
    let start_flash = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards");
    let flash_file = file.path().display().to_string();//"beercoaster.bin";
    //let flash_file = "test.bin";
    //let flash_file = "test_big.bin";
    const CHUNK_SIZE: usize = 512;
    if let Ok(bin_data) = std::fs::read(flash_file){
        let chunks = bin_data.chunks(CHUNK_SIZE);
        let mut count:f32 = 0f32;
        let chunks_size = chunks.len() as f32;
        println!("Chunk size: {}", chunks_size);
        for chunk in chunks {
            let since_the_epoch = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards");
            let time_diff = since_the_epoch.as_millis()-start_flash.as_millis();
            let speed = (CHUNK_SIZE as f32)*count/time_diff.to_f32().unwrap()*(1000f32/1024f32);
            println!("Elapsed: {:.1}s, Speed: {:.1}kB/s",time_diff as f32/1000f32, speed);
            peripheral.write(&data_characteristic, chunk, WriteType::WithoutResponse).await.unwrap();
            if let Some(data) = notification_stream.next().await{
                count+=1f32;
                let progress = count/chunks_size*100f32;
                println!("Sent data {:.1}%, {count}/{chunks_size}chunks!", progress);
                if let Some(x) = FromPrimitive::from_u8(*data.value.first().unwrap()){
                    match x {
                        OTAControlResponse::FLASH_ACK => (),
                        OTAControlResponse::FLASH_NAK => println!("Failed to send!"),
                        _=> println!("Unknown error! {:#?}", x),
                    }
                }else{
                    println!("Nothing received!")
                }
            }
        }
    }
    let end_flash = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards");
    println!("End: {:?}",end_flash.as_millis()%1000);
    println!("Took: {:?}",end_flash.as_millis()-start_flash.as_millis());
    let cmd = &[ToPrimitive::to_u8(&OTAControl::DONE).unwrap()];
    peripheral.write(&control_characteristic, cmd, WriteType::WithoutResponse).await.unwrap();
    loop {
        if let Some(data) = notification_stream.next().await{
            if let Some(x) = FromPrimitive::from_u8(*data.value.first().unwrap()){
                match x {
                    OTAControlResponse::FLASH_ACK => println!("Data sent!"),
                    OTAControlResponse::FLASH_NAK => println!("Failed to send!"),
                    OTAControlResponse::DONE_ACK => {
                        println!("Done flash!");
                        break;
                    },
                    OTAControlResponse::DONE_NAK => {
                        println!("Faile done!");
                        break;
                },
                    _=> println!("Unknown error! {:#?}", x),
                }
            }
        }
    }

    println!("Disconnecting from peripheral ...");
    peripheral.disconnect().await.unwrap();

    Ok(())
}

pub async fn validate_firmware(peripheral: impl Peripheral) -> Result<(), ()> {
    println!("Discover peripheral services...");
    peripheral.discover_services().await.unwrap();
    let chars = peripheral.characteristics();
    let control_characteristic = chars.iter().find(|c| c.uuid == CONTROL_UUID).unwrap();
    let notify_characteristic = chars.iter().find(|c| c.uuid == NOTIFY_UUID).unwrap();

    peripheral.subscribe(&notify_characteristic).await.unwrap();
    // Print the first 4 notifications received.
    let mut notification_stream =
        peripheral.notifications().await.unwrap();

    let cmd: u8 = ToPrimitive::to_u8(&OTAControl::VERIFY).unwrap();
    peripheral.write(&control_characteristic, &[cmd], WriteType::WithoutResponse).await.unwrap();
    if let Some(data) = notification_stream.next().await{
        if let Some(x) = FromPrimitive::from_u8(*data.value.first().unwrap()){
            match x {
                OTAControlResponse::DONE_ACK => {
                    println!("Verification done!");

                },
                OTAControlResponse::DONE_NAK => {
                    println!("Failed  verification!");
            },
                _=> println!("Unknown error! {:#?}", x),
            }
        }
    }

    println!("Disconnecting from peripheral ...");
    peripheral.disconnect().await.unwrap();

    Ok(())
}
