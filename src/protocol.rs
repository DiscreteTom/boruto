use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePayload {
  pub x: i32,
  pub y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
  #[serde(rename = "update")]
  Update(UpdatePayload),
}
