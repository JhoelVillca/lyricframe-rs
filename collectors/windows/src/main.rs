#![windows_subsystem = "windows"]

mod config;
mod media;
mod worker;

use std::sync::Arc;
use tokio::sync::{Notify, RwLock};
use worker::{PlaybackState, Worker};

#[tokio::main]
async fn main() {
    // Intentamos cargar el .env local resolviendo su ruta desde el ejecutable (útil para Autostart)
    if let Ok(mut path) = std::env::current_exe() {
        path.pop();
        path.push(".env");
        let _ = dotenvy::from_path(path);
    }

    // Zero-config: generar o recuperar device_id
    let device_id = config::get_or_create_device_id();

    // Silenciosamente registrar en el Registro de Windows para autostart
    config::setup_autostart();

    // Estado compartido y notificador de eventos
    let state = Arc::new(RwLock::new(PlaybackState::new(device_id)));
    let notify = Arc::new(Notify::new());

    // Iniciamos el worker en segundo plano (heartbeat y POST HTTP)
    let worker_state = state.clone();
    let worker_notify = notify.clone();
    tokio::spawn(async move {
        let worker = Worker::new(worker_state, worker_notify);
        worker.run().await;
    });

    // Arrancamos el listener nativo de Windows (este bloquea o se queda escuchando)
    media::start_media_listener(state, notify).await;
}
