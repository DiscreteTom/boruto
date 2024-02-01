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
  #[serde(rename = "add")]
  Add(PidPayload),
  #[serde(rename = "remove")]
  Remove(PidPayload),
  #[serde(rename = "removeAll")]
  RemoveAll,
  #[serde(rename = "update")]
  Update(UpdatePayload),
}
