use btleplug::api::{Central, CentralEvent, Manager as _, Peripheral, ScanFilter, WriteType};
use btleplug::platform::{Adapter, Manager};
use std::error::Error;
use tokio::time;
use uuid::{uuid, Uuid};
use futures::stream::StreamExt;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const PERIPHERAL_NAME_MATCH_FILTER: &str = "Bierdeckel";

const MTU_UUID: Uuid     = uuid!("BBBBBBBB-21C0-46A4-B722-270E3AE3D830");
// const NOTIFY_UUID: Uuid  = uuid!("BBD671AA-21C0-46A4-B722-270E3AE3D830");
// const CONTROL_UUID: Uuid = uuid!("7AD671AA-21C0-46A4-B722-270E3AE3D830");
// const WRITE_UUID: Uuid   = uuid!("23408888-1F40-4CD8-9B89-CA8D45F8A5B0");

const DESCRIPTION_UUID: Uuid = uuid!("7AD671AA-21C0-46A4-B722-270E3AE3D830");
const BIER_SERVICE_UUID: Uuid  = uuid!("fafafafa-fafa-fafa-fafa-fafafafafafa");

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

    Ok(())
}
async fn scan(adapter_list: &Vec<Adapter>, verify: bool) -> Result<(), ()> {
    for adapter in adapter_list.iter() {
        println!("Starting scan...");

        // if let Err(e) = adapter.stop_scan().await{
        //     println!("Stop: {:?}",e );
        // }

        let mut event_stream = adapter.events().await.unwrap();
        let filter = ScanFilter{services: vec![BIER_SERVICE_UUID]};
        adapter
            .start_scan(filter)
            .await
            .expect("Can't scan BLE adapter for connected devices...");

        while let Some(event) = event_stream.next().await {
            match event {
                CentralEvent::DeviceDiscovered(id) => {
                    let peripheral = adapter.peripheral(&id).await.unwrap();
                    let properties = peripheral.properties().await.unwrap();
                    let name = properties
                        .and_then(|p| p.local_name)
                        .unwrap_or_default();
                    if name.eq(PERIPHERAL_NAME_MATCH_FILTER) {
                        println!("Connecting to Bierdeckel: {:?}", id);
                        let is_connected = peripheral.is_connected().await.unwrap();
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
                    let peripheral = adapter.peripheral(&id).await.unwrap();
                    let is_connected = peripheral.is_connected().await.unwrap();
                    let properties = peripheral.properties().await.unwrap();
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

                    println!("Flashing image");
                    flash_firmware(peripheral).await.unwrap();

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

async fn flash_firmware(peripheral: impl Peripheral) -> Result<(), ()> {

    println!("Discover peripheral services...");
    peripheral.discover_services().await.unwrap();
    let chars = peripheral.characteristics();

    println!("car {:#?}",chars);
    let mtu_characteristic = chars.iter().find(|c| c.uuid == MTU_UUID).unwrap();

    peripheral.subscribe(&mtu_characteristic).await.unwrap();
    // Print the first 4 notifications received.
    let mut notification_stream =
        peripheral.notifications().await.unwrap();


    let mtu = peripheral.read(&mtu_characteristic).await.unwrap();
    println!("frog {:#?}",mtu);
    let mtu = if let Some(&mt) = mtu.first_chunk::<2>(){
        u16::from_le_bytes(mt)
    }else{
        23
    };
    let mtu = 512;

    if let Some(data) = notification_stream.next().await{

    println!("toad {:#?}",data);
    }

    println!("Disconnecting from peripheral ...");
    peripheral.disconnect().await.unwrap();

    Ok(())
}
