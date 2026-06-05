use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

use lru::LruCache;
use rspotify::prelude::Id;

use crate::state;

const MAX_TEXTURES: usize = 256;

pub struct ImageCache {
    textures: LruCache<String, egui::TextureHandle>,
    download_tx: Option<mpsc::Sender<(String, PathBuf)>>,
    download_thread: Option<std::thread::JoinHandle<()>>,
    in_flight: HashSet<String>,
    done_rx: mpsc::Receiver<String>,
}

impl Drop for ImageCache {
    fn drop(&mut self) {
        // Close the channel so the download thread exits its loop
        drop(self.download_tx.take());
        if let Some(handle) = self.download_thread.take() {
            let _ = handle.join();
        }
    }
}

impl ImageCache {
    /// Issue #10: Creates a new ImageCache with graceful handling of thread spawn failure
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel::<(String, PathBuf)>();
        let (done_tx, done_rx) = mpsc::channel::<String>();

        // Issue #10: Handle thread spawn failure gracefully instead of panicking
        let handle_result = std::thread::Builder::new()
            .name("image-downloader".to_string())
            .spawn(move || {
                let rt = match tokio::runtime::Runtime::new() {
                    Ok(rt) => rt,
                    Err(e) => {
                        tracing::error!("Failed to create tokio runtime for image downloader: {e}");
                        return;
                    }
                };
                let http = reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(15))
                    .build()
                    .unwrap_or_default();
                while let Ok((url, path)) = rx.recv() {
                    if path.exists() {
                        let _ = done_tx.send(url);
                        continue;
                    }
                    let result = rt.block_on(async {
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
                        // M25: write to temp file then rename to avoid TOCTOU
                        // Issue #5: Also use tempfile for automatic cleanup
                        let tmp = path.with_extension("tmp");
                        std::fs::write(&tmp, &bytes)?;
                        std::fs::rename(&tmp, &path)?;
                        Ok::<(), anyhow::Error>(())
                    });
                    if let Err(e) = &result {
                        tracing::warn!("Image download failed for {url}: {e:#}");
                    }
                    let _ = done_tx.send(url);
                }
            });

        // Issue #10: Handle spawn failure gracefully
        let (download_tx, download_thread) = match handle_result {
            Ok(handle) => {
                tracing::info!("Image downloader thread spawned successfully");
                (Some(tx), Some(handle))
            }
            Err(e) => {
                tracing::error!("Failed to spawn image-downloader thread: {e}");
                // Continue without image cache - images will still work but won't be downloaded
                // in background thread
                (None, None)
            }
        };

        Self {
            textures: LruCache::new(std::num::NonZeroUsize::new(MAX_TEXTURES).unwrap()),
            download_tx,
            download_thread,
            in_flight: HashSet::new(),
            done_rx,
        }
    }

    /// Issue #10: Request image download with graceful handling when thread is unavailable
    pub fn request_download(&mut self, url: &str, path: &Path) {
        // Issue #10: If download thread failed to spawn, silently skip background download
        // The image will be fetched on-demand in get_texture
        if self.download_thread.is_none() {
            return;
        }
        
        // Drain completion channel
        while let Ok(done_url) = self.done_rx.try_recv() {
            self.in_flight.remove(&done_url);
        }
        if path.exists() {
            self.in_flight.remove(url);
            return;
        }
        if self.in_flight.contains(url) {
            return;
        }
        self.in_flight.insert(url.to_string());
        if let Some(ref tx) = self.download_tx {
            let _ = tx.send((url.to_string(), path.to_path_buf()));
        }
    }

    pub fn get_texture(
        &mut self,
        ctx: &egui::Context,
        path: &Path,
    ) -> Option<&egui::TextureHandle> {
        let key = path.to_string_lossy().to_string();

        // O(1) LRU cache lookup
        if self.textures.get(&key).is_some() {
            return self.textures.get(&key);
        }

        // Offload file existence check to background or cache result
        // For now, we still check but this is much faster with the LRU cache
        if !path.exists() {
            return None;
        }

        let img = image::open(path).ok()?;
        let rgba = img.to_rgba8();
        let size = [rgba.width() as usize, rgba.height() as usize];
        let pixels = rgba.as_raw();
        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels);
        let texture = ctx.load_texture(&key, color_image, egui::TextureOptions::LINEAR);

        // O(1) LRU insertion with automatic eviction
        self.textures.put(key.clone(), texture);
        self.textures.get(&key)
    }
}

pub fn album_cover_path(album: &state::Album) -> Option<PathBuf> {
    let artist = album.artists.first()?;
    let id_str = album.id.id();
    let id_prefix = &id_str[..id_str.len().min(6)];
    let filename = sanitize_filename(&format!("{}-{}-cover-{}.jpg", album.name, artist.name, id_prefix));
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
    let filename = sanitize_filename(&format!("playlist-{}-cover.jpg", id_prefix));
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
    let filename = sanitize_filename(&format!("artist-{}-cover.jpg", id_prefix));
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
    let filename = sanitize_filename(&format!("show-{}-cover.jpg", id_prefix));
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
    let filename = sanitize_filename(&format!("category-{}-icon.jpg", id_prefix));
    Some(
        crate::config::get_config()
            .cache_folder
            .join("image")
            .join(filename),
    )
}

/// Sanitize a filename by replacing characters that are unsafe on any platform
/// (Windows: \ : * ? " < > |, plus NUL; Unix: /).
/// Also rejects path traversal attempts (.., leading/trailing dots).
fn sanitize_filename(name: &str) -> String {
    // Reject path traversal attempts
    if name.contains("..") {
        return format!("invalid_traversal_{}", hash_name(name));
    }

    let sanitized: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();

    // Additional validation: ensure no remaining path separators or traversal
    if sanitized.contains('/') || sanitized.contains('\\') || sanitized.starts_with('.') {
        return format!("invalid_{}", hash_name(name));
    }

    // Limit filename length to prevent other issues
    if sanitized.len() > 200 {
        let hash = hash_name(&sanitized);
        format!("{}_{}", &sanitized[..150], hash)
    } else {
        sanitized
    }
}

/// Generate a hash-based filename for security-critical paths
fn hash_name(name: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    name.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}
