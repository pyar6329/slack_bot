use anyhow::{Error, Result};
use futures_util::sink::SinkExt;
use reqwest::{header::AUTHORIZATION, Client};
use serde::{Deserialize, Serialize};
use strum::EnumIs;
use tokio::sync::oneshot;
use tokio_stream::StreamExt;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{
        client::ClientRequestBuilder as WebSocketBuilder,
        // client::IntoClientRequest,
        http::{Method, Request},
        protocol::Message,
    },
    MaybeTlsStream, WebSocketStream,
};
use tracing::debug;
use url::Url;

#[derive(Debug, Deserialize)]
pub struct SocketMode {
    #[serde(rename = "url")]
    pub url: Url,
}

type StreamDataType = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

pub struct StreamData {
    pub rx: oneshot::Receiver<StreamDataType>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReceivedStreamDataBody {
    #[serde(rename = "envelope_id")]
    pub id: String,
    pub payload: ReceivedStreamDataPayload,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReceivedStreamDataPayload {
    pub event_id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub event: ReceivedStreamDataEvent,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ReceivedStreamDataEvent {
    User(ReceivedStreamDataEventForUser),
    Application(ReceivedStreamDataEventForApplication),
    Bot(ReceivedStreamDataEventForBot),
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReceivedStreamDataEventForUser {
    #[serde(rename = "user")]
    pub user_id: String,
    #[serde(rename = "type")]
    pub category: String,
    #[serde(rename = "ts")]
    pub create_timestamp: String,
    #[serde(rename = "client_msg_id")]
    pub message_id: String,
    #[serde(rename = "text")]
    pub content: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReceivedStreamDataEventForApplication {
    pub bot_id: String,
    #[serde(rename = "type")]
    pub category: String,
    #[serde(rename = "subtype")]
    pub sub_category: String,
    #[serde(rename = "ts")]
    pub create_timestamp: String,
    #[serde(rename = "text")]
    pub content: String,
    #[serde(default)]
    pub attachments: Vec<ReceivedStreamDataEventForApplicationAttachment>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReceivedStreamDataEventForApplicationAttachment {
    pub title: String,
    #[serde(rename = "text")]
    pub content: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReceivedStreamDataEventForBot {
    pub bot_id: String,
    #[serde(rename = "type")]
    pub category: String,
    #[serde(rename = "ts")]
    pub create_timestamp: String,
    #[serde(rename = "text")]
    pub content: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReceivedStreamHello {
    #[serde(rename = "type")]
    pub category: String,
    pub num_connections: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReceivedStreamDisconnect {
    #[serde(rename = "type")]
    pub category: String,
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReceivedStreamShashCommandPayload {
    pub trigger_id: String,
    pub command: String,
    #[serde(rename = "text")]
    pub command_args: String,
    // pub user_id: String,
    pub user_name: String,
    pub channel_id: String,
    // pub channel_name: String,
    // pub token: String
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReceivedStreamShashCommand {
    #[serde(rename = "envelope_id")]
    pub id: String,
    pub payload: ReceivedStreamShashCommandPayload,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReceivedStreamInteractivePayload {
    #[serde(rename = "type")]
    pub category: String, //view_submission以外にも入ってきそう
    pub view: ReceivedStreamInteractivePayloadView,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReceivedStreamInteractivePayloadView {
    pub state: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize, EnumIs)]
#[serde(tag = "type")]
// {"type":"hello"}, {"type":"events_api"}, {"type":"disconnect"} の条件分岐をserde(tag = "type")で行う
pub enum ReceivedStreamData {
    #[serde(alias = "hello")]
    Hello { num_connections: u32 },
    #[serde(alias = "events_api")]
    EventsApi {
        #[serde(rename = "envelope_id")]
        id: String,
        payload: ReceivedStreamDataPayload,
    },
    #[serde(alias = "disconnect")]
    Disconnect { reason: String },
    #[serde(alias = "slash_commands")]
    SlashCommand {
        #[serde(rename = "envelope_id")]
        id: String,
        payload: ReceivedStreamShashCommandPayload,
    },
    #[serde(alias = "interactive")]
    Interactive {
        #[serde(rename = "envelope_id")]
        id: String,
        payload: ReceivedStreamInteractivePayload,
    },
}

impl ReceivedStreamData {
    pub fn get_body(&self) -> Option<ReceivedStreamDataBody> {
        use ReceivedStreamData::*;
        match self {
            EventsApi { id, payload } => Some(ReceivedStreamDataBody {
                id: id.to_owned(),
                payload: payload.to_owned(),
            }),
            _ => None,
        }
    }

    pub fn get_command(&self) -> Option<ReceivedStreamShashCommand> {
        use ReceivedStreamData::*;
        match self {
            SlashCommand { id, payload } => Some(ReceivedStreamShashCommand {
                id: id.to_owned(),
                payload: payload.to_owned(),
            }),
            _ => None,
        }
    }

    pub fn get_id(&self) -> Option<String> {
        use ReceivedStreamData::*;
        match self {
            EventsApi { id, .. } => Some(id.to_owned()),
            SlashCommand { id, .. } => Some(id.to_owned()),
            Interactive { id, .. } => Some(id.to_owned()),
            _ => None,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SendStreamAcknowledge {
    #[serde(rename = "envelope_id")]
    pub id: String,
}

impl SendStreamAcknowledge {
    pub fn new(id: &str) -> Self {
        Self { id: id.to_string() }
    }
}

impl SocketMode {
    pub async fn get_url(socket_mode_token: &str) -> Result<Self, Error> {
        let client = Client::new();
        let body_bytes = client
            .post("https://slack.com/api/apps.connections.open")
            .header(AUTHORIZATION, format!("Bearer {}", socket_mode_token))
            .send()
            .await?
            .bytes()
            .await?;

        let body: SocketMode = serde_json::from_slice(&body_bytes)?;

        debug!("websocket url is: {}", &body.url.to_string().as_str());

        Ok(body)
    }

    pub async fn connect(&self) -> Result<StreamData, Error> {
        let request = WebSocketBuilder::new(self.url.to_string().parse()?);

        let (stream, _response) = connect_async(request).await?;

        let (tx, rx) = oneshot::channel();

        tokio::spawn(async move {
            let _ = tx.send(stream);
        });

        let stream_data = StreamData { rx }; // ここでmutのstreamをtxに入れて渡したい
        Ok(stream_data)
    }

    // ref: https://api.slack.com/apis/socket-mode#connect
    // TODO: 複数接続して切断しても別のstreamを使う感じにするといいっぽい
    // ref: https://www.klab.com/jp/blog/tech/2021/0201-slack.html
    pub async fn begin_stream(data: StreamData, event_token: &str) -> Result<(), Error> {
        let mut stream = data.rx.await?;
        while let Some(s) = stream.next().await {
            match s {
                Ok(Message::Text(msg)) => {
                    let body = parse_body(&msg)?;
                    if let Some(id) = body.get_id() {
                        let ack = SendStreamAcknowledge::new(&id);
                        debug!("send ack: {:?}", ack);
                        // 受信したら3秒以内にACKを送信しないと同じメッセージが何度も送られてくる
                        let _ = stream
                            .send(Message::Text(serde_json::to_string(&ack)?))
                            .await;
                    }
                    if let Some(command) = body.get_command() {
                        use crate::infra::repository::slack::create_modal;
                        let _ = create_modal(&event_token, &command.payload.trigger_id).await?;
                    }
                    if body.is_disconnect() {
                        debug!("disconnect message received");
                        // ここで再接続処理の実装
                        break;
                    }
                    println!("Received a text message: {:?}", body);
                }
                Ok(Message::Binary(msg)) => {
                    println!("Received a binary message: {:?}", msg);
                }
                Ok(Message::Ping(msg)) => {
                    println!("Received a ping message: {:?}", msg);
                }
                Ok(Message::Pong(msg)) => {
                    println!("Received a pong message: {:?}", msg);
                }
                Ok(Message::Close(msg)) => {
                    println!("Received a close message: {:?}", msg);
                }
                Ok(Message::Frame(msg)) => {
                    println!("Received a frame message: {:?}", msg);
                }
                Err(e) => {
                    println!("Received websocket Error: {:?}", e);
                }
            }
        }
        // let (write, read) = stream.split();

        Ok(())
    }
}

fn parse_body(body: &str) -> Result<ReceivedStreamData, Error> {
    println!("Received a messageeeeeeeeeeeeeeeee: {:?}", body);
    let data: ReceivedStreamData = serde_json::from_str(body)?;
    Ok(data)
}
