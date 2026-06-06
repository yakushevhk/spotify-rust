use std::io::{BufReader, BufWriter, Write};
use std::collections::HashMap;
use std::path::Path;

use indexmap::IndexMap;
use serde::{de::DeserializeOwned, Serialize};
use std::sync::LazyLock;

use super::model::{
    Album, Artist, Category, Context, ContextId, Id, Playlist, PlaylistFolderItem,
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
pub static TTL_CACHE_DURATION: LazyLock<std::time::Duration> =
    LazyLock::new(|| std::time::Duration::from_hours(1));

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

    /// Get a list of tracks inside a given context
    #[allow(dead_code)]
    pub fn context_tracks_mut(&mut self, id: &ContextId) -> Option<&mut Vec<Track>> {
        let c = self.caches.context.get_mut(&id.uri())?;

        Some(match c {
            Context::Album { tracks, .. }
            | Context::Playlist { tracks, .. }
            | Context::Tracks { tracks, .. }
            | Context::Artist {
                top_tracks: tracks, ..
            } => tracks,
            Context::Show { .. } => {
                return None;
            }
        })
    }

    #[allow(dead_code)]
    pub fn context_tracks(&self, id: &ContextId) -> Option<&Vec<Track>> {
        let c = self.caches.context.get(&id.uri())?;
        Some(match c {
            Context::Album { tracks, .. }
            | Context::Playlist { tracks, .. }
            | Context::Tracks { tracks, .. }
            | Context::Artist {
                top_tracks: tracks, ..
            } => tracks,
            Context::Show { .. } => {
                return None;
            }
        })
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

    /// Get a list of playlist items that are **possibly** modifiable by user
    ///
    /// If `folder_id` is provided, returns items in the given folder id.
    /// Otherwise, returns the all items.
    #[allow(dead_code)]
    pub fn modifiable_playlist_items(&self, folder_id: Option<usize>) -> Vec<&PlaylistFolderItem> {
        match self.user {
            None => vec![],
            Some(ref u) => self
                .playlists
                .iter()
                // filter items in a folder (if specified)
                .filter(|item| {
                    if let Some(folder_id) = folder_id {
                        match item {
                            PlaylistFolderItem::Playlist(p) => p.current_folder_id == folder_id,
                            PlaylistFolderItem::Folder(f) => f.current_id == folder_id,
                        }
                    } else {
                        true
                    }
                })
                // filter modifiable items
                .filter(|item| match item {
                    PlaylistFolderItem::Playlist(p) => p.owner.1 == u.id || p.collaborative,
                    PlaylistFolderItem::Folder(_) => true,
                })
                .collect(),
        }
    }

    /// Get playlists items for the given folder id
    #[allow(dead_code)]
    pub fn folder_playlists_items(&self, folder_id: usize) -> Vec<&PlaylistFolderItem> {
        self.playlists
            .iter()
            .filter(|item| match item {
                PlaylistFolderItem::Playlist(p) => p.current_folder_id == folder_id,
                PlaylistFolderItem::Folder(f) => f.current_id == folder_id,
            })
            .collect()
    }

    /// Check if a track is a liked track
    #[allow(dead_code)]
    pub fn is_liked_track(&self, track: &Track) -> bool {
        self.saved_tracks.contains_key(&track.id.uri())
    }

    /// Check if a playlist is followed
    #[allow(dead_code)]
    pub fn is_followed_playlist(&self, playlist: &Playlist) -> bool {
        self.playlists.iter().any(|x| match x {
            PlaylistFolderItem::Playlist(p) => p.id == playlist.id,
            PlaylistFolderItem::Folder(_) => false,
        })
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
    if path.exists() {
        tracing::info!("Loading {key:?} data from {}...", path.display());
        let f = match std::fs::File::open(&path) {
            Ok(f) => BufReader::new(f),
            Err(err) => {
                tracing::error!("Failed to open {key:?} cache file: {err:#}");
                return None;
            }
        };
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
    } else {
        None
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
            name_lower: None,
            artists_display: None,
            artists_info_lower: None,
            album_info_lower: None,
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
            name_lower: None,
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
        assert_eq!(*TTL_CACHE_DURATION, std::time::Duration::from_hours(1));
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

    /// Test UserData is_liked_track
    #[test]
    fn test_user_data_is_liked_track() {
        let mut user_data = UserData {
            user: None,
            playlists: vec![],
            playlist_folder_node: None,
            followed_artists: vec![],
            saved_shows: vec![],
            saved_albums: vec![],
            saved_tracks: HashMap::new(),
        };
        
        let track = create_test_track("3n3Ppam7vgaVa1iaRUc9Lp");
        user_data.saved_tracks.insert(track.id.uri(), track.clone());
        
        let test_track = create_test_track("3n3Ppam7vgaVa1iaRUc9Lp");
        let other_track = create_test_track("4uLU6hMCjMI75M1A2tKUQC");
        
        assert!(user_data.is_liked_track(&test_track));
        assert!(!user_data.is_liked_track(&other_track));
    }

    /// Test UserData is_followed_playlist
    #[test]
    fn test_user_data_is_followed_playlist() {
        let playlist = create_test_playlist("37i9dQZF1DXcBWIGoYBM5M");
        let user_data = UserData {
            user: None,
            playlists: vec![PlaylistFolderItem::Playlist(playlist.clone())],
            playlist_folder_node: None,
            followed_artists: vec![],
            saved_shows: vec![],
            saved_albums: vec![],
            saved_tracks: HashMap::new(),
        };
        
        let followed_playlist = create_test_playlist("37i9dQZF1DXcBWIGoYBM5M");
        let other_playlist = create_test_playlist("other");
        
        assert!(user_data.is_followed_playlist(&followed_playlist));
        assert!(!user_data.is_followed_playlist(&other_playlist));
    }

    /// Test UserData modifiable_playlist_items with no user
    #[test]
    fn test_user_data_modifiable_playlist_items_no_user() {
        let user_data = UserData {
            user: None,
            playlists: vec![PlaylistFolderItem::Playlist(create_test_playlist("test"))],
            playlist_folder_node: None,
            followed_artists: vec![],
            saved_shows: vec![],
            saved_albums: vec![],
            saved_tracks: HashMap::new(),
        };
        
        let items = user_data.modifiable_playlist_items(None);
        assert!(items.is_empty());
    }

    /// Test UserData modifiable_playlist_items with user
    #[test]
    fn test_user_data_modifiable_playlist_items_with_user() {
        // Create a minimal PrivateUser using serde_json::from_value
        // to avoid dealing with exact field names
        let user_json = serde_json::json!({
            "id": "owner_id",
            "display_name": "Owner",
            "external_urls": {},
            "href": "",
            "images": [],
            "followers": {"total": 0}
        });
        
        let user: rspotify::model::PrivateUser = serde_json::from_value(user_json).unwrap();
        
        let playlist = Playlist {
            id: PlaylistId::from_id("3n3Ppam7vgaVa1iaRUc9Lp").unwrap().into_static(),
            collaborative: false,
            name: "Test".to_string(),
            owner: ("Owner".to_string(), UserId::from_id("owner_id").unwrap().into_static()),
            desc: "Description".to_string(),
            current_folder_id: 0,
            snapshot_id: "snapshot".to_string(),
            cover_url: None,
            name_lower: None,
            image_path: None,
        };
        
        let user_data = UserData {
            user: Some(user),
            playlists: vec![PlaylistFolderItem::Playlist(playlist)],
            playlist_folder_node: None,
            followed_artists: vec![],
            saved_shows: vec![],
            saved_albums: vec![],
            saved_tracks: HashMap::new(),
        };
        
        let items = user_data.modifiable_playlist_items(None);
        assert_eq!(items.len(), 1);
    }

    /// Test UserData folder_playlists_items
    #[test]
    fn test_user_data_folder_playlists_items() {
        let playlist1 = Playlist {
            id: PlaylistId::from_id("p1").unwrap().into_static(),
            collaborative: false,
            name: "Playlist 1".to_string(),
            owner: ("Owner".to_string(), UserId::from_id("spotify").unwrap().into_static()),
            desc: "Description".to_string(),
            current_folder_id: 1,
            snapshot_id: "snapshot".to_string(),
            cover_url: None,
            name_lower: None,
            image_path: None,
        };
        
        let playlist2 = Playlist {
            id: PlaylistId::from_id("p2").unwrap().into_static(),
            collaborative: false,
            name: "Playlist 2".to_string(),
            owner: ("Owner".to_string(), UserId::from_id("spotify").unwrap().into_static()),
            desc: "Description".to_string(),
            current_folder_id: 2,
            snapshot_id: "snapshot".to_string(),
            cover_url: None,
            name_lower: None,
            image_path: None,
        };
        
        let user_data = UserData {
            user: None,
            playlists: vec![
                PlaylistFolderItem::Playlist(playlist1),
                PlaylistFolderItem::Playlist(playlist2),
            ],
            playlist_folder_node: None,
            followed_artists: vec![],
            saved_shows: vec![],
            saved_albums: vec![],
            saved_tracks: HashMap::new(),
        };
        
        let folder1_items = user_data.folder_playlists_items(1);
        assert_eq!(folder1_items.len(), 1);
        
        let folder2_items = user_data.folder_playlists_items(2);
        assert_eq!(folder2_items.len(), 1);
        
        let folder3_items = user_data.folder_playlists_items(3);
        assert!(folder3_items.is_empty());
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

    /// Test context_tracks with Album context
    #[test]
    fn test_context_tracks_album() {
        let temp_dir = TempDir::new().unwrap();
        let mut app_data = AppData::new(temp_dir.path());
        
        let album_id = AlbumId::from_id("4uLU6hMCjMI75M1A2tKUQC").unwrap().into_static();
        let context_id = ContextId::Album(album_id);
        
        // Insert context with tracks
        let tracks = vec![create_test_track("3n3Ppam7vgaVa1iaRUc9Lp"), create_test_track("4uLU6hMCjMI75M1A2tKUQC")];
        let context = Context::Album {
            album: Album {
                id: AlbumId::from_id("4uLU6hMCjMI75M1A2tKUQC").unwrap().into_static(),
                release_date: "2024".to_string(),
                name: "Test Album".to_string(),
                artists: vec![],
                typ: None,
                added_at: 0,
                cover_url: None,
                name_lower: None,
                artists_display: None,
                image_path: None,
            },
            tracks,
        };
        
        app_data.caches.context.insert(
            context_id.uri(),
            context,
            *TTL_CACHE_DURATION,
        );
        
        let context_tracks = app_data.context_tracks(&context_id);
        assert!(context_tracks.is_some());
        assert_eq!(context_tracks.unwrap().len(), 2);
    }

    /// Test context_tracks with Show context (returns None)
    #[test]
    fn test_context_tracks_show_returns_none() {
        let temp_dir = TempDir::new().unwrap();
        let mut app_data = AppData::new(temp_dir.path());
        
        let show_id = ShowId::from_id("0Xr5K8y0oZbLTHW1zP20mQ").unwrap().into_static();
        let context_id = ContextId::Show(show_id);
        
        // Insert Show context
        let context = Context::Show {
            show: Show {
                id: ShowId::from_id("0Xr5K8y0oZbLTHW1zP20mQ").unwrap().into_static(),
                name: "Test Show".to_string(),
                publisher: "Publisher".to_string(),
                description: "Description".to_string(),
                cover_url: None,
            },
            episodes: vec![],
        };
        
        app_data.caches.context.insert(
            context_id.uri(),
            context,
            *TTL_CACHE_DURATION,
        );
        
        let context_tracks = app_data.context_tracks(&context_id);
        assert!(context_tracks.is_none()); // Show context has no tracks
    }

    /// Test context_tracks with missing context
    #[test]
    fn test_context_tracks_missing() {
        let temp_dir = TempDir::new().unwrap();
        let app_data = AppData::new(temp_dir.path());
        
        let album_id = AlbumId::from_id("4uLU6hMCjMI75M1A2tKUQC").unwrap().into_static();
        let context_id = ContextId::Album(album_id);
        
        let context_tracks = app_data.context_tracks(&context_id);
        assert!(context_tracks.is_none());
    }

    /// Test context_tracks_mut
    #[test]
    fn test_context_tracks_mut() {
        let temp_dir = TempDir::new().unwrap();
        let mut app_data = AppData::new(temp_dir.path());
        
        let album_id = AlbumId::from_id("4uLU6hMCjMI75M1A2tKUQC").unwrap().into_static();
        let context_id = ContextId::Album(album_id);
        
        // Insert context with tracks
        let tracks = vec![create_test_track("3n3Ppam7vgaVa1iaRUc9Lp")];
        let context = Context::Album {
            album: Album {
                id: AlbumId::from_id("4uLU6hMCjMI75M1A2tKUQC").unwrap().into_static(),
                release_date: "2024".to_string(),
                name: "Test Album".to_string(),
                artists: vec![],
                typ: None,
                added_at: 0,
                cover_url: None,
                name_lower: None,
                artists_display: None,
                image_path: None,
            },
            tracks,
        };
        
        app_data.caches.context.insert(
            context_id.uri(),
            context,
            *TTL_CACHE_DURATION,
        );
        
        let context_tracks = app_data.context_tracks_mut(&context_id);
        assert!(context_tracks.is_some());
        
        // Modify tracks
        let tracks = context_tracks.unwrap();
        tracks.push(create_test_track("4uLU6hMCjMI75M1A2tKUQC"));
        assert_eq!(tracks.len(), 2);
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
