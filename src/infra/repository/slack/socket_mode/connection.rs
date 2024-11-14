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

#[derive(Debug, Clone, Deserialize, EnumIs)]
#[serde(untagged)]
pub enum ReceivedStreamData {
    Hello(ReceivedStreamHello),
    Body(ReceivedStreamDataBody),
}

impl ReceivedStreamData {
    pub fn get_body(&self) -> Option<ReceivedStreamDataBody> {
        use ReceivedStreamData::*;
        match self {
            Body(body) => Some(body.to_owned()),
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

    // ref: https://www.klab.com/jp/blog/tech/2021/0201-slack.html
    // TODO: 受信したらACKを返さないと同じ文字列が何度も送られてくる
    pub async fn begin_stream(data: StreamData) -> Result<(), Error> {
        let mut stream = data.rx.await?;
        while let Some(s) = stream.next().await {
            match s {
                Ok(Message::Text(msg)) => {
                    let body = parse_body(&msg)?;
                    if let Some(body_data) = body.get_body() {
                        let ack = SendStreamAcknowledge::new(&body_data.id);
                        debug!("send ack: {:?}", ack);
                        let _ = stream
                            .send(Message::Text(serde_json::to_string(&ack)?))
                            .await;
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
