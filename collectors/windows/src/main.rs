// #![windows_subsystem = "windows"]

mod config;
mod media;
mod worker;

use std::sync::Arc;
use tokio::sync::{Notify, RwLock};
use worker::{PlaybackState, Worker};

#[tokio::main]
async fn main() {
    let mut env_loaded = false;
    if let Ok(mut path) = std::env::current_exe() {
        path.pop();
        path.push(".env");
        if dotenvy::from_path(path).is_ok() {
            env_loaded = true;
        }
    }

    if !env_loaded {
        dotenvy::dotenv().ok();
    }

    let device_id = config::get_or_create_device_id();

    config::setup_autostart();

    let state = Arc::new(RwLock::new(PlaybackState::new(device_id)));
    let notify = Arc::new(Notify::new());

    let worker_state = state.clone();
    let worker_notify = notify.clone();
    tokio::spawn(async move {
        let worker = Worker::new(worker_state, worker_notify);
        worker.run().await;
    });

    media::start_media_listener(state, notify).await;
}
