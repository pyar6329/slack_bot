use anyhow::{bail, Error, Result};
use envy::Error as EnvyError;
use serde::de::Error as _;
use serde::Deserialize;
use strum::EnumIs;

#[derive(Deserialize, Debug, Copy, Clone, PartialEq, Eq, EnumIs)]
pub enum Stage {
    #[serde(rename = "dev")]
    Dev,
    #[serde(rename = "prod")]
    Prod,
    #[serde(other)]
    Local,
}

#[derive(Deserialize, Debug)]
pub struct Config {
    #[serde(default = "default_port")]
    pub port: u16, // tonic server's port
    #[serde(default = "default_stage")]
    pub stage: Stage,
    pub slack_bot_token: String,
    pub slack_bot_socket_mode_token: String,
    pub slack_bot_channel_id: String,
}

// gRPC serverのデフォルトport番号
const fn default_port() -> u16 {
    50051
}

const fn default_stage() -> Stage {
    Stage::Local
}

impl Config {
    pub fn new() -> Result<Config, Error> {
        let envs = envy::from_env::<Config>().map_err(Error::new)?;
        if envs.slack_bot_token.is_empty() {
            bail!(EnvyError::custom(
                "cannot set env as empty string: SLACK_BOT_TOKEN"
            ));
        }
        if envs.slack_bot_socket_mode_token.is_empty() {
            bail!(EnvyError::custom(
                "cannot set env as empty string: SLACK_BOT_SOCKET_MODE_TOKEN"
            ));
        }
        if envs.slack_bot_channel_id.is_empty() {
            bail!(EnvyError::custom(
                "cannot set env as empty string: SLACK_BOT_CHANNEL_ID"
            ));
        }

        Ok(envs)
    }
}
