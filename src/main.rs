#![allow(dead_code)]

mod cli;
mod files;
mod server;

use files::*;
use server::*;

#[tokio::main]
async fn main() {
    let mut cli = cli::CliState::new();
    cli.load_file("test.txt").await.unwrap();
    cli.set_character("Continue".to_string()).await.unwrap();
    cli.generate().await.unwrap();
    tokio::signal::ctrl_c().await.unwrap();
}
