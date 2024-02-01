mod protocol;

use futures_util::{SinkExt, StreamExt};
use serde_json;
// use log::*;
use crate::protocol::Action;
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{
  accept_async,
  tungstenite::{Error, Message, Result},
};
use windows::Win32::{
  Foundation::{BOOL, HWND, LPARAM},
  UI::WindowsAndMessaging::{
    EnumWindows, GetWindowThreadProcessId, SetWindowPos, SET_WINDOW_POS_FLAGS,
  },
};

static mut HWNDS: HWND = HWND(0);

// https://learn.microsoft.com/en-us/previous-versions/windows/desktop/legacy/ms633498(v=vs.85)
extern "system" fn callback(hwnd: HWND, l_param: LPARAM) -> BOOL {
  let mut process_id = 0;
  unsafe {
    GetWindowThreadProcessId(hwnd, Some(&mut process_id));
  }
  if process_id == (l_param.0 as u32) {
    unsafe {
      HWNDS = hwnd;
      println!("Found window: {:?}", hwnd);
    }
    return BOOL(0); // return false to stop enumerating
  }
  return BOOL(1);
}

async fn accept_connection(peer: SocketAddr, stream: TcpStream) {
  if let Err(e) = handle_connection(peer, stream).await {
    match e {
      Error::ConnectionClosed | Error::Protocol(_) | Error::Utf8 => (),
      _ => eprintln!("Error processing connection: {}", e),
      //   err => error!("Error processing connection: {}", err),
    }
  }
}

async fn handle_connection(peer: SocketAddr, stream: TcpStream) -> Result<()> {
  let mut ws_stream = accept_async(stream).await.expect("Failed to accept");

  //   info!("New WebSocket connection: {}", peer);

  while let Some(msg) = ws_stream.next().await {
    let msg = msg?;
    // if msg.is_text() || msg.is_binary() {
    //   ws_stream.send(msg).await?;
    // }
    match msg {
      Message::Text(text) => {
        let action: Action = serde_json::from_str(&text).unwrap();
        match action {
          Action::Update(offset) => unsafe {
            // EnumWindows(Some(callback), 123);
            SetWindowPos(
              HWNDS,
              HWND(0),
              offset.x,
              offset.y,
              0,
              0,
              SET_WINDOW_POS_FLAGS(0),
            );
          },
        }
        // println!("Action: {:?}", action);
      }
      _ => (),
    }
  }

  Ok(())
}

#[tokio::main]
async fn main() {
  unsafe {
    // https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-enumwindows
    EnumWindows(Some(callback), LPARAM(27592));
  }

  //   env_logger::init();

  let addr = "127.0.0.1:9002";
  let listener = TcpListener::bind(&addr).await.expect("Can't listen");
  //   info!("Listening on: {}", addr);

  while let Ok((stream, _)) = listener.accept().await {
    let peer = stream
      .peer_addr()
      .expect("connected streams should have a peer address");
    // info!("Peer address: {}", peer);

    tokio::spawn(accept_connection(peer, stream));
  }
}
