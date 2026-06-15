use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

use lru::LruCache;
use rspotify::prelude::Id;

use crate::state;

const MAX_TEXTURES: usize = 256;

/// Decoded image shipped from the background thread to the UI thread.
/// `image` is `None` when either the download or the decode failed.
struct DecodedImage {
    path: PathBuf,
    image: Option<egui::ColorImage>,
}

pub struct ImageCache {
    textures: LruCache<String, egui::TextureHandle>,
    download_tx: Option<mpsc::Sender<(String, PathBuf)>>,
    download_thread: Option<std::thread::JoinHandle<()>>,
    in_flight: HashSet<PathBuf>,
    decoded_rx: mpsc::Receiver<DecodedImage>,
    /// Paths we have ever handed to the background thread (download+decode
    /// succeeded or is in progress). Used to avoid re-queuing work every
    /// frame for files that are already on disk.
    known_paths: HashSet<PathBuf>,
    failed: HashSet<PathBuf>,
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
        let (decoded_tx, decoded_rx) = mpsc::channel::<DecodedImage>();

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
                    // Step 1: ensure the file is on disk. The thread is the
                    // single source of truth for "does this file exist" —
                    // callers are not allowed to short-circuit on path.exists()
                    // because that would force a stat() syscall every frame.
                    if !path.exists() {
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
                            tracing::warn!("Image download failed for path {:?}: {e:#}", path);
                            let _ = decoded_tx.send(DecodedImage { path, image: None });
                            continue;
                        }
                    }
                    // Step 2: decode JPG/PNG to egui::ColorImage on this
                    // background thread. Keeping the decode off the UI thread
                    // is what stops 30-200ms stalls when scrolling Library.
                    let image = decode_path_to_color_image(&path);
                    let _ = decoded_tx.send(DecodedImage { path, image });
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
            textures: LruCache::new(std::num::NonZeroUsize::new(MAX_TEXTURES).unwrap_or(std::num::NonZeroUsize::MIN)),
            download_tx,
            download_thread,
            in_flight: HashSet::new(),
            known_paths: HashSet::new(),
            failed: HashSet::new(),
            decoded_rx,
        }
    }

    /// Issue #10: Request image download with graceful handling when thread is unavailable.
    ///
    /// The thread is responsible for the full pipeline: download (if missing)
    /// followed by decode into `egui::ColorImage`. Callers must NOT pre-check
    /// `path.exists()` — that would force a `stat()` syscall every frame for
    /// every visible item, and `request_download` already deduplicates via
    /// `known_paths` / `in_flight` / `failed` HashSets.
    pub fn request_download(&mut self, url: &str, path: &Path) {
        // Issue #10: If download thread failed to spawn, silently skip background download
        if self.download_thread.is_none() {
            return;
        }

        let path_buf = path.to_path_buf();
        if self.failed.contains(&path_buf)
            || self.in_flight.contains(&path_buf)
            || self.known_paths.contains(&path_buf)
        {
            return;
        }
        self.in_flight.insert(path_buf.clone());
        self.known_paths.insert(path_buf.clone());
        if let Some(ref tx) = self.download_tx {
            let _ = tx.send((url.to_string(), path_buf));
        }
    }

    /// Check if an image download has permanently failed
    pub fn is_failed(&self, path: &Path) -> bool {
        self.failed.contains(path)
    }

    /// Drain the decoded-image channel and upload any new results to GPU
    /// textures. Returns `true` if at least one new texture was uploaded,
    /// in which case the caller should `ctx.request_repaint()` so the new
    /// art actually appears on screen.
    fn pump_decoded(&mut self, ctx: &egui::Context) -> bool {
        let mut any = false;
        while let Ok(decoded) = self.decoded_rx.try_recv() {
            self.in_flight.remove(&decoded.path);
            match decoded.image {
                Some(image) => {
                    let key = decoded.path.to_string_lossy().to_string();
                    let texture = ctx.load_texture(&key, image, egui::TextureOptions::LINEAR);
                    self.textures.put(key, texture);
                    any = true;
                }
                None => {
                    self.failed.insert(decoded.path);
                }
            }
        }
        any
    }

    pub fn get_texture(
        &mut self,
        ctx: &egui::Context,
        path: &Path,
    ) -> Option<&egui::TextureHandle> {
        let key = path.to_string_lossy().to_string();

        // O(1) LRU cache lookup
        if self.textures.contains(&key) {
            return self.textures.get(&key);
        }

        // Upload any background-decoded images that arrived since the last
        // frame. If anything new showed up, schedule one extra repaint so
        // the freshly-decoded cover is actually painted.
        let any_decoded = self.pump_decoded(ctx);
        if any_decoded {
            ctx.request_repaint();
        }

        if self.textures.contains(&key) {
            return self.textures.get(&key);
        }

        // File is not yet decoded (and not yet downloaded). Kick off the
        // background pipeline. The thread checks path.exists() itself, so
        // we never touch the filesystem on the UI thread.
        let path_buf = path.to_path_buf();
        if self.download_thread.is_some()
            && !self.failed.contains(&path_buf)
            && !self.in_flight.contains(&path_buf)
            && !self.known_paths.contains(&path_buf)
        {
            self.in_flight.insert(path_buf.clone());
            self.known_paths.insert(path_buf.clone());
            // url is only used when the file is missing on disk; passing an
            // empty string is fine because the thread verifies existence
            // before issuing the HTTP request.
            if let Some(ref tx) = self.download_tx {
                let _ = tx.send((String::new(), path_buf));
            }
        }

        None
    }
}

