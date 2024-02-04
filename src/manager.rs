use crate::protocol::{Action, PidsPayload, Reply, StatePayload};
use tokio::sync::{mpsc, watch};
use windows::Win32::{
  Foundation::{BOOL, HWND, LPARAM, RECT},
  UI::WindowsAndMessaging::{EnumWindows, GetWindowRect, GetWindowThreadProcessId, MoveWindow},
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
          // ensure the window is not already managed
          if MANAGED_WINDOWS.iter().any(|w| w.hwnd == hwnd) {
            return BOOL(0); // return false to stop enumerating
          }

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

pub async fn start_manager(mut action_rx: mpsc::Receiver<Action>, reply_tx: watch::Sender<Reply>) {
  loop {
    match action_rx.recv().await {
      None => break,
      Some(action) => match action {
        Action::Start => unsafe {
          STARTED = true;
          println!("Started");
          if let Err(e) = reply_tx.send(Reply::Started) {
            eprintln!("Error sending started reply: {e:?}");
            break;
          }
        },
        Action::Stop => unsafe {
          STARTED = false;
          println!("Stopped");
          if let Err(e) = reply_tx.send(Reply::Stopped) {
            eprintln!("Error sending stopped reply: {e:?}");
            break;
          }
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
          if let Err(e) = reply_tx.send(Reply::CurrentManagedPids(PidsPayload {
            pids: MANAGED_WINDOWS.iter().map(|w| w.pid).collect(),
          })) {
            eprintln!("Error sending current managed pids reply: {e:?}");
            break;
          }
        },
        Action::Remove(pid_payload) => unsafe {
          MANAGED_WINDOWS.retain(|w| w.pid != pid_payload.pid);
          println!("Removed window for pid({})", pid_payload.pid);
          if let Err(e) = reply_tx.send(Reply::CurrentManagedPids(PidsPayload {
            pids: MANAGED_WINDOWS.iter().map(|w| w.pid).collect(),
          })) {
            eprintln!("Error sending current managed pids reply: {e:?}");
            break;
          }
        },
        Action::RemoveAll => unsafe {
          MANAGED_WINDOWS.clear();
          println!("Removed all windows");
          if let Err(e) = reply_tx.send(Reply::CurrentManagedPids(PidsPayload {
            pids: MANAGED_WINDOWS.iter().map(|w| w.pid).collect(),
          })) {
            eprintln!("Error sending current managed pids reply: {e:?}");
            break;
          }
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
                  "Error setting window pos for pid({}), remove it. Error: {e:?}",
                  w.pid
                );
                to_be_removed.push(w.pid);
              }
            }

            // remove the windows that failed to update
            if to_be_removed.len() > 0 {
              MANAGED_WINDOWS.retain(|w| !to_be_removed.contains(&w.pid));
              if let Err(e) = reply_tx.send(Reply::CurrentManagedPids(PidsPayload {
                pids: MANAGED_WINDOWS.iter().map(|w| w.pid).collect(),
              })) {
                eprintln!("Error sending current managed pids reply: {e:?}");
                break;
              }
            }
          }
        },
        Action::Refresh => unsafe {
          if let Err(e) = reply_tx.send(Reply::State(StatePayload {
            started: STARTED,
            pids: MANAGED_WINDOWS.iter().map(|w| w.pid).collect(),
          })) {
            eprintln!("Error sending state reply: {e:?}");
            break;
          }
        },
      },
    }
  }
}
