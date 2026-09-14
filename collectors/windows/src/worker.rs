use serde::Serialize;
use std::env;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{RwLock, Notify};
use tokio::time::{sleep, Duration};

#[derive(Clone, Debug, Serialize)]
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

impl PlaybackState {
    pub fn new(device_id: String) -> Self {
        Self {
            device_id,
            track: String::new(),
            artist: String::new(),
            album: String::new(),
            art_base64: String::new(),
            is_playing: false,
            updated_at: 0,
            schema_version: 1,
        }
    }

    pub fn update_timestamp(&mut self) {
        self.updated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
    }
}

pub struct Worker {
    state: Arc<RwLock<PlaybackState>>,
    notify: Arc<Notify>,
    api_url: String,
    api_key: String,
}

impl Worker {
    pub fn new(state: Arc<RwLock<PlaybackState>>, notify: Arc<Notify>) -> Self {
        let api_url = env::var("CENTRALIZER_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
        let api_key = env::var("X_API_KEY").unwrap_or_default();
        Self { state, notify, api_url, api_key }
    }

    pub async fn run(self) {
        let client = reqwest::Client::new();
        let target_url = format!("{}/api/update", self.api_url.trim_end_matches('/'));

        loop {
            tokio::select! {
                _ = sleep(Duration::from_secs(20)) => {}
                _ = self.notify.notified() => {}
            }

            let payload = {
                let state = self.state.read().await;
                if state.track.is_empty() {
                    continue;
                }
                state.clone()
            };

            let res = client
                .post(&target_url)
                .header("X-API-Key", &self.api_key)
                .json(&payload)
                .send()
                .await;

            match res {
                Ok(response) => {
                    let status = response.status();
                    if status.is_success() {
                        println!("[OK] Aceptado en boveda: {} - {}", payload.track, payload.artist);
                    } else {
                        // Alerta roja: la boveda rechazo el paquete
                        println!("[ERROR ROJO] Boveda rechazo (HTTP {}): {} - {}", status, payload.track, payload.artist);
                    }
                }
                Err(e) => eprintln!("[FALLA NEGRA] El servidor esta muerto o la red cayo: {}", e),
            }
        }
    }
}
