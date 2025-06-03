use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::{broadcast, RwLock};
use tracing::{error, warn};
use uuid::Uuid;
use mcap::{Compression, WriteOptions};
use foxglove::{ChannelBuilder, McapWriter, log, Context};
use foxglove::McapWriterHandle;
use std::io::BufWriter;
use std::fs::File;

use crate::device::{
    devices::DeviceActorHandler,
    manager::{DeviceInfo as ManagerDeviceInfo, DeviceManager, DeviceSelection, ManagerError},
};

mod schemas;
use schemas::{DeviceInfo, Ping1DMessage, Ping360Message, Potato, RecordingHeader};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingSession {
    pub device_id: Uuid,
    pub file_path: PathBuf,
    pub is_active: bool,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub device_type: DeviceSelection,
}

#[derive(Debug, Clone)]
pub struct RecordingManager {
    sessions: Arc<RwLock<HashMap<Uuid, RecordingSession>>>,
    writers: Arc<RwLock<HashMap<Uuid, McapWriterHandle<BufWriter<File>>>>>,
    base_path: PathBuf,
    status_tx: broadcast::Sender<RecordingSession>,
}

impl RecordingManager {
    pub fn new(base_path: impl AsRef<Path>) -> Self {
        let (status_tx, _) = broadcast::channel(100);
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            writers: Arc::new(RwLock::new(HashMap::new())),
            base_path: base_path.as_ref().to_path_buf(),
            status_tx,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<RecordingSession> {
        self.status_tx.subscribe()
    }

    async fn broadcast_status(&self, session: &RecordingSession) {
        let _ = self.status_tx.send(session.clone());
    }

    pub async fn start_recording(
        &self,
        device_manager: &DeviceManager,
        device_id: Uuid,
    ) -> Result<RecordingSession, ManagerError> {
        device_manager.check_device_status(
            device_id,
            &[
                crate::device::manager::DeviceStatus::Running,
                crate::device::manager::DeviceStatus::ContinuousMode,
            ],
        )?;

        if self.sessions.read().await.contains_key(&device_id) {
            return Err(ManagerError::Other(format!(
                "Device {} is already recording",
                device_id
            )));
        }

        tokio::fs::create_dir_all(&self.base_path)
            .await
            .map_err(|e| {
                ManagerError::Other(format!("Failed to create recording directory: {}", e))
            })?;

        let timestamp = chrono::Utc::now();
        let filename = format!(
            "device_{}_{}.mcap",
            device_id,
            timestamp.format("%Y%m%d_%H%M%S")
        );
        let file_path = self.base_path.join(filename);

        let device_info = device_manager.get_device(device_id)?.info();

        let options = WriteOptions::new()
            .chunk_size(Some(1024 * 768))
            .compression(Some(Compression::Zstd));

        // Create context and get writer
        let ctx = Context::new();
        let mcap_writer = ctx.mcap_writer().create_new_buffered_file(&file_path)
            .map_err(|e| ManagerError::Other(format!("Failed to create MCAP file: {}", e)))?;

        let session = RecordingSession {
            device_id,
            file_path: file_path.clone(),
            is_active: true,
            start_time: timestamp,
            device_type: device_info.device_type.clone(),
        };

        self.sessions
            .write()
            .await
            .insert(device_id, session.clone());
        self.writers
            .write()
            .await
            .insert(device_id, mcap_writer);
        self.broadcast_status(&session).await;

        let sessions = self.sessions.clone();
        let writers = self.writers.clone();
        let device_handler = device_manager.get_device_handler(device_id).await?;
        let handler = match device_handler {
            crate::device::manager::Answer::InnerDeviceHandler(h) => h,
            _ => return Err(ManagerError::Other("Invalid device handler".to_string())),
        };

        tokio::spawn(async move {
            if let Err(e) =
                Self::recording_task(handler, file_path, sessions, writers, device_id, device_info, ctx).await
            {
                error!("Recording task failed for device {}: {:?}", device_id, e);
            }
        });

        Ok(session)
    }

    pub async fn stop_recording(&self, device_id: Uuid) -> Result<RecordingSession, ManagerError> {
        let mut sessions = self.sessions.write().await;
        let session = sessions.get_mut(&device_id).ok_or_else(|| {
            ManagerError::Other(format!("No recording session for device {}", device_id))
        })?;

        session.is_active = false;
        if let Some(writer) = self.writers.write().await.remove(&device_id) {
            writer.close().map_err(|e| {
                ManagerError::Other(format!("Failed to close MCAP writer: {}", e))
            })?;
        }
        let session = session.clone();
        self.broadcast_status(&session).await;
        Ok(session)
    }

    pub async fn get_recording_status(
        &self,
        device_id: Uuid,
    ) -> Result<Option<RecordingSession>, ManagerError> {
        Ok(self.sessions.read().await.get(&device_id).cloned())
    }

    pub async fn get_all_recording_status(&self) -> Vec<RecordingSession> {
        self.sessions.read().await.values().cloned().collect()
    }

    async fn recording_task(
        handler: DeviceActorHandler,
        _file_path: PathBuf,
        sessions: Arc<RwLock<HashMap<Uuid, RecordingSession>>>,
        writers: Arc<RwLock<HashMap<Uuid, McapWriterHandle<BufWriter<File>>>>>,
        device_id: Uuid,
        device_info: ManagerDeviceInfo,
        ctx: Arc<Context>,
    ) -> Result<(), ManagerError> {
        let subscriber = handler
            .send(super::devices::PingRequest::GetSubscriber)
            .await
            .map_err(|err| {
                warn!("Something went wrong while executing get_subscriber, details: {err:?}");
                ManagerError::DeviceError(err)
            })?;

        let mut receiver = match subscriber {
            super::devices::PingAnswer::Subscriber(subscriber) => subscriber,
            msg => {
                error!("Failed to receive broadcasted message: {:?}", msg);
                return Err(ManagerError::NoDevices);
            }
        };

        let session = {
            let sessions_guard = sessions.read().await;
            sessions_guard.get(&device_id).unwrap().clone()
        };

        // Define topic strings
        let ping1d_topic = format!("/device_{}/ping1d", device_id);
        let ping360_topic = format!("/device_{}/ping360", device_id);
        let header_topic = format!("/device_{}/header", device_id);

        // Create device-specific channels with proper schema
        let ping1d_channel = ctx.channel_builder(&ping1d_topic) .add_metadata("foxglove.device_id", &device_id.to_string()).build::<Potato>();
        let ping360_channel = ctx.channel_builder(&ping360_topic).add_metadata("foxglove.device_id", &device_id.to_string() ).build::<Potato>();
        let header_channel = ctx.channel_builder(&header_topic).add_metadata("foxglove.device_id", &device_id.to_string() ).build::<Potato>();

        let header = RecordingHeader {
            version: "1.0".to_string(),
            device_info: DeviceInfo {
                device_id: device_id.to_string(),
                start_time: session.start_time.to_rfc3339(),
                device_properties: serde_json::json!(device_info),
            },
            file_format: "mcap".to_string(),
        };

        // Write header message using the specific writer and context
        let header_message = Potato {
            timestamp: chrono::Utc::now().to_rfc3339(),
            message_type: "ping360_auto_device_data".to_string()
        };
        header_channel.log(&header_message);

        while {
            let sessions_guard = sessions.read().await;
            sessions_guard
                .get(&device_id)
                .map(|s| s.is_active)
                .unwrap_or(false)
        } {
            match receiver.recv().await {
                Ok(msg) => {
                    if let Ok(bluerobotics_ping::Messages::Ping360(
                        bluerobotics_ping::ping360::Messages::AutoDeviceData(answer),
                    )) = bluerobotics_ping::Messages::try_from(&msg)
                    {
                        let message = Ping360Message {
                            timestamp: chrono::Utc::now().to_rfc3339(),
                            message_type: "ping360_auto_device_data".to_string(),
                            data: serde_json::json!(answer),
                        };

                        ping360_channel.log(&header_message);
                    }

                    if let Ok(bluerobotics_ping::Messages::Ping360(
                        bluerobotics_ping::ping360::Messages::Transducer(answer),
                    )) = bluerobotics_ping::Messages::try_from(&msg)
                    {
                        let message = Ping360Message {
                            timestamp: chrono::Utc::now().to_rfc3339(),
                            message_type: "ping360_transducer".to_string(),
                            data: serde_json::json!(answer),
                        };

                        ping360_channel.log(&header_message);
                    }

                    if let Ok(bluerobotics_ping::Messages::Ping1D(
                        bluerobotics_ping::ping1d::Messages::Profile(answer),
                    )) = bluerobotics_ping::Messages::try_from(&msg)
                    {
                        let message = Ping1DMessage {
                            timestamp: chrono::Utc::now().to_rfc3339(),
                            message_type: "ping1d_profile".to_string(),
                            data: serde_json::json!(answer),
                        };

                        ping1d_channel.log(&header_message);
                    }
                }
                Err(e) => {
                    error!("Failed to receive broadcasted message: {:?}", e);
                    break;
                }
            }
        }

        sessions.write().await.remove(&device_id);
        writers.write().await.remove(&device_id);
        Ok(())
    }
}
