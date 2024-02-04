use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePayload {
  pub x: i32,
  pub y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HwndPayload {
  pub hwnd: isize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
  #[serde(rename = "start")]
  Start,
  #[serde(rename = "stop")]
  Stop,
  #[serde(rename = "capture")]
  Capture,
  #[serde(rename = "add")]
  Add(HwndPayload),
  #[serde(rename = "remove")]
  Remove(HwndPayload),
  #[serde(rename = "removeAll")]
  RemoveAll,
  #[serde(rename = "update")]
  Update(UpdatePayload),
  /// Internal used to notify the manager to send Reply::State.
  Refresh,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HwndsPayload {
  pub hwnds: Vec<isize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatePayload {
  pub started: bool,
  pub hwnds: Vec<isize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Reply {
  #[serde(rename = "started")]
  Started,
  #[serde(rename = "stopped")]
  Stopped,
  #[serde(rename = "currentManagedHwnds")]
  CurrentManagedHwnds(HwndsPayload),
  #[serde(rename = "state")]
  State(StatePayload),
}
