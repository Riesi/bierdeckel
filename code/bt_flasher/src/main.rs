use btleplug::api::{Central, Manager as _, Peripheral, ScanFilter, WriteType};
use btleplug::platform::{Adapter, Manager};
use std::error::Error;
use std::time::Duration;
use tokio::time;
use uuid::{uuid, Uuid};
use futures::stream::StreamExt;
use std::thread;

use num_derive::FromPrimitive;    
use num_traits::FromPrimitive;
use num_derive::ToPrimitive;    
use num_traits::ToPrimitive;

#[derive(FromPrimitive)]
enum OTAControlResponse {
  FLASH_ACK = 0x00,
  FLASH_NAK = 0x01,
  DONE_ACK = 0x02,
  DONE_NAK = 0x03,
}

#[derive(ToPrimitive)]
enum OTAControl{
  NOP = 0x00,
  REQUEST = 0x01,
  DONE = 0x02,
  VERIFY = 0x03,
  FLASH = 0x04,
  ABORT = 0x05,
}


const PERIPHERAL_NAME_MATCH_FILTER: &str = "Bierdeckel";

const MTU_UUID: Uuid     = uuid!("BBBBBBBB-21C0-46A4-B722-270E3AE3D830");
const NOTIFY_UUID: Uuid  = uuid!("BBD671AA-21C0-46A4-B722-270E3AE3D830");
const CONTROL_UUID: Uuid = uuid!("7AD671AA-21C0-46A4-B722-270E3AE3D830");
const WRITE_UUID: Uuid   = uuid!("23408888-1F40-4CD8-9B89-CA8D45F8A5B0");

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let manager = Manager::new().await?;
    let adapter_list = manager.adapters().await?;
    if adapter_list.is_empty() {
        eprintln!("No Bluetooth adapters found");
    }

    for adapter in adapter_list.iter() {
        println!("Starting scan...");
        adapter
            .start_scan(ScanFilter::default())
            .await
            .expect("Can't scan BLE adapter for connected devices...");
        time::sleep(Duration::from_secs(2)).await;
        let peripherals = adapter.peripherals().await?;

        if peripherals.is_empty() {
            eprintln!("->>> BLE peripheral devices were not found, sorry. Exiting...");
        } else {
            // All peripheral devices in range.
            for peripheral in peripherals.iter() {
                let properties = peripheral.properties().await?;
                let is_connected = peripheral.is_connected().await?;
                let local_name = properties
                    .unwrap()
                    .local_name
                    .unwrap_or(String::from("(peripheral name unknown)"));
                println!(
                    "Peripheral {:?} is connected: {:?}",
                    &local_name, is_connected
                );
                // Check if it's the peripheral we want.
                if local_name.contains(PERIPHERAL_NAME_MATCH_FILTER) {
                    println!("Found matching peripheral {:?}...", &local_name);
                    if !is_connected {
                        // Connect if we aren't already connected.
                        if let Err(err) = peripheral.connect().await {
                            eprintln!("Error connecting to peripheral, skipping: {}", err);
                            continue;
                        }
                    }
                    let is_connected = peripheral.is_connected().await?;
                    println!(
                        "Now connected ({:?}) to peripheral {:?}.",
                        is_connected, &local_name
                    );
                    if is_connected {
                        println!("Discover peripheral {:?} services...", local_name);
                        peripheral.discover_services().await?;
                        let chars = peripheral.characteristics();
                        let control_characteristic = chars.iter().find(|c| c.uuid == CONTROL_UUID).unwrap();
                        let data_characteristic = chars.iter().find(|c| c.uuid == WRITE_UUID).unwrap();
                        let mtu_characteristic = chars.iter().find(|c| c.uuid == MTU_UUID).unwrap();
                        let notify_characteristic = chars.iter().find(|c| c.uuid == NOTIFY_UUID).unwrap();

                        peripheral.subscribe(&notify_characteristic).await?;
                        // Print the first 4 notifications received.
                        let mut notification_stream =
                            peripheral.notifications().await?.take(4);

                        // tokio::spawn(async move { 
                        //     if let Some(data) = notification_stream.next().await {
                        //         println!(
                        //             "Received data from {:?} [{:?}]: {:?}",
                        //             local_name, data.uuid, data.value
                        //         );
                        //     }
                        // });    

                        let cmd: u8 = ToPrimitive::to_u8(&OTAControl::ABORT).unwrap();
                        peripheral.write(&control_characteristic, &[cmd], WriteType::WithoutResponse).await?;

                        let mtu = peripheral.read(&mtu_characteristic).await?;
                        let mtu = if let Some(&mt) = mtu.first_chunk::<2>(){
                            u16::from_le_bytes(mt)
                        }else{
                            23
                        };
                        let mtu = 512;
            
                        let cmd = &[ToPrimitive::to_u8(&OTAControl::FLASH).unwrap()];
                        peripheral.write(&control_characteristic, cmd, WriteType::WithoutResponse).await?;

                        if let Some(data) = notification_stream.next().await{
                            if let Some(OTAControlResponse::FLASH_ACK) = FromPrimitive::from_u8(*data.value.first().unwrap()) {
                                println!("Lets go!");
                            }else{
                                panic!("aaaa");
                            }
                        }
                        if let Ok(bin_data) = std::fs::read("ota-updating-test.bin"){
                            let chunks = bin_data.chunks(512);
                            let mut count = 0;
                            println!("Chunk size: {}", chunks.len());
                            for chunk in chunks {
                                peripheral.write(&data_characteristic, chunk, WriteType::WithoutResponse).await?;
                                if let Some(data) = notification_stream.next().await{
                                    count+=1;
                                    println!("Sent data {count}!");
                                    if let Some(OTAControlResponse::FLASH_ACK) = FromPrimitive::from_u8(*data.value.first().unwrap()) {
                                        println!("Data sent!");
                                    }else{
                                        println!("failed to send");
                                    }
                                }
                            }
                        }

                        let cmd = &[ToPrimitive::to_u8(&OTAControl::DONE).unwrap()];
                        peripheral.write(&control_characteristic, cmd, WriteType::WithoutResponse).await?;

                        if let Some(data) = notification_stream.next().await{
                            if let Some(OTAControlResponse::DONE_ACK) = FromPrimitive::from_u8(*data.value.first().unwrap()) {

                            }else{
                                println!("failed to done");
                            }
                        }

                        println!("Disconnecting from peripheral ...");
                        peripheral.disconnect().await?;
                    }
                } else {
                    //println!("Skipping unknown peripheral {:?}", peripheral);
                }
            }
        }
    }

    time::sleep(Duration::from_millis(200)).await;

    Ok(())
}