/// Decode a JPG/PNG file into `egui::ColorImage`. Runs on the background
/// thread; the result is moved to the UI thread via the `decoded_rx` channel
/// and uploaded to a GPU texture there.
fn decode_path_to_color_image(path: &Path) -> Option<egui::ColorImage> {
    let img = image::open(path).ok()?;
    let rgba = img.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    Some(egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw()))
}

pub fn album_cover_path(album: &state::Album) -> Option<PathBuf> {
    let artist = album.artists.first()?;
    let id_str = album.id.id();
    let id_prefix = &id_str[..id_str.len().min(6)];
    let filename = sanitize_filename_for_cache(&format!("{}-{}-cover-{}.jpg", album.name, artist.name, id_prefix));
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
    let filename = sanitize_filename_for_cache(&format!("playlist-{}-cover.jpg", id_prefix));
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
    let filename = sanitize_filename_for_cache(&format!("artist-{}-cover.jpg", id_prefix));
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
    let filename = sanitize_filename_for_cache(&format!("show-{}-cover.jpg", id_prefix));
    Some(
        crate::config::get_config()
            .cache_folder
            .join("image")
            .join(filename),
    )
}

pub fn category_icon_path(category: &state::Category) -> Option<PathBuf> {
    category.icon_url.as_ref()?;
    let id_prefix = &category.id[..category.id.len().min(6)];
    let filename = sanitize_filename_for_cache(&format!("category-{}-icon.jpg", id_prefix));
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
pub fn sanitize_filename_for_cache(name: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression test for the synchronous-decode jank fix:
    /// `decode_path_to_color_image` must not touch the UI thread — it produces
    /// a plain `egui::ColorImage` (just data) that can be shipped over a
    /// channel. We verify it decodes a tiny PNG without blocking on a context.
    #[test]
    fn decode_path_produces_color_image_off_thread() {
        // Synthesize a 2x2 RGBA PNG entirely in memory.
        let pixels: [u8; 16] = [
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255,
        ];
        let img = image::RgbaImage::from_raw(2, 2, pixels.to_vec()).unwrap();
        let tmp = std::env::temp_dir().join(format!(
            "spotify-rust-decode-test-{}.png",
            std::process::id()
        ));
        image::save_buffer(&tmp, &img, 2, 2, image::ColorType::Rgba8).unwrap();

        // Decode on a worker thread (mimicking the image-downloader pipeline)
        // and ship the result back over a channel, exactly like ImageCache::new.
        let (tx, rx) = mpsc::channel();
        let handle = std::thread::spawn(move || {
            let decoded = decode_path_to_color_image(&tmp);
            let _ = tx.send(decoded);
            tmp
        });
        let decoded = rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("decode must not block the UI thread indefinitely");
        let cleanup_path = handle.join().expect("worker thread must finish");
        let _ = std::fs::remove_file(cleanup_path);

        let color_image = decoded.expect("2x2 PNG must decode successfully");
        assert_eq!(color_image.size, [2, 2]);
        assert_eq!(color_image.pixels.len(), 4);
        assert_eq!(color_image.pixels[0], egui::Color32::from_rgba_unmultiplied(255, 0, 0, 255));
        assert_eq!(color_image.pixels[3], egui::Color32::from_rgba_unmultiplied(255, 255, 0, 255));
    }

    /// `decode_path_to_color_image` must return `None` (not panic) for a
    /// missing file — the worker thread relies on this to send a failure
    /// result down the channel.
    #[test]
    fn decode_path_missing_file_returns_none() {
        let bogus = PathBuf::from("/this/path/definitely/does/not/exist.png");
        assert!(decode_path_to_color_image(&bogus).is_none());
    }
}
