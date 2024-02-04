use crate::protocol::{Action, HwndsPayload, Reply, StatePayload};
use tokio::sync::{mpsc, watch};
use windows::Win32::{
  Foundation::{HWND, POINT, RECT},
  UI::WindowsAndMessaging::{GetCursorPos, GetWindowRect, MoveWindow, WindowFromPoint},
};

#[derive(Debug)]
struct WindowState {
  pub hwnd: HWND,
  pub x: i32,
  pub y: i32,
}

pub async fn start_manager(mut action_rx: mpsc::Receiver<Action>, reply_tx: watch::Sender<Reply>) {
  let mut started = false;
  let mut managed_windows = Vec::new();

  loop {
    match action_rx.recv().await {
      None => break,
      Some(action) => match action {
        Action::Start => {
          started = true;
          println!("Started");
          if let Err(e) = reply_tx.send(Reply::Started) {
            eprintln!("Error sending started reply: {e:?}");
            break;
          }
        }
        Action::Stop => {
          started = false;
          println!("Stopped");
          if let Err(e) = reply_tx.send(Reply::Stopped) {
            eprintln!("Error sending stopped reply: {e:?}");
            break;
          }
        }
        Action::Capture => {
          let mut point = POINT::default();
          unsafe {
            match GetCursorPos(&mut point) {
              Err(e) => eprintln!("Error getting cursor pos: {e:?}"),
              Ok(()) => {
                let hwnd = WindowFromPoint(point);
                let mut rect = RECT::default();
                match GetWindowRect(hwnd, &mut rect) {
                  Ok(_) => {
                    // ensure the window is not already managed
                    if !managed_windows
                      .iter()
                      .any(|w: &WindowState| w.hwnd.0 == hwnd.0)
                    {
                      let state = WindowState {
                        hwnd,
                        // record the initial position
                        x: rect.left,
                        y: rect.top,
                      };
                      println!("Added window: {:?}", state);
                      managed_windows.push(state);
                    }
                  }
                  Err(e) => eprintln!("Error getting window rect for hwnd({}): {e:?}", hwnd.0),
                }
              }
            }
          }
        }
        Action::Add(hwnd_payload) => unsafe {
          let hwnd = HWND(hwnd_payload.hwnd);
          let mut rect = RECT::default();
          // TODO: prevent dup code
          match GetWindowRect(hwnd, &mut rect) {
            Ok(_) => {
              // ensure the window is not already managed
              if !managed_windows.iter().any(|w| w.hwnd.0 == hwnd.0) {
                let state = WindowState {
                  hwnd,
                  // record the initial position
                  x: rect.left,
                  y: rect.top,
                };
                println!("Added window: {:?}", state);
                managed_windows.push(state);
              }
            }
            Err(e) => eprintln!("Error getting window rect for hwnd({}): {e:?}", hwnd.0),
          }
          if let Err(e) = reply_tx.send(Reply::CurrentManagedHwnds(HwndsPayload {
            hwnds: managed_windows.iter().map(|w| w.hwnd.0).collect(),
          })) {
            eprintln!("Error sending current managed hwnds reply: {e:?}");
            break;
          }
        },
        Action::Remove(hwnd_payload) => {
          managed_windows.retain(|w| w.hwnd.0 != hwnd_payload.hwnd);
          println!("Removed window for hwnd({})", hwnd_payload.hwnd);
          if let Err(e) = reply_tx.send(Reply::CurrentManagedHwnds(HwndsPayload {
            hwnds: managed_windows.iter().map(|w| w.hwnd.0).collect(),
          })) {
            eprintln!("Error sending current managed hwnds reply: {e:?}");
            break;
          }
        }
        Action::RemoveAll => {
          managed_windows.clear();
          println!("Removed all windows");
          if let Err(e) = reply_tx.send(Reply::CurrentManagedHwnds(HwndsPayload {
            hwnds: managed_windows.iter().map(|w| w.hwnd.0).collect(),
          })) {
            eprintln!("Error sending current managed hwnds reply: {e:?}");
            break;
          }
        }
        Action::Update(offset) => unsafe {
          if started {
            let mut to_be_removed = Vec::new();
            for w in &managed_windows {
              let mut rect = RECT::default();
              match GetWindowRect(w.hwnd, &mut rect) {
                Err(e) => {
                  eprintln!(
                    "Error getting window rect for hwnd({}), remove it. Error: {e:?}",
                    w.hwnd.0
                  );
                  to_be_removed.push(w.hwnd.0);
                  continue; // update next window
                }
                Ok(_) => (),
              }
              if let Err(e) = MoveWindow(
                w.hwnd,
                // use relative offset
                offset.x + w.x,
                offset.y + w.y,
                // keep original size
                rect.right - rect.left,
                rect.bottom - rect.top,
                false,
              ) {
                eprintln!(
                  "Error setting window pos for hwnd({}), remove it. Error: {e:?}",
                  w.hwnd.0
                );
                to_be_removed.push(w.hwnd.0);
              }
            }

            // remove the windows that failed to update
            if to_be_removed.len() > 0 {
              managed_windows.retain(|w| !to_be_removed.contains(&w.hwnd.0));
              if let Err(e) = reply_tx.send(Reply::CurrentManagedHwnds(HwndsPayload {
                hwnds: managed_windows.iter().map(|w| w.hwnd.0).collect(),
              })) {
                eprintln!("Error sending current managed hwnds reply: {e:?}");
                break;
              }
            }
          }
        },
        Action::Refresh => {
          if let Err(e) = reply_tx.send(Reply::State(StatePayload {
            started,
            hwnds: managed_windows.iter().map(|w| w.hwnd.0).collect(),
          })) {
            eprintln!("Error sending state reply: {e:?}");
            break;
          }
        }
      },
    }
  }
}
