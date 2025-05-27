use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::{
    io::AsyncWriteExt,
    sync::{broadcast, RwLock},
};
use tracing::{error, warn};
use uuid::Uuid;

use crate::device::{
    devices::DeviceActorHandler,
    manager::{DeviceInfo, DeviceManager, DeviceSelection, ManagerError},
};

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
    base_path: PathBuf,
    status_tx: broadcast::Sender<RecordingSession>,
}

impl RecordingManager {
    pub fn new(base_path: impl AsRef<Path>) -> Self {
        let (status_tx, _) = broadcast::channel(100);
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
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
            "device_{}_{}.json",
            device_id,
            timestamp.format("%Y%m%d_%H%M%S")
        );
        let file_path = self.base_path.join(filename);

        let device_info = device_manager.get_device(device_id)?.info();

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
        self.broadcast_status(&session).await;

        let sessions = self.sessions.clone();
        let device_handler = device_manager.get_device_handler(device_id).await?;
        let handler = match device_handler {
            crate::device::manager::Answer::InnerDeviceHandler(h) => h,
            _ => return Err(ManagerError::Other("Invalid device handler".to_string())),
        };

        tokio::spawn(async move {
            if let Err(e) =
                Self::recording_task(handler, file_path, sessions, device_id, device_info).await
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
        file_path: PathBuf,
        sessions: Arc<RwLock<HashMap<Uuid, RecordingSession>>>,
        device_id: Uuid,
        device_info: DeviceInfo,
    ) -> Result<(), ManagerError> {
        let file = tokio::fs::File::create(&file_path)
            .await
            .map_err(|e| ManagerError::Other(format!("Failed to create recording file: {}", e)))?;

        let mut writer = tokio::io::BufWriter::new(file);

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

        let header = serde_json::json!({
            "version": "1.0",
            "device_info": {
                "device_id": device_id.to_string(),
                "start_time": session.start_time.to_rfc3339(),
                "file_format": "json",
                "device_properties": serde_json::json!(device_info),
            }
        });

        if let Err(e) = tokio::io::AsyncWriteExt::write_all(
            &mut writer,
            format!("{}\n", header.to_string()).as_bytes(),
        )
        .await
        {
            error!("Failed to write header to file: {}", e);
            return Err(ManagerError::Other(format!(
                "Failed to write header: {}",
                e
            )));
        }

        while {
            let sessions_guard = sessions.read().await;
            sessions_guard
                .get(&device_id)
                .map(|s| s.is_active)
                .unwrap_or(false)
        } {
            match receiver.recv().await {
                Ok(msg) => {
                    let timestamp = chrono::Utc::now();

                    if let Ok(bluerobotics_ping::Messages::Ping360(
                        bluerobotics_ping::ping360::Messages::AutoDeviceData(answer),
                    )) = bluerobotics_ping::Messages::try_from(&msg)
                    {
                        let decoded_measurement = serde_json::json!({
                            "timestamp": timestamp.to_rfc3339(),
                            "decoded_message": {
                                "type": "ping360_auto_device_data",
                                "data": answer
                            }
                        });

                        if let Err(e) = tokio::io::AsyncWriteExt::write_all(
                            &mut writer,
                            format!("{}\n", decoded_measurement.to_string()).as_bytes(),
                        )
                        .await
                        {
                            error!("Failed to write decoded Ping360 measurement to file: {}", e);
                            break;
                        }
                    }

                    if let Ok(bluerobotics_ping::Messages::Ping360(
                        bluerobotics_ping::ping360::Messages::Transducer(answer),
                    )) = bluerobotics_ping::Messages::try_from(&msg)
                    {
                        let decoded_measurement = serde_json::json!({
                            "timestamp": timestamp.to_rfc3339(),
                            "decoded_message": {
                                "type": "ping360_auto_device_data",
                                "data": answer
                            }
                        });

                        if let Err(e) = tokio::io::AsyncWriteExt::write_all(
                            &mut writer,
                            format!("{}\n", decoded_measurement.to_string()).as_bytes(),
                        )
                        .await
                        {
                            error!("Failed to write decoded Ping360 measurement to file: {}", e);
                            break;
                        }
                    }

                    if let Ok(bluerobotics_ping::Messages::Ping1D(
                        bluerobotics_ping::ping1d::Messages::Profile(answer),
                    )) = bluerobotics_ping::Messages::try_from(&msg)
                    {
                        let decoded_measurement = serde_json::json!({
                            "timestamp": timestamp.to_rfc3339(),
                            "decoded_message": {
                                "type": "ping1d_profile",
                                "data": answer
                            }
                        });

                        if let Err(e) = tokio::io::AsyncWriteExt::write_all(
                            &mut writer,
                            format!("{}\n", decoded_measurement.to_string()).as_bytes(),
                        )
                        .await
                        {
                            error!("Failed to write decoded Ping1D measurement to file: {}", e);
                            break;
                        }
                    }

                    if let Err(e) = writer.flush().await {
                        error!("Failed to flush writer: {}", e);
                        break;
                    }
                }
                Err(e) => {
                    error!("Failed to receive broadcasted message: {:?}", e);
                    break;
                }
            }
        }

        sessions.write().await.remove(&device_id);
        Ok(())
    }
}
