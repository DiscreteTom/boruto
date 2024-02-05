mod manager;
mod protocol;
mod server;

use crate::protocol::Reply;
use manager::start_manager;
use server::start_server;
use std::env;
use tokio::sync::{mpsc, watch};

#[tokio::main]
async fn main() {
  let addr = env::args()
    .nth(1)
    .unwrap_or_else(|| "0.0.0.0:9002".to_string());

  let (action_tx, action_rx) = mpsc::channel(100);
  let (reply_tx, reply_rx) = watch::channel(Reply::Stopped);

  tokio::spawn(async move {
    start_manager(action_rx, reply_tx).await;
  });

  start_server(addr, action_tx, reply_rx).await;
}
