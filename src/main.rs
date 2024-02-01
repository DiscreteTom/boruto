mod protocol;

use crate::protocol::Action;
use futures_util::StreamExt;
use serde_json;
use std::{env, net::SocketAddr};
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
extern "system" fn enum_windows_callback(hwnd: HWND, l_param: LPARAM) -> BOOL {
  let pid = l_param.0 as u32; // the target pid
  let mut hwnd_pid = 0; // pid for the hwnd
  unsafe {
    // get pid by hwnd
    GetWindowThreadProcessId(hwnd, Some(&mut hwnd_pid));
  }
  if hwnd_pid == pid {
    unsafe {
      let mut rect = RECT::default();
      match GetWindowRect(hwnd, &mut rect) {
        Ok(_) => {
          let state = WindowState {
            hwnd,
            // record the initial position
            x: rect.left,
            y: rect.top,
            pid,
          };
          println!("Added window: {:?}", state);
          MANAGED_WINDOWS.push(state);
        }
        Err(_) => eprintln!(
          "Error getting window rect for pid({}) and hwnd({})",
          hwnd_pid, hwnd.0
        ),
      }
    }
    return BOOL(0); // return false to stop enumerating
  }
  return BOOL(1); // return true to continue enumerating
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

  println!("WebSocket Connected: {}", peer);

  while let Some(msg) = ws_stream.next().await {
    let msg = msg?;
    match msg {
      Message::Text(text) => {
        let action: Action = serde_json::from_str(&text).unwrap();
        match action {
          Action::Start => unsafe {
            STARTED = true;
            println!("Started");
          },
          Action::Stop => unsafe {
            STARTED = false;
            println!("Stopped");
          },
          Action::Add(pid_payload) => unsafe {
            // https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-enumwindows
            match EnumWindows(
              Some(enum_windows_callback),
              LPARAM(pid_payload.pid as isize),
            ) {
              Ok(_) => (),
              Err(_) => eprintln!("Error enumerating windows for pid({})", pid_payload.pid),
            }
          },
          Action::Remove(pid_payload) => unsafe {
            MANAGED_WINDOWS.retain(|w| w.pid != pid_payload.pid);
            println!("Removed window for pid({})", pid_payload.pid);
          },
          Action::RemoveAll => unsafe {
            MANAGED_WINDOWS.clear();
            println!("Removed all windows");
          },
          Action::Update(offset) => unsafe {
            if STARTED {
              let mut to_be_removed = Vec::new();
              for w in &MANAGED_WINDOWS {
                let mut rect = RECT::default();
                match GetWindowRect(w.hwnd, &mut rect) {
                  Err(_) => {
                    eprintln!("Error getting window rect for pid({}), remove it", w.pid);
                    to_be_removed.push(w.pid);
                    continue; // update next window
                  }
                  Ok(_) => (),
                }
                if let Err(_) = SetWindowPos(
                  w.hwnd,
                  HWND(0), // TODO: is this correct?
                  // use relative offset
                  offset.x + w.x,
                  offset.y + w.y,
                  // keep original size
                  rect.right - rect.left,
                  rect.bottom - rect.top,
                  SET_WINDOW_POS_FLAGS(0), // TODO: is this correct?
                ) {
                  eprintln!("Error setting window pos for pid({}), remove it", w.pid);
                  to_be_removed.push(w.pid);
                }
              }
            }
          },
        }
      }
      // other websocket message types are ignored
      _ => (),
    }
  }

  println!("WebSocket Disconnected: {}", peer);
  Ok(())
}

#[tokio::main]
async fn main() {
  let addr = env::args()
    .nth(1)
    .unwrap_or_else(|| "127.0.0.1:9002".to_string());
  let listener = TcpListener::bind(&addr).await.expect("Can't listen");

  println!("Listening on: {}", addr);

  while let Ok((stream, _)) = listener.accept().await {
    let peer = stream
      .peer_addr()
      .expect("connected streams should have a peer address");

    tokio::spawn(accept_connection(peer, stream));
  }
}
