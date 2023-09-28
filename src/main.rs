#![allow(dead_code)]

mod cli;
mod files;
mod server;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    cli::Cli::new().run().await
}
