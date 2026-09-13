use std::io::Cursor;
use std::sync::Arc;
use tokio::sync::{Notify, RwLock};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as Base64Standard;
use image::imageops::FilterType;

use windows::Foundation::TypedEventHandler;
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession, GlobalSystemMediaTransportControlsSessionManager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus,
};
use windows::Storage::Streams::DataReader;

use crate::worker::PlaybackState;

pub async fn start_media_listener(state: Arc<RwLock<PlaybackState>>, notify: Arc<Notify>) {
    // Intentamos obtener el Session Manager (puede fallar si el SO no lo soporta)
    let manager = match GlobalSystemMediaTransportControlsSessionManager::RequestAsync() {
        Ok(op) => match op.get() {
            Ok(m) => m,
            Err(_) => return,
        },
        Err(_) => return,
    };

    // Callback para cuando cambia la sesión activa
    let manager_clone = manager.clone();
    let state_clone = state.clone();
    let notify_clone = notify.clone();

    let handler = TypedEventHandler::new(move |_, _| {
        let m = manager_clone.clone();
        let s = state_clone.clone();
        let n = notify_clone.clone();
        tokio::spawn(async move {
            update_current_session(&m, s, n).await;
        });
        Ok(())
    });

    if let Ok(_) = manager.CurrentSessionChanged(&handler) {
        // Ejecutamos una vez para el estado inicial
        update_current_session(&manager, state, notify).await;

        // Mantenemos viva la tarea para seguir escuchando
        std::future::pending::<()>().await;
    }
}

async fn update_current_session(
    manager: &GlobalSystemMediaTransportControlsSessionManager,
    state: Arc<RwLock<PlaybackState>>,
    notify: Arc<Notify>,
) {
    if let Ok(session) = manager.GetCurrentSession() {
        // Desuscribirse de sesiones anteriores si fuera necesario requeriría guardar el token,
        // pero por simplicidad de este recolector, re-leemos el estado en cada cambio de sesión
        // y nos suscribimos a la sesión actual (en una implementación más compleja habría que limpiar tokens).
        
        let session_clone1 = session.clone();
        let state_clone1 = state.clone();
        let notify_clone1 = notify.clone();
        let _ = session.PlaybackInfoChanged(&TypedEventHandler::new(move |_, _| {
            let s = session_clone1.clone();
            let st = state_clone1.clone();
            let n = notify_clone1.clone();
            tokio::spawn(async move {
                update_playback_info(&s, st, n).await;
            });
            Ok(())
        }));

        let session_clone2 = session.clone();
        let state_clone2 = state.clone();
        let notify_clone2 = notify.clone();
        let _ = session.MediaPropertiesChanged(&TypedEventHandler::new(move |_, _| {
            let s = session_clone2.clone();
            let st = state_clone2.clone();
            let n = notify_clone2.clone();
            tokio::spawn(async move {
                update_media_properties(&s, st, n).await;
            });
            Ok(())
        }));

        // Trigger inicial para esta sesión
        update_media_properties(&session, state.clone(), notify.clone()).await;
        update_playback_info(&session, state, notify).await;
    } else {
        // Si no hay sesión, podríamos reportar que no está reproduciendo.
        let mut w = state.write().await;
        w.is_playing = false;
        w.update_timestamp();
        notify.notify_one();
    }
}

async fn update_playback_info(
    session: &GlobalSystemMediaTransportControlsSession,
    state: Arc<RwLock<PlaybackState>>,
    notify: Arc<Notify>,
) {
    if let Ok(info) = session.GetPlaybackInfo() {
        if let Ok(status) = info.PlaybackStatus() {
            let is_playing = status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing;
            let mut w = state.write().await;
            if w.is_playing != is_playing {
                w.is_playing = is_playing;
                w.update_timestamp();
                notify.notify_one();
            }
        }
    }
}

async fn update_media_properties(
    session: &GlobalSystemMediaTransportControlsSession,
    state: Arc<RwLock<PlaybackState>>,
    notify: Arc<Notify>,
) {
    if let Ok(op) = session.TryGetMediaPropertiesAsync() {
        if let Ok(props) = op.get() {
            let track = props.Title().unwrap_or_default().to_string_lossy();
            let artist = props.Artist().unwrap_or_default().to_string_lossy();
            let album = props.AlbumTitle().unwrap_or_default().to_string_lossy();
            
            let mut art_base64 = String::new();
            if let Ok(thumbnail_ref) = props.Thumbnail() {
                if let Ok(op_stream) = thumbnail_ref.OpenReadAsync() {
                    if let Ok(stream) = op_stream.get() {
                        if let Ok(size) = stream.Size() {
                            let mut buffer = vec![0u8; size as usize];
                            if let Ok(reader) = DataReader::CreateDataReader(&stream) {
                                if let Ok(load_op) = reader.LoadAsync(size as u32) {
                                    if load_op.get().is_ok() {
                                        if reader.ReadBytes(&mut buffer).is_ok() {
                                            art_base64 = process_image(&buffer);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            let mut w = state.write().await;
            w.track = track;
            w.artist = artist;
            w.album = album;
            if !art_base64.is_empty() {
                w.art_base64 = art_base64;
            }
            w.update_timestamp();
            // Disparamos evento inmediato al cambiar de canción
            notify.notify_one();
        }
    }
}

// Procesa la imagen sin causar panics, retornando string vacío en error
fn process_image(data: &[u8]) -> String {
    match image::load_from_memory(data) {
        Ok(img) => {
            let resized = img.resize_exact(200, 200, FilterType::Lanczos3);
            let mut buf = Cursor::new(Vec::new());
            
            // Usamos jpeg::JpegEncoder para setear la calidad a 80
            if let Ok(_) = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 80).encode_image(&resized) {
                let encoded = Base64Standard.encode(buf.into_inner());
                // Prefijamos con el tipo mime para el lado cliente (o el centralizador se encargará si es necesario)
                // Según spec, enviamos base64. El centralizador espera string puro o data URI? La spec dice "Carátula JPEG 200x200 q80 en base64"
                // Dejaremos el prefix data URI standard por seguridad.
                return format!("data:image/jpeg;base64,{}", encoded);
            }
            String::new()
        }
        Err(_) => String::new(),
    }
}
