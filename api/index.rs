use askama::Template;
use base64::{engine::general_purpose, Engine as _};
use color_thief::{get_palette, ColorFormat};
use reqwest::Client;
use vercel_runtime::{run, Error, Request, Response};

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

async fn get_image_data(client: &Client, url: &str) -> (String, Vec<u8>) {
    if url.is_empty() {
        return (String::new(), Vec::new());
    }
    let Ok(res) = client.get(url).send().await else { return (String::new(), Vec::new()) };
    let Ok(bytes) = res.bytes().await else { return (String::new(), Vec::new()) };
    
    let b64 = format!("data:image/jpeg;base64,{}", general_purpose::STANDARD.encode(&bytes));
    (b64, bytes.to_vec())
}

async fn handler(_req: Request) -> Result<Response<String>, Error> {
    if _req.uri().path() == "/favicon.ico" {
        return Ok(Response::builder()
            .status(404)
            .body("Not Found".to_string())?);
    }

    let client_id = std::env::var("SPOTIFY_CLIENT_ID").unwrap_or_default();
    let client_secret = std::env::var("SPOTIFY_SECRET_ID").unwrap_or_default();
    let refresh_token = std::env::var("SPOTIFY_REFRESH_TOKEN").unwrap_or_default();
    
    let client = reqwest::Client::new();

    let auth = general_purpose::STANDARD.encode(format!("{}:{}", client_id, client_secret));
    let params = [
        ("grant_type", "refresh_token"),
        ("refresh_token", &refresh_token),
    ];

    let url_token = "https://accounts.spotify.com/api/token";
    
    let res = client.post(url_token)
        .header("Authorization", format!("Basic {}", auth))
        .form(&params)
        .send()
        .await?;

    let json: serde_json::Value = res.json().await?;
    println!("DEBUG - SPOTIFY_CLIENT_ID: '{}'", client_id);
    println!("DEBUG - SPOTIFY_SECRET_ID: '{}'", client_secret);
    println!("DEBUG - SPOTIFY_REFRESH_TOKEN: '{}'", refresh_token);
    println!("DEBUG - Spotify Token Response: {}", json);

    let token = json["access_token"].as_str().unwrap_or_default();

    if token.is_empty() {
        return Ok(Response::builder()
            .status(401)
            .body("Fallo al obtener token de Spotify".to_string())?);
    }

    let mut track_data = serde_json::Value::Null;
    let mut is_playing = false;

    let url_current = "https://api.spotify.com/v1/me/player/currently-playing";
    let url_recent = "https://api.spotify.com/v1/me/player/recently-played?limit=1";

    if let Ok(res) = client.get(url_current)
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
        if let Ok(res) = client.get(url_recent)
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
        return Ok(Response::builder()
            .status(503)
            .body("No se pudo obtener datos de Spotify".to_string())?);
    }

    let cancion = track_data["name"].as_str().unwrap_or("Sin conexion").to_string();
    let artista = track_data["artists"][0]["name"].as_str().unwrap_or("Desconocido").to_string();
    let album = track_data["album"]["name"].as_str().unwrap_or("Desconocido").to_string();
    let url_cancion = track_data["external_urls"]["spotify"].as_str().unwrap_or("").to_string();
    let url_artista = track_data["artists"][0]["external_urls"]["spotify"].as_str().unwrap_or("").to_string();
    
    let image_url = track_data["album"]["images"][1]["url"].as_str().unwrap_or("");
    let (imagen_b64, bytes_imagen) = get_image_data(&client, image_url).await;

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

    let renderizado = template.render().unwrap_or_default();

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "image/svg+xml; charset=utf-8")
        .header("Cache-Control", "s-maxage=60, stale-while-revalidate=30")
        .body(renderizado)?)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    dotenvy::dotenv().ok();
    run(vercel_runtime::service_fn(handler)).await
}