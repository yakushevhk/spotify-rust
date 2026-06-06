use std::io::{BufReader, BufWriter, Write};
use std::collections::HashMap;
use std::path::Path;

use indexmap::IndexMap;
use serde::{de::DeserializeOwned, Serialize};

use super::model::{
    Album, Artist, Category, Context, Playlist, PlaylistFolderItem,
    PlaylistFolderNode, SearchResults, Show, Track,
};
use super::Lyrics;

#[derive(Debug, Copy, Clone)]
pub enum FileCacheKey {
    Playlists,
    PlaylistFolders,
    FollowedArtists,
    SavedShows,
    SavedAlbums,
    SavedTracks,
}

/// default time-to-live cache duration
pub const TTL_CACHE_DURATION: std::time::Duration = std::time::Duration::from_secs(3600);

/// the application's data
pub struct AppData {
    pub user_data: UserData,
    pub caches: MemoryCaches,
    pub browse: BrowseData,
    pub shows_loading: bool,
}

#[derive(Debug)]
/// current user's data
pub struct UserData {
    pub user: Option<rspotify::model::PrivateUser>,
    pub playlists: Vec<PlaylistFolderItem>,
    pub playlist_folder_node: Option<PlaylistFolderNode>,
    pub followed_artists: Vec<Artist>,
    pub saved_shows: Vec<Show>,
    pub saved_albums: Vec<Album>,
    pub saved_tracks: HashMap<String, Track>,
}

/// the application's in-memory caches
pub struct MemoryCaches {
    pub context: ttl_cache::TtlCache<String, Context>,
    pub search: ttl_cache::TtlCache<String, SearchResults>,
    pub lyrics: ttl_cache::TtlCache<String, Option<Lyrics>>,
    pub genres: ttl_cache::TtlCache<String, Vec<String>>,
    #[cfg(feature = "image")]
    pub images: ttl_cache::TtlCache<String, image::DynamicImage>,
}

#[derive(Default, Debug)]
/// Spotify browse data
pub struct BrowseData {
    pub categories: Vec<Category>,
    pub category_playlists: IndexMap<String, Vec<Playlist>>,
    pub categories_loading: bool,
    pub category_playlists_loading: Option<String>,
}

/// Maximum number of category playlists to cache before evicting oldest entries.
const MAX_CATEGORY_PLAYLISTS: usize = 64;

impl BrowseData {
    /// Insert a category playlist and evict the oldest entry if over limit (M4).
    pub fn insert_category_playlists(&mut self, category_id: String, playlists: Vec<Playlist>) {
        self.category_playlists.insert(category_id, playlists);
        if self.category_playlists.len() > MAX_CATEGORY_PLAYLISTS {
            self.category_playlists.shift_remove_index(0);
        }
    }
}

impl MemoryCaches {
    pub fn new() -> Self {
        Self {
            context: ttl_cache::TtlCache::new(64),
            search: ttl_cache::TtlCache::new(64),
            lyrics: ttl_cache::TtlCache::new(64),
            genres: ttl_cache::TtlCache::new(64),
            #[cfg(feature = "image")]
            images: ttl_cache::TtlCache::new(64),
        }
    }
}

impl AppData {
    pub fn new(cache_folder: &Path) -> Self {
        Self {
            user_data: UserData::new_from_file_caches(cache_folder),
            caches: MemoryCaches::new(),
            browse: BrowseData::default(),
            shows_loading: false,
        }
    }
}

impl UserData {
    /// Construct a new user data based on file caches
    pub fn new_from_file_caches(cache_folder: &Path) -> Self {
        Self {
            user: None,
            playlists: load_data_from_file_cache(FileCacheKey::Playlists, cache_folder)
                .unwrap_or_else(|| {
                    tracing::warn!("Playlist cache not available or corrupted, starting empty");
                    Vec::new()
                }),
            playlist_folder_node: load_data_from_file_cache(
                FileCacheKey::PlaylistFolders,
                cache_folder,
            ),
            followed_artists: load_data_from_file_cache(
                FileCacheKey::FollowedArtists,
                cache_folder,
            )
            .unwrap_or_else(|| {
                tracing::warn!("Followed artists cache not available or corrupted, starting empty");
                Vec::new()
            }),
            saved_shows: load_data_from_file_cache(FileCacheKey::SavedShows, cache_folder)
                .unwrap_or_else(|| {
                    tracing::warn!("Saved shows cache not available or corrupted, starting empty");
                    Vec::new()
                }),
            saved_albums: load_data_from_file_cache(FileCacheKey::SavedAlbums, cache_folder)
                .unwrap_or_else(|| {
                    tracing::warn!("Saved albums cache not available or corrupted, starting empty");
                    Vec::new()
                }),
            saved_tracks: load_data_from_file_cache(FileCacheKey::SavedTracks, cache_folder)
                .unwrap_or_else(|| {
                    tracing::warn!("Saved tracks cache not available or corrupted, starting empty");
                    std::collections::HashMap::new()
                }),
        }
    }

}

