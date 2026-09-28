use serde::{Deserialize, Serialize};

use super::ClipSlot;
use crate::error::Result;
use crate::types::Color;

/// A scene as serialized by the Remote Script.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RawScene {
    pub id: String,
    pub name: String,
    pub color: Color,
}

live_object! {
    /// A row of Live's Session View.
    object Scene { ns: "scene", raw: RawScene }
}

props! { Scene {
    clip_slots: Vec<ClipSlot> [cached observe];
    color: Color [get set observe];
    color_index: i32 [get set observe];
    is_empty: bool [get];
    is_triggered: bool [get observe];
    name: String [get set observe];
    tempo: f64 [get set];
}}

impl Scene {
    /// Fires every clip slot of the scene and selects it.
    pub async fn fire(&self) -> Result<()> {
        self.handle.call("fire", None).await
    }
}
