mod protocol;

use crate::protocol::Action;
use futures_util::StreamExt;
use serde_json;
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{
  accept_async,
  tungstenite::{Error, Message, Result},
};
use windows::Win32::{
  Foundation::{BOOL, HWND, LPARAM, RECT},
  UI::WindowsAndMessaging::{
    EnumWindows, GetWindowRect, GetWindowThreadProcessId, SetWindowPos, SET_WINDOW_POS_FLAGS,
  },
};

#[derive(Debug)]
struct WindowState {
  pub hwnd: HWND,
  pub pid: u32,
  pub x: i32,
  pub y: i32,
}

static mut MANAGED_WINDOWS: Vec<WindowState> = Vec::new();
static mut STARTED: bool = false;

// https://learn.microsoft.com/en-us/previous-versions/windows/desktop/legacy/ms633498(v=vs.85)
extern "system" fn callback(hwnd: HWND, l_param: LPARAM) -> BOOL {
  let mut pid = 0;
  unsafe {
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
  }
  if pid == (l_param.0 as u32) {
    unsafe {
      MANAGED_WINDOWS.push(WindowState {
        hwnd,
        x: 0,
        y: 0,
        pid,
      });
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
    }
  }
}

async fn handle_connection(peer: SocketAddr, stream: TcpStream) -> Result<()> {
  let mut ws_stream = accept_async(stream).await.expect("Failed to accept");

  while let Some(msg) = ws_stream.next().await {
    let msg = msg?;
    // if msg.is_text() || msg.is_binary() {
    //   ws_stream.send(msg).await?;
    // }
    match msg {
      Message::Text(text) => {
        let action: Action = serde_json::from_str(&text).unwrap();
        match action {
          Action::Start => unsafe {
            STARTED = true;
          },
          Action::Stop => unsafe {
            STARTED = false;
          },
          Action::Add(pid_payload) => unsafe {
            // https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-enumwindows
            EnumWindows(Some(callback), LPARAM(pid_payload.pid as isize));
            // TODO: handle error
            let mut lprect = RECT::default();
            let last = MANAGED_WINDOWS.last_mut().unwrap();
            GetWindowRect(last.hwnd, &mut lprect);
            last.x = lprect.left;
            last.y = lprect.top;
            println!("Added window: {:?}", last);
          },
          Action::Remove(pid_payload) => unsafe {
            MANAGED_WINDOWS.retain(|w| w.pid != pid_payload.pid);
          },
          Action::RemoveAll => unsafe {
            MANAGED_WINDOWS.clear();
          },
          Action::Update(offset) => unsafe {
            if STARTED {
              for w in &MANAGED_WINDOWS {
                let mut lprect = RECT::default();
                GetWindowRect(w.hwnd, &mut lprect);
                SetWindowPos(
                  w.hwnd,
                  HWND(0),
                  offset.x,
                  offset.y,
                  lprect.right - lprect.left,
                  lprect.bottom - lprect.top,
                  SET_WINDOW_POS_FLAGS(0),
                );
              }
            }
          },
          // TODO
          _ => (),
        }
      }
      _ => (),
    }
  }

  Ok(())
}

#[tokio::main]
async fn main() {
  let addr = "127.0.0.1:9002";
  let listener = TcpListener::bind(&addr).await.expect("Can't listen");

  while let Ok((stream, _)) = listener.accept().await {
    let peer = stream
      .peer_addr()
      .expect("connected streams should have a peer address");

    tokio::spawn(accept_connection(peer, stream));
  }
}