/// Issue #2: Store data to file cache with disk full error handling
pub fn store_data_into_file_cache<T: Serialize>(
    key: FileCacheKey,
    cache_folder: &Path,
    data: &T,
) -> std::io::Result<()> {
    let path = cache_folder.join(format!("{key:?}_cache.json"));
    let result = (|| -> std::io::Result<()> {
        let temp_path = path.with_extension("tmp");
        let mut f = BufWriter::new(std::fs::File::create(&temp_path)?);
        serde_json::to_writer(&mut f, data)?;
        f.flush()?;
        if let Err(e) = std::fs::rename(&temp_path, &path) {
            #[cfg(unix)]
            let is_cross_device = e.raw_os_error() == Some(18); // EXDEV
            #[cfg(not(unix))]
            let is_cross_device = false;
            if is_cross_device {
                std::fs::copy(&temp_path, &path)?;
                std::fs::remove_file(&temp_path)?;
            } else {
                let _ = std::fs::remove_file(&temp_path);
                return Err(e);
            }
        }
        Ok(())
    })();
    
    // Clean up orphaned .tmp files on error
    if result.is_err() {
        let _ = std::fs::remove_file(path.with_extension("tmp"));
    }
    
    // Issue #2: Handle write failures generically with user-friendly message
    if let Err(ref e) = result {
        #[cfg(unix)]
        let is_disk_full = e.kind() == std::io::ErrorKind::StorageFull
            || e.raw_os_error() == Some(28) // ENOSPC
            || e.raw_os_error() == Some(69); // EDQUOT
        #[cfg(not(unix))]
        let is_disk_full = e.kind() == std::io::ErrorKind::StorageFull;

        if is_disk_full {
            tracing::error!("Disk full when writing to {}: {e}", path.display());
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Write failed - check disk space (path: {})", path.display())
            ));
        }
    }
    
    result
}

