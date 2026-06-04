use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::mpsc;

use rspotify::prelude::Id;

use crate::state;

const MAX_TEXTURES: usize = 256;

pub struct ImageCache {
    textures: HashMap<String, egui::TextureHandle>,
    access_order: VecDeque<String>,
    download_tx: mpsc::Sender<(String, PathBuf)>,
}

impl ImageCache {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel::<(String, PathBuf)>();

        std::thread::Builder::new()
            .name("image-downloader".to_string())
            .spawn(move || {
                let rt = match tokio::runtime::Runtime::new() {
                    Ok(rt) => rt,
                    Err(_) => return,
                };
                let http = reqwest::Client::new();
                while let Ok((url, path)) = rx.recv() {
                    if path.exists() {
                        continue;
                    }
                    let _ = rt.block_on(async {
                        let resp = http
                            .get(&url)
                            .send()
                            .await
                            .map_err(|e| anyhow::anyhow!("{e}"))?;
                        let bytes = resp
                            .bytes()
                            .await
                            .map_err(|e| anyhow::anyhow!("{e}"))?;
                        if let Some(parent) = path.parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        std::fs::write(&path, &bytes)?;
                        Ok::<(), anyhow::Error>(())
                    });
                }
            })
            .ok();

        Self {
            textures: HashMap::new(),
            access_order: VecDeque::new(),
            download_tx: tx,
        }
    }

    pub fn request_download(&self, url: &str, path: &Path) {
        let _ = self
            .download_tx
            .send((url.to_string(), path.to_path_buf()));
    }

    pub fn get_texture(
        &mut self,
        ctx: &egui::Context,
        path: &Path,
    ) -> Option<&egui::TextureHandle> {
        let key = path.to_string_lossy().to_string();

        if self.textures.contains_key(&key) {
            self.touch(&key);
            return self.textures.get(&key);
        }

        if !path.exists() {
            return None;
        }

        let img = image::open(path).ok()?;
        let rgba = img.to_rgba8();
        let size = [rgba.width() as usize, rgba.height() as usize];
        let pixels = rgba.as_raw();
        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels);
        let texture = ctx.load_texture(&key, color_image, egui::TextureOptions::LINEAR);

        self.evict_if_needed();
        self.textures.insert(key.clone(), texture);
        self.access_order.push_back(key.clone());
        self.textures.get(&key)
    }

    fn touch(&mut self, key: &str) {
        if let Some(pos) = self.access_order.iter().position(|k| k == key) {
            self.access_order.remove(pos);
            self.access_order.push_back(key.to_string());
        }
    }

    fn evict_if_needed(&mut self) {
        while self.textures.len() >= MAX_TEXTURES {
            if let Some(oldest) = self.access_order.pop_front() {
                self.textures.remove(&oldest);
            } else {
                break;
            }
        }
    }
}

pub fn album_cover_path(album: &state::Album) -> Option<PathBuf> {
    let artist = album.artists.first()?;
    let id_str = album.id.id();
    let id_prefix = &id_str[..id_str.len().min(6)];
    let filename = format!("{}-{}-cover-{}.jpg", album.name, artist.name, id_prefix)
        .replace('/', "");
    Some(
        crate::config::get_config()
            .cache_folder
            .join("image")
            .join(filename),
    )
}

pub fn playlist_cover_path(playlist: &state::Playlist) -> Option<PathBuf> {
    let id_str = playlist.id.id();
    let id_prefix = &id_str[..id_str.len().min(6)];
    let filename = format!("playlist-{}-cover.jpg", id_prefix).replace('/', "");
    Some(
        crate::config::get_config()
            .cache_folder
            .join("image")
            .join(filename),
    )
}

pub fn artist_cover_path(artist: &state::Artist) -> Option<PathBuf> {
    let id_str = artist.id.id();
    let id_prefix = &id_str[..id_str.len().min(6)];
    let filename = format!("artist-{}-cover.jpg", id_prefix).replace('/', "");
    Some(
        crate::config::get_config()
            .cache_folder
            .join("image")
            .join(filename),
    )
}

pub fn show_cover_path(show: &state::Show) -> Option<PathBuf> {
    let id_str = show.id.id();
    let id_prefix = &id_str[..id_str.len().min(6)];
    let filename = format!("show-{}-cover.jpg", id_prefix).replace('/', "");
    Some(
        crate::config::get_config()
            .cache_folder
            .join("image")
            .join(filename),
    )
}

pub fn category_icon_path(category: &state::Category) -> Option<PathBuf> {
    if category.icon_url.is_none() {
        return None;
    }
    let id_prefix = &category.id[..category.id.len().min(6)];
    let filename = format!("category-{}-icon.jpg", id_prefix).replace('/', "");
    Some(
        crate::config::get_config()
            .cache_folder
            .join("image")
            .join(filename),
    )
}
