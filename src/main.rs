use serde::{Deserialize, Serialize};
use tracing::info;
use device::manager::{CreateStruct, DeviceRequestStruct, UuidWrapper};
use serde_json::json;
use uuid::Uuid;


#[macro_use]
extern crate lazy_static;

mod cli;
/// The Device module consists of two main modules: devices and manager.
mod device;
mod logger;
mod server;

use schemars::{schema_for, JsonSchema};

#[derive(JsonSchema)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "module")]
pub enum ModuleType {
    DeviceManager(device::manager::Request),
}

#[tokio::main]
async fn main() {
    // CLI should be started before logger to allow control over verbosity
    cli::manager::init();
    // Logger should start before everything else to register any log information
    logger::manager::init();

    let (mut manager, handler) = device::manager::DeviceManager::new(10);

    //Todo: Load previous devices
    if cli::manager::is_enable_auto_create() {
        match manager.auto_create().await {
            Ok(answer) => info!("DeviceManager initialized with following devices: {answer:?}"),
            Err(err) => info!("DeviceManager unable to initialize with devices, details {err:?}"),
        }
    }

    let schema = schema_for!(ModuleType);
    println!("{}", serde_json::to_string_pretty(&schema).unwrap());


    let requests = vec![
        crate::device::manager::Request::Ping(DeviceRequestStruct {
            uuid: Uuid::parse_str("00000000-0000-0000-001e-10da679f8cee").unwrap(),
            device_request: crate::device::devices::PingRequest::Ping360(
                crate::device::devices::Ping360Request::Transducer(
                    bluerobotics_ping::ping360::TransducerStruct {
                        mode: 1,
                        gain_setting: 2,
                        angle: 0,
                        transmit_duration: 500,
                        sample_period: 80,
                        transmit_frequency: 700,
                        number_of_samples: 1200,
                        transmit: 1,
                        reserved: 1,
                    },
                ),
            ),
        }),
        crate::device::manager::Request::Ping(DeviceRequestStruct {
            uuid: Uuid::parse_str("00000000-0000-0000-001e-10da679f8cee").unwrap(),
            device_request: crate::device::devices::PingRequest::Ping1D(
                crate::device::devices::Ping1DRequest::Voltage5
            ),
        }),
        crate::device::manager::Request::EnableContinuousMode(UuidWrapper {
            uuid: Uuid::parse_str("00000000-0000-0000-001e-10da679f8cee").unwrap(),
        }),
        crate::device::manager::Request::List,
        crate::device::manager::Request::Create(CreateStruct {
            source: device::manager::SourceSelection::SerialStream(
                device::manager::SourceSerialStruct {
                    path: "/dev/ttyUSB0".to_string(),
                    baudrate: 115200,
                },
            ),
            device_selection: device::manager::DeviceSelection::Auto,
        }),
        crate::device::manager::Request::Create(CreateStruct {
            source: device::manager::SourceSelection::UdpStream(device::manager::SourceUdpStruct {
                ip: "192.168.0.1".parse().unwrap(),
                port: 9092,
            }),
            device_selection: device::manager::DeviceSelection::Auto,
        }),
    ];

    // Print each request as JSON
    for request in requests {
        // println!("{}", json!(Command{module : ModuleType::DeviceManager(request)}));
        println!("{}", json!(ModuleType::DeviceManager(request)));
    }

    println!("{}", json!(        crate::device::manager::Request::Create(CreateStruct {
        source: device::manager::SourceSelection::UdpStream(device::manager::SourceUdpStruct {
            ip: "192.168.0.1".parse().unwrap(),
            port: 9092,
        }),
        device_selection: device::manager::DeviceSelection::Auto,
    })));

    tokio::spawn(async move { manager.run().await });

    server::manager::run(&cli::manager::server_address(), handler)
        .await
        .unwrap();
}
