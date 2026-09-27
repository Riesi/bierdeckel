use embassy_futures::join::join;
use embassy_futures::select::select;
use embassy_time::Timer;
use esp_radio::ble::controller::BleConnector;
use log::info;
use log::warn;
use trouble_host::prelude::*;

/// Max number of connections
const CONNECTIONS_MAX: usize = 1;

/// Max number of L2CAP channels.
const L2CAP_CHANNELS_MAX: usize = 2; // Signal + att



const PERIPHERAL_NAME: &str = "Bierdeckel";

const MTU_UUID: Uuid     = uuid!("BBBBBBBB-21C0-46A4-B722-270E3AE3D830");
const NOTIFY_UUID: Uuid  = uuid!("BBD671AA-21C0-46A4-B722-270E3AE3D830");
const DESCRIPTION_UUID: Uuid = uuid!("7AD671AA-21C0-46A4-B722-270E3AE3D830");
const WRITE_UUID: Uuid   = uuid!("23408888-1F40-4CD8-9B89-CA8D45F8A5B0");

const BIER_SERVICE_UUID: Uuid  = uuid!("fafafafa-fafa-fafa-fafa-fafafafafafa");


// GATT Server definition
#[gatt_server]
struct Server {
    battery_service: BatteryService,
}

/// Battery service
#[gatt_service(uuid = BIER_SERVICE_UUID)]
struct BatteryService {
    /// Battery Level
    #[descriptor(uuid = NOTIFY_UUID, read, value = [0, 100])]
    #[descriptor(uuid = DESCRIPTION_UUID, name = "hello", read, value = "Battery Level", type = &'static str)]
    #[characteristic(uuid = WRITE_UUID, read, notify, value = 10)]
    level: u8,
    #[characteristic(uuid = MTU_UUID, write, read, notify)]
    status: (),
}

/// Run the BLE stack.
pub async fn run<C>(controller: C)
where
    C: Controller,
{
    // Using a fixed "random" address can be useful for testing. In real scenarios, one would
    // use e.g. the MAC 6 byte array as the address (how to get that varies by the platform).
    let address: Address = Address::random([0xff, 0x8f, 0x1a, 0x05, 0xe4, 0xff]);
    info!("Our address = {:?}", address);

    let mut resources: HostResources<DefaultPacketPool, CONNECTIONS_MAX, L2CAP_CHANNELS_MAX> =
        HostResources::new();
    let stack = trouble_host::new(controller, &mut resources)
        .set_random_address(address)
        .build();
    let runner = stack.runner();
    let mut peripheral = stack.peripheral();

    info!("Starting advertising and GATT service");
    let server = Server::new_with_config(GapConfig::Peripheral(PeripheralConfig {
        name: PERIPHERAL_NAME,
        appearance: &appearance::sensor::GENERIC_SENSOR,
    }))
    .unwrap();

    let _ = join(ble_task_local(runner), async {
        loop {
            match advertise(PERIPHERAL_NAME, &mut peripheral, &server).await {
                Ok(conn) => {
                    // set up tasks when the connection is established to a central, so they don't run when no one is connected.
                    let a = gatt_events_task(&server, &conn);
                    let b = custom_task(&server, &conn, &stack);
                    // run until any task ends (usually because the connection has been closed),
                    // then return to advertising state.
                    select(a, b).await;
                }
                Err(e) => {
                    panic!("[adv] error: {:?}", e);
                }
            }
        }
    })
    .await;
}

/// This is a background task that is required to run forever alongside any other BLE tasks.
///
/// ## Alternative
///
/// If you didn't require this to be generic for your application, you could statically spawn this with i.e.
///
/// ```rust,ignore
///
/// #[embassy_executor::task]
/// async fn ble_task(mut runner: Runner<'static, SoftdeviceController<'static>>) {
///     runner.run().await;
/// }
///
/// spawner.must_spawn(ble_task(runner));
/// ```
async fn ble_task_local<C: Controller, P: PacketPool>(mut runner: Runner<'_, C, P>) {
    loop {
        if let Err(e) = runner.run().await {
            panic!("[ble_task] error: {:?}", e);
        }
    }
}
#[embassy_executor::task]
pub async fn ble_task(controller: ExternalController<BleConnector<'static>, 20>) {
    run(controller).await;
}


