use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlaybackState {
    pub device_id: String,
    pub track: String,
    pub artist: String,
    pub album: String,
    pub art_base64: String,
    pub is_playing: bool,
    pub updated_at: u64,
    pub schema_version: u32,
}

pub type AppState = Arc<RwLock<HashMap<String, PlaybackState>>>;

pub fn new_state() -> AppState {
    Arc::new(RwLock::new(HashMap::new()))
}

pub async fn get_current_state(state: &AppState) -> Option<PlaybackState> {
    let map = state.read().await;
    map.values()
        .max_by(|a, b| {
            a.updated_at.cmp(&b.updated_at)
                .then_with(|| b.device_id.cmp(&a.device_id))
        })
        .cloned()
}
