use paperclip::actix::Apiv2Schema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
    net::{Ipv4Addr, SocketAddrV4},
};
use tokio::sync::{mpsc, oneshot};
use tokio_serial::{SerialPort, SerialPortBuilderExt, SerialStream};
use tracing::{error, info, trace, warn};
use udp_stream::UdpStream;
use uuid::Uuid;

use super::devices::{DeviceActor, DeviceActorHandler};
use bluerobotics_ping::device::{Ping1D, Ping360};

struct Device {
    id: Uuid,
    source: SourceSelection,
    handler: super::devices::DeviceActorHandler,
    actor: tokio::task::JoinHandle<DeviceActor>,
    broadcast: Option<tokio::task::JoinHandle<()>>,
    status: DeviceStatus,
    device_type: DeviceSelection,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeviceInfo {
    pub id: Uuid,
    pub source: SourceSelection,
    pub status: DeviceStatus,
    pub device_type: DeviceSelection,
}
impl Device {
    pub fn info(&self) -> DeviceInfo {
        DeviceInfo {
            id: self.id,
            source: self.source.clone(),
            status: self.status.clone(),
            device_type: self.device_type.clone(),
        }
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        trace!(
            "Removing Device from DeviceManager, details: {:?}",
            self.info()
        );
        self.actor.abort();
        if let Some(broadcast_handle) = &self.broadcast {
            trace!("Device broadcast handle closed for: {:?}", self.info().id);
            broadcast_handle.abort();
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeviceSelection {
    Common,
    Ping1D,
    Ping360,
    Auto,
}

#[derive(Debug, Clone, Deserialize, Serialize, Hash)]
pub enum SourceSelection {
    UdpStream(SourceUdpStruct),
    SerialStream(SourceSerialStruct),
}

enum SourceType {
    Udp(UdpStream),
    Serial(SerialStream),
}

#[derive(Clone, Debug, Deserialize, Serialize, Hash, Apiv2Schema)]
pub struct SourceUdpStruct {
    pub ip: Ipv4Addr,
    pub port: u16,
}

#[derive(Clone, Debug, Deserialize, Serialize, Hash, Apiv2Schema)]
pub struct SourceSerialStruct {
    pub path: String,
    pub baudrate: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeviceStatus {
    Running,
    Stopped,
    Broadcasting,
}

pub struct DeviceManager {
    receiver: mpsc::Receiver<ManagerActorRequest>,
    device: HashMap<Uuid, Device>,
}

#[derive(Debug)]
pub struct ManagerActorRequest {
    pub request: Request,
    pub respond_to: oneshot::Sender<Result<Answer, ManagerError>>,
}
#[derive(Clone)]
pub struct ManagerActorHandler {
    pub sender: mpsc::Sender<ManagerActorRequest>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Apiv2Schema)]
pub enum Answer {
    DeviceMessage(DeviceAnswer),
    #[serde(skip)]
    InnerDeviceHandler(DeviceActorHandler),
    DeviceInfo(Vec<DeviceInfo>),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ManagerError {
    DeviceNotExist(Uuid),
    DeviceAlreadyExist(Uuid),
    DeviceStatus(DeviceStatus, Uuid),
    DeviceError(super::devices::DeviceError),
    DeviceSourceError(String),
    NoDevices,
    TokioMpsc(String),
    NotImplemented(Request),
    Other(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeviceAnswer {
    #[serde(flatten)]
    pub answer: crate::device::devices::PingAnswer,
    pub device_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, Apiv2Schema)]
pub enum Request {
    Create(CreateStruct),
    Delete(Uuid),
    List,
    Info(Uuid),
    Search,
    Ping(DeviceRequestStruct),
    GetDeviceHandler(Uuid),
    EnableBroadcasting(Uuid),
    DisableBroadcasting(Uuid),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateStruct {
    pub source: SourceSelection,
    pub device_selection: DeviceSelection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceRequestStruct {
    pub target: Uuid,
    pub request: crate::device::devices::PingRequest,
}

impl DeviceManager {
    async fn handle_message(&mut self, actor_request: ManagerActorRequest) {
        trace!("DeviceManager: Received a request, details: {actor_request:?}");
        match actor_request.request {
            Request::Create(request) => {
                let result = self.create(request.source, request.device_selection).await;
                if let Err(e) = actor_request.respond_to.send(result) {
                    error!("DeviceManager: Failed to return Create response: {e:?}");
                }
            }
            Request::Delete(uuid) => {
                let result = self.delete(uuid).await;
                if let Err(e) = actor_request.respond_to.send(result) {
                    error!("DeviceManager: Failed to return Delete response: {e:?}");
                }
            }
            Request::List => {
                let result = self.list().await;
                if let Err(e) = actor_request.respond_to.send(result) {
                    error!("DeviceManager: Failed to return List response: {e:?}");
                }
            }
            Request::Info(device_id) => {
                let result = self.info(device_id).await;
                if let Err(e) = actor_request.respond_to.send(result) {
                    error!("DeviceManager: Failed to return Info response: {:?}", e);
                }
            }
            Request::EnableBroadcasting(uuid) => {
                let result = self.broadcast(uuid).await;
                if let Err(e) = actor_request.respond_to.send(result) {
                    error!(
                        "DeviceManager: Failed to return EnableBroadcasting response: {:?}",
                        e
                    );
                }
            }
            Request::DisableBroadcasting(uuid) => {
                let result = self.broadcast_off(uuid).await;
                if let Err(e) = actor_request.respond_to.send(result) {
                    error!(
                        "DeviceManager: Failed to return DisableBroadcasting response: {:?}",
                        e
                    );
                }
            }
            Request::GetDeviceHandler(id) => {
                let answer = self.get_device_handler(id).await;
                if let Err(e) = actor_request.respond_to.send(answer) {
                    error!("DeviceManager: Failed to return GetDeviceHandler response: {e:?}");
                }
            }
            _ => {
                if let Err(e) = actor_request
                    .respond_to
                    .send(Err(ManagerError::NotImplemented(actor_request.request)))
                {
                    warn!("DeviceManager: Failed to return response: {e:?}");
                }
            }
        }
    }

    pub fn new(size: usize) -> (Self, ManagerActorHandler) {
        let (sender, receiver) = mpsc::channel(size);
        let actor = DeviceManager {
            receiver,
            device: HashMap::new(),
        };
        let actor_handler = ManagerActorHandler { sender };

        trace!("DeviceManager and handler successfully created: Success");
        (actor, actor_handler)
    }

    pub async fn run(mut self) {
        while let Some(msg) = self.receiver.recv().await {
            self.update_devices_status().await; // Todo: move to an outer process
            self.handle_message(msg).await;
        }
        error!("DeviceManager has stopped please check your application");
    }

    pub async fn update_devices_status(&mut self) {
        if let Ok(Answer::DeviceInfo(answer)) = self.list().await {
            for device in answer {
                if let Some(device_entry) = self.device.get_mut(&device.id) {
                    if device_entry.status == DeviceStatus::Stopped {
                        break;
                    }
                    if device_entry.actor.is_finished() {
                        info!("Device stopped, device id: {device:?}");
                        device_entry.status = DeviceStatus::Stopped;
                    }
                }
            }
        }
    }

    pub async fn create(
        &mut self,
        source: SourceSelection,
        mut device_selection: DeviceSelection,
    ) -> Result<Answer, ManagerError> {
        let mut hasher = DefaultHasher::new();
        source.hash(&mut hasher);
        let hash = Uuid::from_u128(hasher.finish().into());

        if self.device.contains_key(&hash) {
            trace!("Device creation error: Device already exist for provided SourceSelection, details: {source:?}");
            return Err(ManagerError::DeviceAlreadyExist(hash));
        }

        let port = match &source {
            SourceSelection::UdpStream(source_udp_struct) => {
                let socket_addr = SocketAddrV4::new(source_udp_struct.ip, source_udp_struct.port);

                let udp_stream = UdpStream::connect(socket_addr.into())
                    .await
                    .map_err(|err| ManagerError::DeviceSourceError(err.to_string()))?;
                SourceType::Udp(udp_stream)
            }
            SourceSelection::SerialStream(source_serial_struct) => {
                let serial_stream = tokio_serial::new(
                    source_serial_struct.path.clone(),
                    source_serial_struct.baudrate,
                )
                .open_native_async()
                .map_err(|err| ManagerError::DeviceSourceError(err.to_string()))?;

                serial_stream
                    .clear(tokio_serial::ClearBuffer::All)
                    .map_err(|err| ManagerError::DeviceSourceError(err.to_string()))?;

                SourceType::Serial(serial_stream)
            }
        };

        let device = match port {
            SourceType::Udp(udp_port) => match device_selection {
                DeviceSelection::Common | DeviceSelection::Auto => {
                    crate::device::devices::DeviceType::Common(
                        bluerobotics_ping::common::Device::new(udp_port),
                    )
                }
                DeviceSelection::Ping1D => {
                    crate::device::devices::DeviceType::Ping1D(Ping1D::new(udp_port))
                }
                DeviceSelection::Ping360 => {
                    crate::device::devices::DeviceType::Ping360(Ping360::new(udp_port))
                }
            },
            SourceType::Serial(serial_port) => match device_selection {
                DeviceSelection::Common | DeviceSelection::Auto => {
                    crate::device::devices::DeviceType::Common(
                        bluerobotics_ping::common::Device::new(serial_port),
                    )
                }
                DeviceSelection::Ping1D => {
                    crate::device::devices::DeviceType::Ping1D(Ping1D::new(serial_port))
                }
                DeviceSelection::Ping360 => {
                    crate::device::devices::DeviceType::Ping360(Ping360::new(serial_port))
                }
            },
        };

        let (mut device, handler) = super::devices::DeviceActor::new(device, 10);

        if device_selection == DeviceSelection::Auto {
            match device.try_upgrade().await {
                Ok(super::devices::PingAnswer::UpgradeResult(result)) => match result {
                    super::devices::UpgradeResult::Unknown => {
                        device_selection = DeviceSelection::Common;
                    }
                    super::devices::UpgradeResult::Ping1D => {
                        device_selection = DeviceSelection::Ping1D;
                    }
                    super::devices::UpgradeResult::Ping360 => {
                        device_selection = DeviceSelection::Ping360;
                    }
                },
                Err(err) => {
                    error!(
                        "Device creation error: Can't auto upgrade the DeviceType, details: {err:?}"
                    );
                    return Err(ManagerError::DeviceError(err));
                }
                _ => todo!(),
            }
        }

        let actor = tokio::spawn(async move { device.run().await });

        let device = Device {
            id: hash,
            source,
            handler,
            actor,
            status: DeviceStatus::Running,
            broadcast: None,
            device_type: device_selection,
        };

        let device_info = device.info();

        self.device.insert(hash, device);

        trace!("Device broadcast enable by default for: {:?}", device_info);
        let _ = self.broadcast(hash).await?;

        info!(
            "New device created and available, details: {:?}",
            device_info
        );
        Ok(Answer::DeviceInfo(vec![device_info]))
    }

    pub async fn list(&self) -> Result<Answer, ManagerError> {
        if self.device.is_empty() {
            trace!("No devices available for list generation request");
            return Err(ManagerError::NoDevices);
        };
        let mut list = Vec::new();
        for device in self.device.values() {
            list.push(device.info())
        }
        Ok(Answer::DeviceInfo(list))
    }

    pub async fn info(&self, device_id: Uuid) -> Result<Answer, ManagerError> {
        self.check_device_uuid(device_id)?;
        Ok(Answer::DeviceInfo(vec![self.get_device(device_id)?.info()]))
    }

    fn check_device_uuid(&self, device_id: Uuid) -> Result<(), ManagerError> {
        if self.device.contains_key(&device_id) {
            return Ok(());
        }
        error!(
            "Getting device handler for device: {:?} : Error, device doesn't exist",
            device_id
        );
        Err(ManagerError::DeviceNotExist(device_id))
    }

    fn get_device(&self, device_id: Uuid) -> Result<&Device, ManagerError> {
        let device = self
            .device
            .get(&device_id)
            .ok_or(ManagerError::DeviceNotExist(device_id))?;
        Ok(device)
    }

    pub async fn delete(&mut self, device_id: Uuid) -> Result<Answer, ManagerError> {
        match self.device.remove(&device_id) {
            Some(device) => {
                let device_info = device.info();
                drop(device);
                trace!("Device delete id {:?}: Success", device_id);
                Ok(Answer::DeviceInfo(vec![device_info]))
            }
            None => {
                error!("Device delete id {device_id:?} : Error, device doesn't exist");
                Err(ManagerError::DeviceNotExist(device_id))
            }
        }
    }

    pub async fn get_device_handler(&self, device_id: Uuid) -> Result<Answer, ManagerError> {
        self.check_device_uuid(device_id)?;

        trace!(
            "Getting device handler for device: {:?} : Success",
            device_id
        );

        // Fail-fast if device is stopped
        self.check_device_status(
            device_id,
            &[DeviceStatus::Broadcasting, DeviceStatus::Running],
        )?;

        let handler: DeviceActorHandler = self.get_device(device_id)?.handler.clone();

        Ok(Answer::InnerDeviceHandler(handler))
    }

    fn check_device_status(
        &self,
        device_id: Uuid,
        valid_statuses: &[DeviceStatus],
    ) -> Result<(), ManagerError> {
        let status = &self.get_device(device_id)?.status;
        if !valid_statuses.contains(status) {
            return Err(ManagerError::DeviceStatus(status.clone(), device_id));
        }
        Ok(())
    }

    fn get_mut_device(&mut self, device_id: Uuid) -> Result<&mut Device, ManagerError> {
        let device = self
            .device
            .get_mut(&device_id)
            .ok_or(ManagerError::DeviceNotExist(device_id))?;
        Ok(device)
    }

    fn get_device_type(&self, device_id: Uuid) -> Result<DeviceSelection, ManagerError> {
        let device_type = self.device.get(&device_id).unwrap().device_type.clone();
        Ok(device_type)
    }

    fn extract_handler(&self, device_handler: Answer) -> Result<DeviceActorHandler, ManagerError> {
        match device_handler {
            Answer::InnerDeviceHandler(handler) => Ok(handler),
            answer => Err(ManagerError::Other(format!(
                "Unreachable: extract_handler helper, detail: {answer:?}"
            ))),
        }
    }

    async fn get_subscriber(
        &self,
        device_id: Uuid,
    ) -> Result<
        tokio::sync::broadcast::Receiver<bluerobotics_ping::message::ProtocolMessage>,
        ManagerError,
    > {
        let handler_request = self.get_device_handler(device_id).await?;
        let handler = self.extract_handler(handler_request)?;

        let subscriber = handler
            .send(super::devices::PingRequest::GetSubscriber)
            .await
            .map_err(|err| {
                trace!("Something went wrong while executing get_subscriber, details: {err:?}");
                ManagerError::DeviceError(err)
            })?;

        match subscriber {
            super::devices::PingAnswer::Subscriber(subscriber) => Ok(subscriber),
            _ => Err(ManagerError::Other(
                "Unreachable: get_subscriber helper".to_string(),
            )),
        }
    }

    pub async fn broadcast(&mut self, device_id: Uuid) -> Result<Answer, ManagerError> {
        self.check_device_status(device_id, &[DeviceStatus::Running])?;
        let device_type = self.get_device_type(device_id)?;

        // Get an inner subscriber for device's stream
        let subscriber = self.get_subscriber(device_id).await?;

        let broadcast_handle = self.start_broadcasting(subscriber, device_id, device_type.clone());

        let device = self.get_mut_device(device_id)?;
        device.broadcast = broadcast_handle;
        device.status = DeviceStatus::Broadcasting;

        self.broadcast_startup_routine(device_id, device_type)
            .await?;

        let updated_device_info = self.get_device(device_id)?.info();

        Ok(Answer::DeviceInfo(vec![updated_device_info]))
    }

    pub async fn broadcast_off(&mut self, device_id: Uuid) -> Result<Answer, ManagerError> {
        self.check_device_status(device_id, &[DeviceStatus::Broadcasting])?;
        let device_type = self.get_device_type(device_id)?;

        let device = self.get_mut_device(device_id)?;
        if let Some(broadcast) = device.broadcast.take() {
            broadcast.abort_handle().abort();
        }

        device.status = DeviceStatus::Running;

        let updated_device_info = device.info();

        self.broadcast_stop_routine(device_id, device_type).await?;

        Ok(Answer::DeviceInfo(vec![updated_device_info]))
    }

    // Series of inner helpers specially for broadcast methods

    // Call the helpers specifically for each device type
    fn start_broadcasting(
        &self,
        mut subscriber: tokio::sync::broadcast::Receiver<
            bluerobotics_ping::message::ProtocolMessage,
        >,
        device_id: Uuid,
        device_type: DeviceSelection,
    ) -> Option<tokio::task::JoinHandle<()>> {
        match device_type {
            DeviceSelection::Ping1D => Some(tokio::spawn(async move {
                loop {
                    match subscriber.recv().await {
                        Ok(msg) => {
                            Self::ping1d_broadcast_helper(msg, device_id);
                        }
                        Err(_e) => {
                            Self::handle_error_broadcast_message(_e, device_id);
                            break;
                        }
                    }
                }
            })),
            DeviceSelection::Ping360 => Some(tokio::spawn(async move {
                loop {
                    match subscriber.recv().await {
                        Ok(msg) => {
                            Self::ping360_broadcast_helper(msg, device_id);
                        }
                        Err(_e) => {
                            Self::handle_error_broadcast_message(_e, device_id);
                            break;
                        }
                    }
                }
            })),
            DeviceSelection::Common | DeviceSelection::Auto => None,
        }
    }

    // Execute some especial commands required for device enter in auto_send mode
    async fn broadcast_startup_routine(
        &self,
        device_id: Uuid,
        device_type: DeviceSelection,
    ) -> Result<(), ManagerError> {
        if device_type == DeviceSelection::Ping1D {
            let handler_request = self.get_device_handler(device_id).await?;
            let handler = self.extract_handler(handler_request)?;

            let id = <bluerobotics_ping::ping1d::ProfileStruct as bluerobotics_ping::message::MessageInfo>::id();
            let _ = handler
                .send(super::devices::PingRequest::Ping1D(
                    super::devices::Ping1DRequest::ContinuousStart(
                        bluerobotics_ping::ping1d::ContinuousStartStruct { id },
                    ),
                ))
                .await
                .map_err(|err| {trace!("Something went wrong while executing broadcast_startup_routine, details: {err:?}"); ManagerError::DeviceError(err)})?;
        }
        Ok(())
    }

    // Execute some especial commands required for device stop auto_send mode
    async fn broadcast_stop_routine(
        &self,
        device_id: Uuid,
        device_type: DeviceSelection,
    ) -> Result<(), ManagerError> {
        let handler_request = self.get_device_handler(device_id).await?;
        let handler = self.extract_handler(handler_request)?;

        if device_type == DeviceSelection::Ping1D {
            let id = <bluerobotics_ping::ping1d::ProfileStruct as bluerobotics_ping::message::MessageInfo>::id();
            let _ = handler
                .send(super::devices::PingRequest::Ping1D(
                    super::devices::Ping1DRequest::ContinuousStop(
                        bluerobotics_ping::ping1d::ContinuousStopStruct { id },
                    ),
                ))
                .await
                .map_err(|err| {trace!("Something went wrong while executing broadcast_startup_routine, details: {err:?}"); ManagerError::DeviceError(err)})?;
        }
        Ok(())
    }

    // An inner helper focused on Ping1D, which uses Profile message to plot graphs
    fn ping1d_broadcast_helper(msg: bluerobotics_ping::message::ProtocolMessage, device_id: Uuid) {
        if msg.message_id == <bluerobotics_ping::ping1d::ProfileStruct as bluerobotics_ping::message::MessageInfo>::id() {
            if let Ok(bluerobotics_ping::Messages::Ping1D(bluerobotics_ping::ping1d::Messages::Profile(_answer))) = bluerobotics_ping::Messages::try_from(&msg) {
                let answer = Answer::DeviceMessage(DeviceAnswer {
                    answer: super::devices::PingAnswer::PingMessage(
                        bluerobotics_ping::Messages::try_from(&msg).unwrap(),
                    ),
                    device_id,
                });
                crate::server::protocols::v1::websocket::send_to_websockets(json!(answer), Some(device_id));
            }
        }
    }

    // An inner helper focused on Ping360, which uses DeviceData message to plot graphs
    fn ping360_broadcast_helper(msg: bluerobotics_ping::message::ProtocolMessage, device_id: Uuid) {
        if msg.message_id == <bluerobotics_ping::ping360::DeviceDataStruct as bluerobotics_ping::message::MessageInfo>::id() {
            if let Ok(bluerobotics_ping::Messages::Ping360(bluerobotics_ping::ping360::Messages::DeviceData(_answer))) = bluerobotics_ping::Messages::try_from(&msg) {
                let answer = Answer::DeviceMessage(DeviceAnswer {
                    answer: super::devices::PingAnswer::PingMessage(
                        bluerobotics_ping::Messages::try_from(&msg).unwrap(),
                    ),
                    device_id,
                });
                crate::server::protocols::v1::websocket::send_to_websockets(json!(answer), Some(device_id));
            }
        }
    }

    // An inner helper that returns error to requester
    fn handle_error_broadcast_message(
        error: tokio::sync::broadcast::error::RecvError,
        device_id: Uuid,
    ) {
        let error = ManagerError::DeviceError(super::devices::DeviceError::PingError(
            bluerobotics_ping::error::PingError::TokioBroadcastError(error.to_string()),
        ));
        crate::server::protocols::v1::websocket::send_to_websockets(json!(error), Some(device_id));
    }
}

impl ManagerActorHandler {
    pub async fn send(&self, request: Request) -> Result<Answer, ManagerError> {
        let (result_sender, result_receiver) = oneshot::channel();

        match &request {
            // Devices requests are forwarded directly to device and let manager handle other incoming request.
            Request::Ping(request) => {
                trace!("Handling Ping request: {request:?}: Forwarding request to device handler");
                let get_handler_target = request.target;
                let handler_request = Request::GetDeviceHandler(get_handler_target);
                let manager_request = ManagerActorRequest {
                    request: handler_request,
                    respond_to: result_sender,
                };
                self.sender
                    .send(manager_request)
                    .await
                    .map_err(|err| ManagerError::TokioMpsc(err.to_string()))?;
                let result = match result_receiver
                    .await
                    .map_err(|err| ManagerError::TokioMpsc(err.to_string()))
                {
                    Ok(ans) => ans,
                    Err(err) => {
                        error!("DeviceManagerHandler: Failed to receive handler from Manager, details: {err:?}");
                        return Err(err);
                    }
                };

                match result? {
                    Answer::InnerDeviceHandler(handler) => {
                        trace!(
                            "Handling Ping request: {request:?}: Successfully received the handler"
                        );
                        let result = handler.send(request.request.clone()).await;
                        match result {
                            Ok(result) => {
                                info!("Handling Ping request: {request:?}: Success");
                                Ok(Answer::DeviceMessage(DeviceAnswer {
                                    answer: result,
                                    device_id: request.target,
                                }))
                            }
                            Err(err) => {
                                error!(
                                    "Handling Ping request: {request:?}: Error ocurred on device: {err:?}"                                );
                                Err(ManagerError::DeviceError(err))
                            }
                        }
                    }
                    answer => Ok(answer), //should be unreachable
                }
            }
            _ => {
                trace!("Handling DeviceManager request: {request:?}: Forwarding request.");
                let device_request = ManagerActorRequest {
                    request: request.clone(),
                    respond_to: result_sender,
                };

                self.sender
                    .send(device_request)
                    .await
                    .map_err(|err| ManagerError::TokioMpsc(err.to_string()))?;

                match result_receiver
                    .await
                    .map_err(|err| ManagerError::TokioMpsc(err.to_string()))?
                {
                    Ok(ans) => {
                        info!("Handling DeviceManager request: {request:?}: Success");
                        Ok(ans)
                    }
                    Err(err) => {
                        error!(
                            "Handling DeviceManager request: {request:?}: Error ocurred on manager: {err:?}",
                        );
                        Err(err)
                    }
                }
            }
        }
    }
}