/// Stream Events until the connection closes.
///
/// This function will handle the GATT events and process them.
/// This is how we interact with read and write requests.
async fn gatt_events_task<P: PacketPool>(
    server: &Server<'_>,
    conn: &GattConnection<'_, '_, P>,
) -> Result<(), Error> {
    let level = server.battery_service.level;
    let status_handle = server.battery_service.status.handle;
    let mut status = false;
    let reason = loop {
        match conn.next().await {
            GattConnectionEvent::Disconnected { reason } => break reason,
            GattConnectionEvent::Gatt { event } => {
                let reply = match event {
                    GattEvent::Read(event) => {
                        if event.handle() == level.handle {
                            let value = conn.get(&level);
                            info!("[gatt] Read Event to Level Characteristic: {:?}", value);
                            event.accept()
                        } else if event.handle() == status_handle {
                            event.accept_unprocessed(&status)
                        } else {
                            event.accept()
                        }
                    }
                    GattEvent::Write(event) => {
                        if event.handle() == level.handle {
                            event.with_data(|offset, data| {
                                info!(
                                    "[gatt] Write Event to Level Characteristic at {}: {:?}",
                                    offset, data
                                )
                            });
                            event.accept()
                        } else if event.handle() == status_handle {
                            match event.validate(1, 1) {
                                Ok(()) => {
                                    event.with_data(|offset, data| {
                                        if data.len() == 1 {
                                            // If data.len() is 1, offset must be 0 or else validate would have errored
                                            assert!(offset == 0);
                                            status = data[0] != 0;
                                        }
                                    });
                                    event.accept_unprocessed()
                                }
                                Err(err) => event.reject(err),
                            }
                        } else {
                            event.accept()
                        }
                    }
                    _ => event.accept(),
                };
                // This step is also performed at drop(), but writing it explicitly is necessary
                // in order to ensure reply is sent.
                match reply {
                    Ok(reply) => reply.send().await,
                    Err(e) => warn!("[gatt] error sending response: {:?}", e),
                };
            }
            _ => {} // ignore other Gatt Connection Events
        }
    };
    info!("[gatt] disconnected: {:?}", reason);
    Ok(())
}

/// Create an advertiser to use to connect to a BLE Central, and wait for it to connect.
async fn advertise<'values, 'server, C: Controller>(
    name: &'values str,
    peripheral: &mut Peripheral<'values, C, DefaultPacketPool>,
    server: &'server Server<'values>,
) -> Result<GattConnection<'values, 'server, DefaultPacketPool>, BleHostError<C::Error>> {
    let mut advertiser_data = [0; 31];
    let len = AdStructure::encode_slice(
        &[
            AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
            AdStructure::IncompleteServiceUuids16(&[[0x0f, 0x18]]),
            AdStructure::CompleteLocalName(name.as_bytes()),
        ],
        &mut advertiser_data[..],
    )?;
    let advertiser = peripheral
        .advertise(
            &Default::default(),
            Advertisement::ConnectableScannableUndirected {
                adv_data: &advertiser_data[..len],
                scan_data: &[],
            },
        )
        .await?;
    info!("[adv] advertising");
    let conn = advertiser.accept().await?.with_attribute_server(server)?;
    info!("[adv] connection established");
    Ok(conn)
}

/// Example task to use the BLE notifier interface.
/// This task will notify the connected central of a counter value every 2 seconds.
/// It will also read the RSSI value every 2 seconds.
/// and will stop when the connection is closed by the central or an error occurs.
async fn custom_task<C: Controller, P: PacketPool>(
    server: &Server<'_>,
    conn: &GattConnection<'_, '_, P>,
    stack: &Stack<'_, C, P>,
) {
    let mut tick: u8 = 0;
    let level = server.battery_service.level;
    loop {
        tick = tick.wrapping_add(1);
        info!("[custom_task] notifying connection of tick {}", tick);
        if level.notify(conn, &tick, true).await.is_err() {
            info!("[custom_task] error notifying connection");
            break;
        };
        // read RSSI (Received Signal Strength Indicator) of the connection.
        if let Ok(rssi) = conn.raw().rssi(stack).await {
            info!("[custom_task] RSSI: {:?}", rssi);
        } else {
            info!("[custom_task] error getting RSSI");
            break;
        };
        Timer::after_secs(2).await;
    }
}
