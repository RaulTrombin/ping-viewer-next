// use std::sync::Arc;

// use tokio::sync::Mutex;
use std::{
    net::{Ipv4Addr, SocketAddrV4},
    sync::Arc,
};

use bluerobotics_ping::{
    device::{Common, Ping1D, Ping360, PingDevice},
    message::MessageInfo,
    ping1d::{self, ProfileStruct},
    Messages,
};

use paperclip::actix::Apiv2Schema;
use serde::{Deserialize, Serialize};

use serde_json::json;
use tokio::sync::RwLock;
use tokio_serial::{SerialPort, SerialPortBuilderExt, SerialStream};
use udp_stream::UdpStream;

use crate::server::protocols::v1::websocket;

struct Device {
    source: SourceSelection,
    device: DeviceType,
    status: DeviceStatus,
}
pub enum DeviceType {
    Common(bluerobotics_ping::device::Common),
    Ping1D(bluerobotics_ping::device::Ping1D),
    Ping360(bluerobotics_ping::device::Ping360),
}

pub enum DeviceSelection {
    Common,
    Ping1D,
    Ping360,
}

#[derive(Apiv2Schema, Debug, Deserialize, Serialize)]
pub enum SourceSelection {
    UdpStream(SourceUdpStruct),
    SerialStream(SourceSerialStruct),
}

enum SourceType {
    Udp(UdpStream),
    Serial(SerialStream),
}

#[derive(Apiv2Schema, Debug, Deserialize, Serialize)]
pub struct SourceUdpStruct {
    pub ip: Ipv4Addr,
    pub port: u16,
}

#[derive(Apiv2Schema, Debug, Deserialize, Serialize)]
pub struct SourceSerialStruct {
    pub path: String,
    pub baudrate: u32,
}
#[derive(Debug)]
enum DeviceStatus {
    Running,
    Stopped,
    Resetting,
    Error,
}

#[derive(Default)]
pub struct DeviceManager {
    ping: Vec<Device>,
    // subscribed
    monitor: Option<std::thread::JoinHandle<()>>,
}

lazy_static! {
    static ref MANAGER: Arc<RwLock<DeviceManager>> = Default::default();
}

impl DeviceManager {
    pub fn get_instance() -> &'static RwLock<Self> {
        &MANAGER
    }

    pub async fn create(source: SourceSelection, device_selection: DeviceSelection) {
        let _device: DeviceType;

        let port = match &source {
            SourceSelection::UdpStream(source_udp_struct) => {
                let socket_addr = SocketAddrV4::new(source_udp_struct.ip, source_udp_struct.port);

                let udp_stream = UdpStream::connect(socket_addr.into())
                    .await
                    .map_err(|e| {
                        eprintln!("Error connecting to UDP socket: {}", e);
                        e
                    })
                    .unwrap();
                SourceType::Udp(udp_stream)
            }
            SourceSelection::SerialStream(source_serial_struct) => {
                let serial_stream = tokio_serial::new(
                    source_serial_struct.path.clone(),
                    source_serial_struct.baudrate,
                )
                .open_native_async()
                .map_err(|e| {
                    eprintln!("Error opening serial port: {}", e);
                    e
                })
                .unwrap();
                serial_stream.clear(tokio_serial::ClearBuffer::All).unwrap();

                SourceType::Serial(serial_stream)
            }
        };

        let device = match device_selection {
            DeviceSelection::Common => match port {
                SourceType::Udp(udp_port) => DeviceType::Common(Common::new(udp_port)),
                SourceType::Serial(serial_port) => DeviceType::Common(Common::new(serial_port)),
            },
            DeviceSelection::Ping1D => match port {
                SourceType::Udp(udp_port) => DeviceType::Ping1D(Ping1D::new(udp_port)),
                SourceType::Serial(serial_port) => DeviceType::Ping1D(Ping1D::new(serial_port)),
            },
            DeviceSelection::Ping360 => match port {
                SourceType::Udp(udp_port) => DeviceType::Ping360(Ping360::new(udp_port)),
                SourceType::Serial(serial_port) => DeviceType::Ping360(Ping360::new(serial_port)),
            },
        };

        let status = DeviceStatus::Running;

        let lock = Self::get_instance().write();
        lock.await.ping.push(Device {
            source,
            device,
            status,
        })
    }

    pub async fn list() -> Vec<String> {
        let lock = Self::get_instance().read().await;
        let mut list = Vec::new();
        for device in lock.ping.iter() {
            list.push(format!("{:?}", device.source));
        }
        list
    }

    pub async fn init() {
        let lock = Self::get_instance().read().await;
        if !lock.ping.is_empty() {
            match &lock.ping[0].device {
                DeviceType::Ping1D(inner) => {
                    inner
                        .continuous_start(bluerobotics_ping::ping1d::ProfileStruct::id())
                        .await
                        .unwrap();
                }
                DeviceType::Ping360(_) => todo!(),
                DeviceType::Common(_) => todo!(),
            }
        }
    }

    pub async fn stop() {
        let lock = Self::get_instance().read().await;
        if !lock.ping.is_empty() {
            match &lock.ping[0].device {
                DeviceType::Ping1D(inner) => {
                    inner
                        .continuous_stop(bluerobotics_ping::ping1d::ProfileStruct::id())
                        .await
                        .unwrap();
                }
                DeviceType::Ping360(_) => todo!(),
                DeviceType::Common(_) => todo!(),
            }
        }
    }

    pub async fn subscribe() {
        let lock = Self::get_instance().read().await;
        if !lock.ping.is_empty() {
            match &lock.ping[0].device {
                DeviceType::Ping1D(inner) => {
                    let mut subscribed = inner.subscribe();

                    let (_tx, _rx) = tokio::sync::oneshot::channel::<Vec<ProfileStruct>>();
                    inner
                        .continuous_start(bluerobotics_ping::ping1d::ProfileStruct::id())
                        .await
                        .unwrap();

                    tokio::spawn(async move {
                        let mut profile_struct_vector: Vec<ProfileStruct> = Vec::new();
                        loop {
                            let received = subscribed.recv().await;
                            match received {
                                Ok(msg) => {
                                    if msg.message_id
                                        == bluerobotics_ping::ping1d::ProfileStruct::id()
                                    {
                                        match Messages::try_from(&msg) {
                                            Ok(Messages::Ping1D(ping1d::Messages::Profile(
                                                answer,
                                            ))) => {
                                                profile_struct_vector.push(answer.clone());
                                                websocket::send_to_websockets(json!(format!(
                                                    "{:?}",
                                                    answer
                                                )));
                                            }
                                            _ => continue,
                                        }
                                    }
                                }
                                Err(_e) => break,
                            }
                        }
                    });
                }
                DeviceType::Ping360(_) => todo!(),
                DeviceType::Common(_) => todo!(),
            }
        }
    }
}
