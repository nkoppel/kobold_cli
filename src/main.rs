#![allow(dead_code)]

mod cli;
mod files;
mod server;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let mut cli = cli::CliState::new();
    cli.run().await
}
