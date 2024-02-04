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

async fn process_action(
  action_rx: &mut mpsc::Receiver<Action>,
  reply_tx: &watch::Sender<Reply>,
  started: &mut bool,
  managed_windows: &mut Vec<WindowState>,
) -> Result<(), String> {
  match action_rx.recv().await {
    None => Ok(()),
    Some(action) => match action {
      Action::Start => {
        *started = true;
        println!("Started");
        reply_tx
          .send(Reply::Started)
          .map_err(|e| format!("Error sending started reply: {e:?}"))
      }
      Action::Stop => {
        *started = false;
        println!("Stopped");
        reply_tx
          .send(Reply::Stopped)
          .map_err(|e| format!("Error sending stopped reply: {e:?}"))
      }
      Action::Capture => {
        let mut point = POINT::default();
        unsafe {
          match GetCursorPos(&mut point) {
            Err(e) => Err(format!("Error getting cursor pos: {e:?}")),
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
                  Ok(())
                }
                Err(e) => Err(format!(
                  "Error getting window rect for hwnd({}): {e:?}",
                  hwnd.0
                )),
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
        reply_tx
          .send(Reply::CurrentManagedHwnds(HwndsPayload {
            hwnds: managed_windows.iter().map(|w| w.hwnd.0).collect(),
          }))
          .map_err(|e| format!("Error sending current managed hwnds reply: {e:?}"))
      },
      Action::Remove(hwnd_payload) => {
        managed_windows.retain(|w| w.hwnd.0 != hwnd_payload.hwnd);
        println!("Removed window for hwnd({})", hwnd_payload.hwnd);
        reply_tx
          .send(Reply::CurrentManagedHwnds(HwndsPayload {
            hwnds: managed_windows.iter().map(|w| w.hwnd.0).collect(),
          }))
          .map_err(|e| format!("Error sending current managed hwnds reply: {e:?}"))
      }
      Action::RemoveAll => {
        managed_windows.clear();
        println!("Removed all windows");
        reply_tx
          .send(Reply::CurrentManagedHwnds(HwndsPayload {
            hwnds: managed_windows.iter().map(|w| w.hwnd.0).collect(),
          }))
          .map_err(|e| format!("Error sending current managed hwnds reply: {e:?}"))
      }
      Action::Update(offset) => unsafe {
        // only update when started
        if !*started {
          return Ok(());
        }

        let mut to_be_removed = Vec::new();
        for w in &*managed_windows {
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

          return reply_tx
            .send(Reply::CurrentManagedHwnds(HwndsPayload {
              hwnds: managed_windows.iter().map(|w| w.hwnd.0).collect(),
            }))
            .map_err(|e| format!("Error sending current managed hwnds reply: {e:?}"));
        }
        Ok(())
      },
      Action::Refresh => reply_tx
        .send(Reply::Refresh(StatePayload {
          started: *started,
          hwnds: managed_windows.iter().map(|w| w.hwnd.0).collect(),
        }))
        .map_err(|e| format!("Error sending state reply: {e:?}")),
    },
  }
}

pub async fn start_manager(mut action_rx: mpsc::Receiver<Action>, reply_tx: watch::Sender<Reply>) {
  let mut started = false;
  let mut managed_windows = Vec::new();

  loop {
    if let Err(e) = process_action(
      &mut action_rx,
      &reply_tx,
      &mut started,
      &mut managed_windows,
    )
    .await
    {
      eprintln!("{e}");
      break;
    }
  }
}
