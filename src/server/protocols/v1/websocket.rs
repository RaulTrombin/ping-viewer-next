use actix::{
    dev::ContextFutureSpawner, fut, Actor, ActorFutureExt, Addr, AsyncContext, Handler, Message,
    StreamHandler, WrapFuture,
};
use actix_web::HttpRequest;
use actix_web_actors::ws;
use lazy_static::lazy_static;
use paperclip::actix::{
    api_v2_operation, get,
    web::{self, HttpResponse},
    Apiv2Schema,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use tracing::info;
use uuid::Uuid;

use crate::device::manager::{Answer, ManagerActorHandler, Request};

pub struct StringMessage(String);

impl Message for StringMessage {
    type Result = ();
}

#[derive(Serialize, Debug)]
pub struct WebsocketError {
    pub error: String,
}

#[derive(Debug)]
pub struct WebsocketActorContent {
    pub actor: Addr<WebsocketActor>,
    pub re: Option<Regex>,
    pub device_number: Option<Uuid>,
}

#[derive(Debug, Default)]
pub struct WebsocketManager {
    pub clients: Vec<WebsocketActorContent>,
}

impl WebsocketManager {
    pub fn send(&self, value: &serde_json::Value, name: &str, device_number: Option<Uuid>) {
        if self.clients.is_empty() {
            return;
        }

        let string = serde_json::to_string(value).unwrap();
        for client in &self.clients {
            // check client list was subscribed or subscribed to all
            if client.device_number.is_none() || client.device_number == device_number {
                let is_match = client.re.as_ref().is_some_and(|regx| regx.is_match(name));
                if is_match {
                    client.actor.do_send(StringMessage(string.clone()));
                }
            }
        }
    }
}

lazy_static! {
    pub static ref MANAGER: Arc<Mutex<WebsocketManager>> =
        Arc::new(Mutex::new(WebsocketManager::default()));
}

pub fn send_to_websockets(message: Value, device: Option<Uuid>) {
    MANAGER
        .lock()
        .unwrap()
        .send(&message, &message.to_string(), device);
}

pub struct WebsocketActor {
    server: Arc<Mutex<WebsocketManager>>,
    pub filter: String,
    pub device_number: Option<Uuid>,
    pub manager_handler: web::Data<ManagerActorHandler>,
}

impl WebsocketActor {
    pub fn new(
        message_filter: String,
        device_number: Option<Uuid>,
        manager_handler: web::Data<ManagerActorHandler>,
    ) -> Self {
        Self {
            server: MANAGER.clone(),
            filter: message_filter,
            device_number,
            manager_handler,
        }
    }
}

impl Handler<StringMessage> for WebsocketActor {
    type Result = ();

    fn handle(&mut self, message: StringMessage, context: &mut Self::Context) {
        context.text(message.0);
    }
}

impl Actor for WebsocketActor {
    type Context = ws::WebsocketContext<Self>;
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WebsocketActor {
    fn started(&mut self, ctx: &mut Self::Context) {
        info!("ServerManager: Starting websocket client, add itself in manager.");
        self.server
            .lock()
            .unwrap()
            .clients
            .push(WebsocketActorContent {
                actor: ctx.address(),
                re: Regex::new(&self.filter).ok(),
                device_number: (self.device_number),
            });
    }

    fn finished(&mut self, ctx: &mut Self::Context) {
        info!("ServerManager: Finishing websocket, remove itself from manager.");
        self.server
            .lock()
            .unwrap()
            .clients
            .retain(|x| x.actor != ctx.address());
    }

    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(text)) => {
                let manager_requests: Vec<crate::ModuleType> = match serde_json::from_str(&text) {
                    Ok(requests) => requests,
                    Err(err) => match serde_json::from_str(&text) {
                        Ok(request) => vec![request],
                        Err(_) => {
                            let error_msg = format!("Error: {}", err);
                            ctx.text(error_msg);
                            return;
                        }
                    },
                };

                for request in manager_requests {
                    match request {
                        crate::ModuleType::DeviceManager(request) => {
                            let manager_handler = self.manager_handler.clone();

                            let request_has_id = match &request {
                                Request::ModifyDevice(modify) => Some(modify.uuid),
                                Request::Ping(device_request) => Some(device_request.uuid),
                                Request::Delete(uuid_wrapper) => Some(uuid_wrapper.uuid),
                                Request::Info(uuid_wrapper) => Some(uuid_wrapper.uuid),
                                Request::EnableContinuousMode(uuid_wrapper) => {
                                    Some(uuid_wrapper.uuid)
                                }
                                Request::DisableContinuousMode(uuid_wrapper) => {
                                    Some(uuid_wrapper.uuid)
                                }
                                _ => None,
                            };

                            let future =
                                async move { manager_handler.send(request).await }.into_actor(self);

                            future
                                .then(move |res, actor, ctx| {
                                    match &res {
                                        Ok(result) => {
                                            let device_number = match request_has_id{
                                                Some(device_number) => Some(device_number),
                                                None => actor.device_number,
                                            };
                                            crate::server::protocols::v1::websocket::send_to_websockets(
                                                json!(result),
                                                device_number,
                                            );
                                        }
                                        Err(err) => {
                                            ctx.text(serde_json::to_string_pretty(err).unwrap());
                                        }
                                    }
                                    fut::ready(())
                                })
                                .wait(ctx);
                        }
                    }
                }
            }
            Ok(ws::Message::Close(msg)) => ctx.close(msg),
            _ => (),
        }
    }
}

