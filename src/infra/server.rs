use crate::infra::env::Config;
use crate::infra::repository::slack::SocketMode;
use anyhow::{anyhow, Error, Result};
use tokio::{signal::ctrl_c, time::Duration};
use tokio_stream::wrappers::TcpListenerStream;
use tracing::{debug, error, info, warn};
use tracing_subscriber;
use tracing_subscriber::EnvFilter;

pub async fn run() -> Result<(), Error> {
    let config = Config::new()?;

    setup_tracing(&config)?;

    initialize_tls()?;

    let socket_mode = SocketMode::get_url(&config.slack_bot_socket_mode_token).await?;
    let socket = socket_mode
        .connect(&config.slack_bot_socket_mode_token)
        .await?;

    let _ = SocketMode::begin_stream(socket).await?;
    debug!("socket_mode result: {:?}", socket_mode);

    Ok(())
}

fn setup_tracing(config: &Config) -> Result<(), Error> {
    let log_filter = if config.stage.is_local() {
        EnvFilter::from_default_env() // We can use: error!(), warn!(), info!(), debug!()
            .add_directive("ama=debug".parse()?)
    } else {
        EnvFilter::from_default_env() // We can use: error!(), warn!(), info!()
            .add_directive("ama=info".parse()?)
    };

    tracing_subscriber::fmt()
        .json()
        .with_current_span(false)
        .flatten_event(true)
        .with_span_list(true)
        .with_file(true)
        .with_line_number(true)
        .with_env_filter(log_filter)
        .init();

    debug!("tracing_subscriber config is completed");
    Ok(())
}

// We need to run this one if tokio_tungstenite connect with wss://
fn initialize_tls() -> Result<(), Error> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .map_err(|_| anyhow!("failed to initialize rustls-tls-webpki-roots"))
}
