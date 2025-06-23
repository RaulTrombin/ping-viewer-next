use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
// #[schemars(description = "Ping1D message format")]
// pub struct Ping1DMessage {
//     #[schemars(description = "Timestamp of the message")]
//     pub timestamp: String,
//     #[schemars(description = "Type of the message")]
//     pub message_type: String,
//     // #[schemars(description = "Message data", type = "object")]
//     pub data: serde_json::Value,
// }

// #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
// #[schemars(description = "Ping360 message format")]
// pub struct Ping360Message {
//     #[schemars(description = "Timestamp of the message")]
//     pub timestamp: String,
//     #[schemars(description = "Type of the message")]
//     pub message_type: String,
//     // #[schemars(description = "Message data", type = "object")]
//     pub data: serde_json::Value,
// }

// #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
// #[schemars(description = "Common message format")]
// pub struct CommonMessage {
//     #[schemars(description = "Timestamp of the message")]
//     pub timestamp: String,
//     #[schemars(description = "Type of the message")]
//     pub message_type: String,
//     // #[schemars(description = "Message data", type = "object")]
//     pub data: serde_json::Value,
// }

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Recording header format")]
pub struct RecordingHeader {
    #[schemars(description = "Version of the recording format")]
    pub version: String,
    #[schemars(description = "Device information")]
    pub device_info: DeviceInfo,
    #[schemars(description = "File format")]
    pub file_format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Device information format")]
pub struct DeviceInfo {
    #[schemars(description = "Device ID")]
    pub device_id: String,
    #[schemars(description = "Start time of recording")]
    pub start_time: String,
    // #[schemars(description = "Device properties", type = "object")]
    pub device_properties: serde_json::Value,
}

// #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
// #[schemars(description = "Device information format")]
// pub struct Potato {
//     #[schemars(description = "Betroot")]
//     pub timestamp: String,
//     #[schemars(description = "Type of the message")]
//     pub message_type: String,
// }