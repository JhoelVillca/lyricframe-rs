use askama::Template;
use base64::{engine::general_purpose, Engine as _};
use color_thief::get_palette;
use image::load_from_memory;

#[derive(Template)]
#[template(path = "widget.svg", escape = "xml")]
pub struct WidgetTemplate {
    pub is_empty: bool,
    pub is_playing: bool,
    pub track: String,
    pub artist: String,
    pub album: String,
    pub art_base64: String,
    pub bg_r: u8,
    pub bg_g: u8,
    pub bg_b: u8,
    pub fg_r: u8,
    pub fg_g: u8,
    pub fg_b: u8,
}

pub fn render_svg(
    is_empty: bool,
    is_playing: bool,
    track: &str,
    artist: &str,
    album: &str,
    art_base64_str: &str,
) -> String {
    let mut bg = (20, 20, 20);
    let mut fg = (230, 230, 230);
    let mut sanitized_base64 = art_base64_str.to_string();

    if !is_empty && !art_base64_str.is_empty() {
        let b64_data = if let Some(idx) = art_base64_str.find(',') {
            &art_base64_str[idx + 1..]
        } else {
            art_base64_str
        };

        if let Ok(decoded) = general_purpose::STANDARD.decode(b64_data) {
            if let Ok(img) = load_from_memory(&decoded) {
                let rgba = img.to_rgba8();
                if let Ok(palette) = get_palette(rgba.as_raw(), color_thief::ColorFormat::Rgba, 10, 2) {
                    if let Some(dominant) = palette.first() {
                        bg = (dominant.r, dominant.g, dominant.b);
                        // Contraste básico de luminancia
                        let luminance = 0.299 * dominant.r as f32 + 0.587 * dominant.g as f32 + 0.114 * dominant.b as f32;
                        if luminance > 128.0 {
                            fg = (20, 20, 20);
                        }
                    }
                }
            } else {
                sanitized_base64 = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=".to_string();
            }
        } else {
            sanitized_base64 = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=".to_string();
        }
    }

    let tpl = WidgetTemplate {
        is_empty,
        is_playing,
        track: track.to_string(),
        artist: artist.to_string(),
        album: album.to_string(),
        art_base64: sanitized_base64,
        bg_r: bg.0,
        bg_g: bg.1,
        bg_b: bg.2,
        fg_r: fg.0,
        fg_g: fg.1,
        fg_b: fg.2,
    };

    tpl.render().unwrap_or_else(|_| String::new())
}
