use askama::Template;
use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use base64::{engine::general_purpose, Engine as _};
use color_thief::{get_palette, ColorFormat};
use dotenvy::var;
use reqwest::Client;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpListener;
use tokio::sync::RwLock;

struct Barra {
    altura: u32,
    delay: f32,
}

#[derive(Template)]
#[template(path = "widget.svg")]
struct WidgetTemplate {
    css_barras_eq: String,
    color_fondo_final: String,
    paleta_color_1: String,
    paleta_color_2: String,
    paleta_color_3: String,
    color_borde: String,
    color_texto_contraste: String,
    url_cancion: String,
    url_artista: String,
    imagen_b64: String,
    estado: String,
    cancion: String,
    artista: String,
    album: String,
    barras: Vec<Barra>,
}

struct CachedToken {
    access_token: String,
    expires_at: Instant,
}

struct AppState {
    http_client: Client,
    client_id: String,
    client_secret: String,
    refresh_token: String,
    token_cache: RwLock<Option<CachedToken>>,
}



// --- LOGICA DE COLOR ---
fn rgb_a_hex(color: (u8, u8, u8)) -> String {
    format!("#{:02x}{:02x}{:02x}", color.0, color.1, color.2)
}

fn mezclar_con_blanco(color: (u8, u8, u8), factor: f32) -> (u8, u8, u8) {
    let r = (color.0 as f32 + (255.0 - color.0 as f32) * factor).min(255.0) as u8;
    let g = (color.1 as f32 + (255.0 - color.1 as f32) * factor).min(255.0) as u8;
    let b = (color.2 as f32 + (255.0 - color.2 as f32) * factor).min(255.0) as u8;
    (r, g, b)
}

fn es_color_claro(color: (u8, u8, u8), umbral: f32) -> bool {
    let brillo = 0.299 * (color.0 as f32) + 0.587 * (color.1 as f32) + 0.114 * (color.2 as f32);
    brillo > umbral
}

fn extraer_paleta_y_fondo(bytes_imagen: &[u8]) -> (Vec<(u8, u8, u8)>, String, String) {
    let default_palette = vec![(80, 80, 80), (120, 120, 120), (160, 160, 160), (200, 200, 200)];
    let mut paleta = default_palette.clone();
    let mut color_fondo_final = String::from("#181414");
    let mut color_texto_contraste = String::from("#FAFAFA");

    if let Ok(img) = image::load_from_memory(bytes_imagen) {
        let rgb_img = img.to_rgb8();
        let pixels = rgb_img.into_raw();
        
        if let Ok(p) = get_palette(&pixels, ColorFormat::Rgb, 10, 4) {
            paleta = p.into_iter().map(|c| (c.r, c.g, c.b)).collect();
            
            let color_dominante = paleta[0];
            let color_mezclado = mezclar_con_blanco(color_dominante, 0.20);
            color_fondo_final = rgb_a_hex(color_mezclado);
            
            if es_color_claro(color_mezclado, 135.0) {
                color_texto_contraste = String::from("#222222");
            }
        }
    }

    while paleta.len() < 4 {
        paleta.push((255, 255, 255));
    }

    (paleta, color_fondo_final, color_texto_contraste)
}


async fn get_access_token(state: &AppState) -> Option<String> {
    {
        let cache = state.token_cache.read().await;
        if let Some(cached) = &*cache {
            if cached.expires_at > Instant::now() {
                return Some(cached.access_token.clone());
            }
        }
    } 

    let auth = general_purpose::STANDARD.encode(format!("{}:{}", state.client_id, state.client_secret));
    let params = [
        ("grant_type", "refresh_token"),
        ("refresh_token", &state.refresh_token),
    ];

    let url_token = "https://accounts.spotify.com/api/token";

    let res = state.http_client.post(url_token)
        .header("Authorization", format!("Basic {}", auth))
        .form(&params)
        .send()
        .await
        .ok()?;

    let json: serde_json::Value = res.json().await.ok()?;
    let new_token = json["access_token"].as_str()?.to_string();
    let expires_in = json["expires_in"].as_u64().unwrap_or(3600);

    let mut cache = state.token_cache.write().await;
    *cache = Some(CachedToken {
        access_token: new_token.clone(),
        expires_at: Instant::now() + Duration::from_secs(expires_in - 60),
    });

    Some(new_token)
}