pub struct RecordingStatusActor {
    pub device_number: Option<Uuid>,
    pub manager_handler: web::Data<ManagerActorHandler>,
    recording_subscriber: broadcast::Receiver<crate::device::recording::RecordingSession>,
}

impl RecordingStatusActor {
    pub fn new(
        device_number: Option<Uuid>,
        manager_handler: web::Data<ManagerActorHandler>,
        recording_subscriber: broadcast::Receiver<crate::device::recording::RecordingSession>,
    ) -> Self {
        Self {
            device_number,
            manager_handler,
            recording_subscriber,
        }
    }
}

impl Actor for RecordingStatusActor {
    type Context = ws::WebsocketContext<Self>;
}

impl Handler<StringMessage> for RecordingStatusActor {
    type Result = ();

    fn handle(&mut self, message: StringMessage, ctx: &mut Self::Context) {
        ctx.text(message.0);
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for RecordingStatusActor {
    fn started(&mut self, ctx: &mut Self::Context) {
        info!("RecordingStatusActor: Starting websocket client");

        let addr = ctx.address();
        let mut subscriber = self.recording_subscriber.resubscribe();
        let device_number = self.device_number;
        // let manager_handler = self.manager_handler.clone();

        // Send current recording sessions
        // let addr_clone = addr.clone();
        // tokio::spawn(async move {
        //     if let Ok(Answer::RecordingManager(manager)) = manager_handler.send(Request::GetRecordingManager).await {
        //         let sessions = manager.get_all_recording_status().await;
        //         for session in sessions {
        //             if device_number.is_none() || device_number == Some(session.device_id) {
        //                 let _ = addr_clone.do_send(StringMessage(serde_json::to_string(&session).unwrap()));
        //             }
        //         }
        //     }
        // });

        // Handle future updates
        tokio::spawn(async move {
            while let Ok(session) = subscriber.recv().await {
                if device_number.is_none() || device_number == Some(session.device_id) {
                    let _ = addr.do_send(StringMessage(serde_json::to_string(&session).unwrap()));
                }
            }
        });
    }

    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Close(msg)) => ctx.close(msg),
            _ => (),
        }
    }
}

#[api_v2_operation(skip)]
#[get("ws")]
pub async fn websocket(
    req: HttpRequest,
    query: web::Query<WebsocketQuery>,
    stream: web::Payload,
    manager_handler: web::Data<ManagerActorHandler>,
) -> Result<HttpResponse, actix_web::Error> {
    let query_inner = query.into_inner();

    let filter = match query_inner.filter {
        Some(filter) => filter,
        _ => ".*".to_owned(),
    };
    let device_number = query_inner.device_number;

    if let Some(device_number) = device_number {
        let request = crate::device::manager::Request::Info(crate::device::manager::UuidWrapper {
            uuid: device_number,
        });
        match manager_handler.send(request).await {
            Ok(response) => {
                info!(
                    "ServerManager: Received websocket request connection for device: {response:?}"
                );
            }
            Err(err) => {
                return Ok(HttpResponse::InternalServerError().json(json!(err)));
            }
        }
    }

    ws::start(
        WebsocketActor::new(filter, device_number, manager_handler.clone()),
        &req,
        stream,
    )
}

#[api_v2_operation(skip)]
#[get("ws/recording")]
pub async fn recording_websocket(
    req: HttpRequest,
    query: web::Query<WebsocketQuery>,
    stream: web::Payload,
    manager_handler: web::Data<ManagerActorHandler>,
) -> Result<HttpResponse, actix_web::Error> {
    let query_inner = query.into_inner();
    let device_number = query_inner.device_number;

    if let Some(device_number) = device_number {
        let request = crate::device::manager::Request::Info(crate::device::manager::UuidWrapper {
            uuid: device_number,
        });
        match manager_handler.send(request).await {
            Ok(response) => {
                info!(
                    "RecordingStatusActor: Received websocket request connection for device: {response:?}"
                );
            }
            Err(err) => {
                return Ok(HttpResponse::InternalServerError().json(json!(err)));
            }
        }
    }

    let recording_manager = match manager_handler.send(Request::GetRecordingManager).await {
        Ok(Answer::RecordingManager(manager)) => manager,
        _ => {
            return Ok(HttpResponse::InternalServerError().json(json!({
                "error": "Failed to get recording manager"
            })))
        }
    };
    let subscriber = recording_manager.subscribe();

    ws::start(
        RecordingStatusActor::new(device_number, manager_handler.clone(), subscriber),
        &req,
        stream,
    )
}

#[derive(Deserialize, Apiv2Schema, Clone)]
pub struct WebsocketQuery {
    /// Regex filter to select the desired incoming messages
    filter: Option<String>,
    device_number: Option<Uuid>,
}
