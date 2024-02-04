use crate::protocol::{Action, HwndsPayload, Reply, StatePayload};
use tokio::sync::{mpsc, watch};
use windows::Win32::{
  Foundation::{HWND, POINT, RECT},
  UI::WindowsAndMessaging::{GetCursorPos, GetParent, GetWindowRect, MoveWindow, WindowFromPoint},
};

#[derive(Debug)]
struct WindowState {
  pub hwnd: HWND,
  pub x: i32,
  pub y: i32,
}

struct ManagerState {
  pub started: bool,
  pub managed_windows: Vec<WindowState>,
}

pub async fn start_manager(mut action_rx: mpsc::Receiver<Action>, reply_tx: watch::Sender<Reply>) {
  let mut state = ManagerState {
    started: false,
    managed_windows: Vec::new(),
  };

  loop {
    if let Err(e) = process_action(&mut action_rx, &reply_tx, &mut state).await {
      eprintln!("{e}");
      break;
    }
  }
}

async fn process_action(
  action_rx: &mut mpsc::Receiver<Action>,
  reply_tx: &watch::Sender<Reply>,
  state: &mut ManagerState,
) -> Result<(), String> {
  match action_rx.recv().await {
    None => Ok(()),
    Some(action) => match action {
      Action::Start => {
        state.started = true;
        println!("Started");
        reply_tx
          .send(Reply::Started)
          .map_err(|e| format!("Error sending started reply: {e:?}"))
      }
      Action::Stop => {
        state.started = false;
        println!("Stopped");
        reply_tx
          .send(Reply::Stopped)
          .map_err(|e| format!("Error sending stopped reply: {e:?}"))
      }
      Action::Capture => {
        let mut point = POINT::default();
        unsafe {
          if let Err(e) = GetCursorPos(&mut point) {
            eprintln!("Error getting cursor pos: {e:?}");
            return Ok(());
          }

          // TODO: get parent hwnd?
          let hwnd = WindowFromPoint(point);
          let hwnd = GetParent(hwnd);
          add_hwnd_reply_current(hwnd, state, reply_tx)
        }
      }
      Action::Add(hwnd_payload) => add_hwnd_reply_current(HWND(hwnd_payload.hwnd), state, reply_tx),
      Action::Remove(hwnd_payload) => {
        state
          .managed_windows
          .retain(|w| w.hwnd.0 != hwnd_payload.hwnd);
        println!("Removed window for hwnd({})", hwnd_payload.hwnd);
        reply_current_managed_hwnds(reply_tx, state)
      }
      Action::RemoveAll => {
        state.managed_windows.clear();
        println!("Removed all windows");
        reply_current_managed_hwnds(reply_tx, state)
      }
      Action::Update(offset) => unsafe {
        // only update when started
        if !state.started {
          return Ok(());
        }

        let mut to_be_removed = Vec::new();
        for w in &state.managed_windows {
          let mut rect = RECT::default();
          // GetWindowRect is very fast, < 1ms
          if let Err(e) = GetWindowRect(w.hwnd, &mut rect) {
            eprintln!(
              "Error getting window rect for hwnd({}), remove it. Error: {e:?}",
              w.hwnd.0
            );
            to_be_removed.push(w.hwnd.0);
            continue; // update next window
          }
          // TODO: check if the window is still visible
          // TODO: check if there is any faster way to get window position
          // currently MoveWindow will take ~10ms
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

        // if no window failed to update, just return ok
        if to_be_removed.len() == 0 {
          return Ok(());
        }
        // else, remove the windows that failed to update and reply current
        state
          .managed_windows
          .retain(|w| !to_be_removed.contains(&w.hwnd.0));
        reply_current_managed_hwnds(reply_tx, state)
      },
      Action::Refresh => reply_tx
        .send(Reply::Refresh(StatePayload {
          started: state.started,
          hwnds: state.managed_windows.iter().map(|w| w.hwnd.0).collect(),
        }))
        .map_err(|e| format!("Error sending state reply: {e:?}")),
    },
  }
}

fn add_hwnd_reply_current(
  hwnd: HWND,
  state: &mut ManagerState,
  reply_tx: &watch::Sender<Reply>,
) -> Result<(), String> {
  let mut rect = RECT::default();

  if let Err(e) = unsafe { GetWindowRect(hwnd, &mut rect) } {
    eprintln!("Error getting window rect for hwnd({}): {e:?}", hwnd.0);
    return Ok(());
  }

  // skip if the window is already managed
  if state
    .managed_windows
    .iter()
    .any(|w: &WindowState| w.hwnd.0 == hwnd.0)
  {
    return Ok(());
  }

  let window_state = WindowState {
    hwnd,
    // record the initial position
    x: rect.left,
    y: rect.top,
  };
  println!("Added window: {:?}", window_state);
  state.managed_windows.push(window_state);
  reply_current_managed_hwnds(reply_tx, state)
}

fn reply_current_managed_hwnds(
  reply_tx: &watch::Sender<Reply>,
  state: &mut ManagerState,
) -> Result<(), String> {
  reply_tx
    .send(Reply::CurrentManagedHwnds(HwndsPayload {
      hwnds: state.managed_windows.iter().map(|w| w.hwnd.0).collect(),
    }))
    .map_err(|e| format!("Error sending current managed hwnds reply: {e:?}"))
}
