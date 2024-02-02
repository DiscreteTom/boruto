mod protocol;

use crate::protocol::Action;
use futures_util::StreamExt;
use http_body_util::Full;
use hyper::{
  body::{Bytes, Incoming},
  server::conn::http1,
  Request, Response,
};
use hyper_tungstenite::HyperWebsocket;
use hyper_util::rt::TokioIo;
use serde_json;
use std::{env, net::SocketAddr};
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::{Error, Message, Result};
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
        Err(e) => eprintln!(
          "Error getting window rect for pid({hwnd_pid}) and hwnd({}): {e:?}",
          hwnd.0
        ),
      }
    }
    return BOOL(0); // return false to stop enumerating
  }
  return BOOL(1); // return true to continue enumerating
}

/// Handle a HTTP or WebSocket request.
async fn handle_request(
  mut request: Request<Incoming>,
  peer: SocketAddr,
) -> Result<Response<Full<Bytes>>, Error> {
  // Check if the request is a websocket upgrade request.
  if hyper_tungstenite::is_upgrade_request(&request) {
    let (response, websocket) = hyper_tungstenite::upgrade(&mut request, None)?;

    // Spawn a task to handle the websocket connection.
    tokio::spawn(async move {
      if let Err(e) = serve_websocket(websocket, peer).await {
        eprintln!("Error in websocket connection: {e:?}");
      }
    });

    // Return the response so the spawned future can continue.
    Ok(response)
  } else {
    // Handle regular HTTP requests here.
    Ok(Response::new(Full::<Bytes>::from("Hello HTTP!")))
  }
}

async fn serve_websocket(ws: HyperWebsocket, peer: SocketAddr) -> Result<()> {
  let mut ws = ws.await?;

  println!("WebSocket Connected: {peer}");

  while let Some(msg) = ws.next().await {
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
              Err(e) => {
                // if the return code is 0, there is no error
                if e.code().0 != 0 {
                  eprintln!(
                    "Error enumerating windows for pid({}): {e:?}",
                    pid_payload.pid
                  )
                }
              }
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
                  Err(e) => {
                    eprintln!(
                      "Error getting window rect for pid({}), remove it. Error: {e:?}",
                      w.pid
                    );
                    to_be_removed.push(w.pid);
                    continue; // update next window
                  }
                  Ok(_) => (),
                }
                if let Err(e) = SetWindowPos(
                  w.hwnd,
                  None,
                  // use relative offset
                  offset.x + w.x,
                  offset.y + w.y,
                  // keep original size
                  rect.right - rect.left,
                  rect.bottom - rect.top,
                  SET_WINDOW_POS_FLAGS(0),
                ) {
                  eprintln!(
                    "Error setting window pos for pid({}), remove it. Error: {e:?}",
                    w.pid
                  );
                  to_be_removed.push(w.pid);
                }
              }

              // remove the windows that failed to update
              MANAGED_WINDOWS.retain(|w| !to_be_removed.contains(&w.pid));
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

  let mut http = http1::Builder::new();
  http.keep_alive(true);

  println!("Listening on: {addr}");

  loop {
    let (stream, peer) = listener.accept().await.expect("failed to accept");
    let connection = http
      .serve_connection(
        TokioIo::new(stream),
        hyper::service::service_fn(move |req| handle_request(req, peer)),
      )
      .with_upgrades();

    tokio::spawn(async move {
      if let Err(err) = connection.await {
        println!("Error serving HTTP connection: {err:?}");
      }
    });
  }
}
