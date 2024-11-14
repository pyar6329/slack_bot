use anyhow::{Error, Result};
use reqwest::{header::AUTHORIZATION, Client};
use serde::Deserialize;
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
use url::Url;

#[derive(Debug, Deserialize)]
pub struct SocketMode {
    #[serde(rename = "url")]
    pub url: Url,
}

// pub struct StreamData<T> {
//     tx: oneshot::Sender<T>,
//     rx: oneshot::Receiver<T>,
// }

type StreamDataType = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

pub struct StreamData {
    //   tx: oneshot::Sender<StreamDataType>,
    rx: oneshot::Receiver<StreamDataType>,
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

        tracing::debug!("url is: {}", &body.url.to_string().as_str());

        Ok(body)
    }

    pub async fn connect(&self, socket_mode_token: &str) -> Result<StreamData, Error> {
        let request = WebSocketBuilder::new(self.url.to_string().parse()?);

        println!("aaaaaaaaaaaaaaa");
        let (mut stream, response) = connect_async(request).await?;
        println!("bbbbbbbbbbbbb");

        let (tx, rx) = oneshot::channel();

        // let _ = tx.send(stream)?;
        tokio::spawn(async move {
            let _ = tx.send(stream);
            println!("cccccccccc");
        });

        println!("dddddddddddd");

        let stream_data = StreamData { rx }; // ここでmutのstreamをtxに入れて渡したい
        Ok(stream_data)

        // Ok(())
    }

    pub async fn begin_stream(data: StreamData) -> Result<(), Error> {
        println!("eeeeeeeeeeee");
        let mut stream = data.rx.await?;
        println!("fffffffffffffffff");
        //       let bbb = stream.next().await;
        while let Some(s) = stream.next().await {
            match s {
                Ok(Message::Text(msg)) => {
                    println!("Received a message: {:?}", msg);
                }
                Ok(Message::Binary(msg)) => {
                    println!("Received a message: {:?}", msg);
                }
                Ok(Message::Ping(msg)) => {
                    println!("Received a message: {:?}", msg);
                }
                Ok(Message::Pong(msg)) => {
                    println!("Received a message: {:?}", msg);
                }
                Ok(Message::Close(msg)) => {
                    println!("Received a message: {:?}", msg);
                }
                Ok(Message::Frame(msg)) => {
                    println!("Received a message: {:?}", msg);
                }
                Err(e) => {
                    println!("Error: {:?}", e);
                }
            }
        }
        // let (write, read) = stream.split();
        println!("ggggggggggggg");

        Ok(())
    }
}
