use ama::infra::server;
use anyhow::{Error, Result};

#[tokio::main]
async fn main() -> Result<(), Error> {
    println!("Hello, world!");

    let _ = server::run().await?;

    Ok(())
}
