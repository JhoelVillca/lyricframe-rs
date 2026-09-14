use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::renderer::render_svg;
use crate::state::PlaybackState;

#[derive(Clone)]
pub struct SvgCache {
    pub hash: u64,
    pub svg: String,
}

pub type AppCache = Arc<RwLock<SvgCache>>;

pub fn new_cache() -> AppCache {
    Arc::new(RwLock::new(SvgCache {
        hash: 0,
        svg: render_svg(true, false, "", "", "", ""),
    }))
}

pub async fn get_or_render_svg(cache: &AppCache, state: Option<PlaybackState>) -> String {
    let (hash, is_empty, is_playing, track, artist, album, art) = match state {
        Some(s) => {
            let mut hasher = DefaultHasher::new();
            s.track.hash(&mut hasher);
            s.artist.hash(&mut hasher);
            s.is_playing.hash(&mut hasher);
            s.schema_version.hash(&mut hasher);
            (hasher.finish(), false, s.is_playing, s.track, s.artist, s.album, s.art_base64)
        },
        None => {
            (1, true, false, String::new(), String::new(), String::new(), String::new())
        }
    };

    {
        let c = cache.read().await;
        if c.hash == hash {
            return c.svg.clone();
        }
    }

    // Heavy render operation
    let svg = render_svg(is_empty, is_playing, &track, &artist, &album, &art);

    {
        let mut c = cache.write().await;
        c.hash = hash;
        c.svg = svg.clone();
    }

    svg
}