pub fn load_data_from_file_cache<T>(key: FileCacheKey, cache_folder: &Path) -> Option<T>
where
    T: DeserializeOwned,
{
    let path = cache_folder.join(format!("{key:?}_cache.json"));
    match std::fs::File::open(&path) {
        Ok(file) => {
            tracing::info!("Loading {key:?} data from {}...", path.display());
            let f = BufReader::new(file);
            match serde_json::from_reader(f) {
                Ok(data) => {
                    tracing::info!("Successfully loaded {key:?} data!");
                    Some(data)
                }
                Err(err) => {
                    tracing::error!("Failed to load {key:?} data: {err:#}");
                    None
                }
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            None
        }
        Err(e) => {
            tracing::warn!("Failed to open cache file {}: {e:#}", path.display());
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::model::{TrackId, PlaylistId, UserId, AlbumId, ShowId, ContextId, Album, Show, Context};
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn create_test_track(id: &str) -> Track {
        Track {
            id: TrackId::from_id(id).unwrap().into_static(),
            name: format!("Track {}", id),
            artists: vec![],
            album: None,
            duration: std::time::Duration::from_secs(180),
            explicit: false,
            added_at: 0,
            artists_display: None,
        }
    }

    fn create_test_playlist(id: &str) -> Playlist {
        Playlist {
            id: PlaylistId::from_id(id).unwrap().into_static(),
            collaborative: false,
            name: format!("Playlist {}", id),
            owner: ("Owner".to_string(), UserId::from_id("spotify").unwrap().into_static()),
            desc: "Description".to_string(),
            current_folder_id: 0,
            snapshot_id: "snapshot".to_string(),
            cover_url: None,
            image_path: None,
        }
    }

    /// Test FileCacheKey debug formatting
    #[test]
    fn test_file_cache_key_debug() {
        assert!(format!("{:?}", FileCacheKey::Playlists).contains("Playlists"));
        assert!(format!("{:?}", FileCacheKey::SavedTracks).contains("SavedTracks"));
    }

    /// Test TTL_CACHE_DURATION value
    #[test]
    fn test_ttl_cache_duration() {
        assert_eq!(TTL_CACHE_DURATION, std::time::Duration::from_secs(3600));
    }

    /// Test MemoryCaches initialization
    #[test]
    fn test_memory_caches_new() {
        let caches = MemoryCaches::new();
        
        // Caches should be created successfully
        // Note: TtlCache doesn't have is_empty(), but we can verify it was created
        let _ = &caches.context;
        let _ = &caches.search;
        let _ = &caches.lyrics;
        let _ = &caches.genres;
    }

    /// Test BrowseData default values
    #[test]
    fn test_browse_data_default() {
        let browse = BrowseData::default();
        assert!(browse.categories.is_empty());
        assert!(browse.category_playlists.is_empty());
        assert!(!browse.categories_loading);
    }

    /// Test BrowseData insert_category_playlists
    #[test]
    fn test_browse_data_insert_category_playlists() {
        let mut browse = BrowseData::default();
        let playlists = vec![create_test_playlist("test")];
        
        browse.insert_category_playlists("category1".to_string(), playlists);
        assert_eq!(browse.category_playlists.len(), 1);
        assert!(browse.category_playlists.contains_key("category1"));
    }

    /// Test BrowseData insert_category_playlists eviction
    #[test]
    fn test_browse_data_insert_category_playlists_eviction() {
        let mut browse = BrowseData::default();
        
        // Insert more than MAX_CATEGORY_PLAYLISTS
        for i in 0..MAX_CATEGORY_PLAYLISTS + 5 {
            let playlists = vec![create_test_playlist(&format!("playlist{}", i))];
            browse.insert_category_playlists(format!("category{}", i), playlists);
        }
        
        // Should have at most MAX_CATEGORY_PLAYLISTS
        assert!(browse.category_playlists.len() <= MAX_CATEGORY_PLAYLISTS);
    }

    /// Test AppData initialization
    #[test]
    fn test_app_data_new() {
        let temp_dir = TempDir::new().unwrap();
        let app_data = AppData::new(temp_dir.path());
        
        assert!(!app_data.shows_loading);
    }

    /// Test store_data_into_file_cache and load_data_from_file_cache
    #[test]
    fn test_file_cache_store_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let data: Vec<Playlist> = vec![create_test_playlist("test")];
        
        // Store data
        let result = store_data_into_file_cache(FileCacheKey::Playlists, temp_dir.path(), &data);
        assert!(result.is_ok());
        
        // Load data back
        let loaded: Option<Vec<Playlist>> = load_data_from_file_cache(
            FileCacheKey::Playlists,
            temp_dir.path(),
        );
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().len(), 1);
    }

    /// Test load_data_from_file_cache with missing file
    #[test]
    fn test_file_cache_load_missing() {
        let temp_dir = TempDir::new().unwrap();
        
        let loaded: Option<Vec<Playlist>> = load_data_from_file_cache(
            FileCacheKey::Playlists,
            temp_dir.path(),
        );
        assert!(loaded.is_none());
    }

    /// Test load_data_from_file_cache with corrupted file
    #[test]
    fn test_file_cache_load_corrupted() {
        let temp_dir = TempDir::new().unwrap();
        let cache_file = temp_dir.path().join("Playlists_cache.json");
        
        // Write invalid JSON
        std::fs::write(&cache_file, "not valid json").unwrap();
        
        let loaded: Option<Vec<Playlist>> = load_data_from_file_cache(
            FileCacheKey::Playlists,
            temp_dir.path(),
        );
        assert!(loaded.is_none());
    }

    /// Test SearchResults default
    #[test]
    fn test_search_results_default() {
        let results = SearchResults::default();
        assert!(results.tracks.is_empty());
        assert!(results.artists.is_empty());
        assert!(results.albums.is_empty());
        assert!(results.playlists.is_empty());
        assert!(results.shows.is_empty());
        assert!(results.episodes.is_empty());
    }

    /// Test UserData default
    #[test]
    fn test_user_data_default() {
        let user_data = UserData {
            user: None,
            playlists: vec![],
            playlist_folder_node: None,
            followed_artists: vec![],
            saved_shows: vec![],
            saved_albums: vec![],
            saved_tracks: HashMap::new(),
        };
        
        assert!(user_data.user.is_none());
        assert!(user_data.playlists.is_empty());
        assert!(user_data.followed_artists.is_empty());
    }
}
