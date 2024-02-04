use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePayload {
  pub x: i32,
  pub y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PidPayload {
  pub pid: u32,
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
  Add(PidPayload),
  #[serde(rename = "remove")]
  Remove(PidPayload),
  #[serde(rename = "removeAll")]
  RemoveAll,
  #[serde(rename = "update")]
  Update(UpdatePayload),
  /// Internal used to notify the manager to send Reply::State.
  Refresh,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PidsPayload {
  pub pids: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatePayload {
  pub started: bool,
  pub pids: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Reply {
  #[serde(rename = "started")]
  Started,
  #[serde(rename = "stopped")]
  Stopped,
  #[serde(rename = "currentManagedPids")]
  CurrentManagedPids(PidsPayload),
  #[serde(rename = "state")]
  State(StatePayload),
}