async fn get_image_data(client: &Client, url: &str) -> (String, Vec<u8>) {
    if url.is_empty() {
        return (String::new(), Vec::new());
    }
    let Ok(res) = client.get(url).send().await else { return (String::new(), Vec::new()) };
    let Ok(bytes) = res.bytes().await else { return (String::new(), Vec::new()) };
    
    let b64 = format!("data:image/jpeg;base64,{}", general_purpose::STANDARD.encode(&bytes));
    (b64, bytes.to_vec())
}

async fn renderizar_widget(State(state): State<Arc<AppState>>) -> Result<impl IntoResponse, StatusCode> {
    let token = get_access_token(&state).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let mut track_data = serde_json::Value::Null;
    let mut is_playing = false;

    let url_current = "https://api.spotify.com/v1/me/player/currently-playing";
    let url_recent = "https://api.spotify.com/v1/me/player/recently-played?limit=1";

    if let Ok(res) = state.http_client.get(url_current)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await 
    {
        if res.status().as_u16() == 200 {
            if let Ok(mut json) = res.json::<serde_json::Value>().await {
                track_data = json["item"].take();
                is_playing = json["is_playing"].as_bool().unwrap_or(false);
            }
        }
    }

    if track_data.is_null() {
        if let Ok(res) = state.http_client.get(url_recent)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await 
        {
            if res.status().as_u16() == 200 {
                if let Ok(mut json) = res.json::<serde_json::Value>().await {
                    track_data = json["items"][0]["track"].take();
                }
            }
        }
    }

    if track_data.is_null() {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }

    let cancion = track_data["name"].as_str().unwrap_or("Sin conexion").to_string();
    let artista = track_data["artists"][0]["name"].as_str().unwrap_or("Desconocido").to_string();
    let album = track_data["album"]["name"].as_str().unwrap_or("Desconocido").to_string();
    let url_cancion = track_data["external_urls"]["spotify"].as_str().unwrap_or("").to_string();
    let url_artista = track_data["artists"][0]["external_urls"]["spotify"].as_str().unwrap_or("").to_string();
    
    let image_url = track_data["album"]["images"][1]["url"].as_str().unwrap_or("");
    let (imagen_b64, bytes_imagen) = get_image_data(&state.http_client, image_url).await;

    let (paleta, color_fondo_final, color_texto_contraste) = extraer_paleta_y_fondo(&bytes_imagen);

    let estado = if is_playing { "Escuchando ahora:" } else { "Reproducido recientemente:" }.to_string();

    let alturas = vec![40, 60, 80, 100, 80, 60, 40];
    let barras = alturas.into_iter().enumerate().map(|(i, altura)| Barra {
        altura,
        delay: i as f32 * 0.1,
    }).collect();

    let template = WidgetTemplate {
        css_barras_eq: String::new(),
        color_fondo_final,
        paleta_color_1: rgb_a_hex(paleta[0]),
        paleta_color_2: rgb_a_hex(paleta[1]),
        paleta_color_3: rgb_a_hex(paleta[2]),
        color_borde: String::from("#333333"),
        color_texto_contraste,
        url_cancion,
        url_artista,
        imagen_b64,
        estado,
        cancion,
        artista,
        album,
        barras,
    };

    let renderizado = template.render().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((
        [
            (header::CONTENT_TYPE, "image/svg+xml; charset=utf-8"),
            (header::CACHE_CONTROL, "no-cache, no-store, must-revalidate"),
            (header::PRAGMA, "no-cache"),
            (header::EXPIRES, "0"),
        ],
        Html(renderizado),
    ))
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let client_id = var("SPOTIFY_CLIENT_ID").expect("Falta SPOTIFY_CLIENT_ID en el .env");
    let client_secret = var("SPOTIFY_SECRET_ID").expect("Falta SPOTIFY_SECRET_ID en el .env");
    let refresh_token = var("SPOTIFY_REFRESH_TOKEN").expect("Falta SPOTIFY_REFRESH_TOKEN en el .env");

    let state = Arc::new(AppState {
        http_client: Client::new(),
        client_id,
        client_secret,
        refresh_token,
        token_cache: RwLock::new(None),
    });

    let app = Router::new()
        .route("/", get(renderizar_widget))
        .with_state(state);

    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Servidor de acero levantado. Escuchando en el puerto 3000...");
    axum::serve(listener, app).await.unwrap();
}