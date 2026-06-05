pub mod keymap;
pub mod theme;

const DEFAULT_CONFIG_FOLDER: &str = ".config/spotify-player";
const DEFAULT_CACHE_FOLDER: &str = ".cache/spotify-player";
const APP_CONFIG_FILE: &str = "app.toml";
const THEME_CONFIG_FILE: &str = "theme.toml";
const KEYMAP_CONFIG_FILE: &str = "keymap.toml";

use anyhow::{anyhow, Result};
use config_parser2::{config_parser_impl, ConfigParse, ConfigParser};
use librespot_core::config::SessionConfig;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};

use keymap::KeymapConfig;
use theme::ThemeConfig;

pub use theme::Theme;

use crate::auth::{NCSPOT_CLIENT_ID, SPOTIFY_CLIENT_ID};

static CONFIGS: OnceLock<parking_lot::Mutex<Configs>> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct Configs {
    pub app_config: AppConfig,
    pub keymap_config: KeymapConfig,
    pub theme_config: ThemeConfig,
    pub cache_folder: std::path::PathBuf,
}

impl Configs {
    pub fn new(config_folder: &std::path::Path, cache_folder: &std::path::Path) -> Result<Self> {
        Ok(Self {
            app_config: AppConfig::new(config_folder)?,
            keymap_config: KeymapConfig::new(config_folder)?,
            theme_config: ThemeConfig::new(config_folder)?,
            cache_folder: cache_folder.to_path_buf(),
        })
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, ConfigParse)]
#[allow(clippy::struct_excessive_bools)]
/// Application configurations loaded from `~/.config/spotify-player/app.toml`.
pub struct AppConfig {
    /// Name of the color theme (built-in or custom).
    pub theme: String,
    /// Spotify application client ID used for OAuth PKCE flow.
    /// Falls back to ncspot's client ID when not set.
    pub client_id: Option<String>,
    /// External command whose stdout is used as the client ID.
    pub client_id_command: Option<Command>,

    /// Local HTTP server port for the OAuth redirect callback.
    // Unused in GUI
    #[allow(dead_code)]
    pub client_port: u16,

    /// OAuth redirect URI registered with Spotify.
    pub login_redirect_uri: String,

    /// Directory for log files. Defaults to `~/.cache/spotify-player`.
    pub log_folder: Option<PathBuf>,

    /// External command run on each player event (play, pause, skip, etc.).
    pub player_event_hook_command: Option<Command>,

    /// Format string for the playback status line in the TUI (unused in GUI).
    pub playback_format: String,
    // Unused in GUI
    #[allow(dead_code)]
    pub playback_metadata_fields: Vec<String>,
    #[cfg(feature = "notify")]
    pub notify_format: NotifyFormat,
    #[cfg(feature = "notify")]
    pub notify_timeout_in_secs: u64,
    #[cfg(feature = "notify")]
    #[cfg(all(unix, not(target_os = "macos")))]
    pub notify_transient: bool,

    /// Maximum number of tracks fetched in a single API call.
    pub tracks_playback_limit: usize,

    /// HTTP proxy URL for Spotify API requests.
    pub proxy: Option<String>,
    /// librespot access point port override.
    pub ap_port: Option<u16>,

    /// How often the GUI redraws (in milliseconds).
    pub app_refresh_duration_in_ms: u64,
    /// How often to poll the Spotify API for playback state updates.
    pub playback_refresh_duration_in_ms: u64,

    /// Number of rows per page in track tables.
    pub page_size_in_rows: usize,

    // icon configs
    // Unused in GUI
    #[allow(dead_code)]
    pub play_icon: String,
    // Unused in GUI
    #[allow(dead_code)]
    pub pause_icon: String,
    // Unused in GUI
    #[allow(dead_code)]
    pub liked_icon: String,
    pub explicit_icon: String,

    // layout configs
    // Unused in GUI
    #[allow(dead_code)]
    pub border_type: BorderType,
    // Unused in GUI
    #[allow(dead_code)]
    pub progress_bar_type: ProgressBarType,
    // Unused in GUI
    #[allow(dead_code)]
    pub progress_bar_position: ProgressBarPosition,

    pub layout: LayoutConfig,

    // Unused in GUI
    #[allow(dead_code)]
    pub genre_num: u8,

    #[cfg(feature = "image")]
    pub cover_img_length: usize,
    #[cfg(feature = "image")]
    pub cover_img_width: usize,
    #[cfg(feature = "image")]
    pub cover_img_scale: f32,
    #[cfg(feature = "pixelate")]
    pub cover_img_pixels: u32,

    #[cfg(feature = "media-control")]
    pub enable_media_control: bool,

    /// Streaming backend: Always, DaemonOnly, or Never.
    pub enable_streaming: StreamingType,

    /// Show an FFT audio visualization when streaming locally.
    #[cfg(feature = "streaming")]
    pub enable_audio_visualization: bool,

    /// Enable desktop notifications on track change.
    #[cfg(feature = "notify")]
    pub enable_notify: bool,

    /// Cache downloaded cover images to the disk cache.
    pub enable_cover_image_cache: bool,

    /// Name of the default Spotify Connect device.
    pub default_device: String,

    /// Integrated librespot device configuration.
    pub device: DeviceConfig,

    /// Only send notifications when streaming locally (not via Spotify Connect).
    #[cfg(all(feature = "streaming", feature = "notify"))]
    pub notify_streaming_only: bool,

    /// Seconds to seek forward/backward on seek commands.
    pub seek_duration_secs: u16,

    /// Sort artist albums by type (album, single, compilation).
    pub sort_artist_albums_by_type: bool,

    /// Volume change per scroll tick (0-100 scale).
    pub volume_scroll_step: u8,
    // Unused in GUI
    #[allow(dead_code)]
    pub enable_mouse_scroll_volume: bool,

    /// Enable app-managed queue for full playlist playback.
    /// Requires streaming. When disabled, playback uses Spotify-native queue
    /// management.
    pub custom_queue: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum Position {
    Top,
    Bottom,
}
config_parser_impl!(Position);

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub enum BorderType {
    Hidden,
    Plain,
    Rounded,
    Double,
    Thick,
}
config_parser_impl!(BorderType);

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum ProgressBarType {
    Line,
    Rectangle,
}
config_parser_impl!(ProgressBarType);

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum ProgressBarPosition {
    Bottom,
    Right,
}
config_parser_impl!(ProgressBarPosition);

#[derive(Debug, Deserialize, Serialize, ConfigParse, Clone)]
pub struct Command {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
}

/// Whitelist of allowed commands for security
const ALLOWED_COMMANDS: &[&str] = &[
    "/usr/bin/cat",
    "/usr/bin/head",
    "/usr/bin/tail",
    "/usr/bin/printf",
    "/usr/bin/echo",
    "/usr/bin/pass",
    "/usr/bin/secret-tool",
    "/usr/bin/security",
    "/usr/bin/gpg",
    "/usr/bin/gpg2",
    "/usr/bin/bw",
    "/usr/bin/op",
    "/usr/local/bin/bw",
    "/usr/local/bin/op",
    "/opt/homebrew/bin/bw",
    "/opt/homebrew/bin/op",
    "/bin/cat",
    "/bin/echo",
];

impl Command {
    pub fn new<C, A>(command: C, args: &[A]) -> Self
    where
        C: std::fmt::Display,
        A: std::fmt::Display,
    {
        Self {
            command: command.to_string(),
            args: args.iter().map(std::string::ToString::to_string).collect(),
        }
    }

    /// Execute a command, returning stdout if succeeded or stderr if failed
    /// 
    /// SECURITY: This function validates that the command is an absolute path
    /// to a known allowed binary. Commands with relative paths or shell
    /// metacharacters are rejected to prevent command injection.
    pub fn execute(&self, extra_args: Option<Vec<String>>) -> anyhow::Result<String> {
        // Validate command is an absolute path
        let cmd_path = std::path::Path::new(&self.command);
        if !cmd_path.is_absolute() {
            anyhow::bail!(
                "Command must be an absolute path, got: {}",
                self.command
            );
        }

        // Validate command is in whitelist
        if !ALLOWED_COMMANDS.contains(&self.command.as_str()) {
            anyhow::bail!(
                "Command not in allowed list: {}. Allowed commands: {:?}",
                self.command,
                ALLOWED_COMMANDS
            );
        }

        // Validate no shell metacharacters in args
        for arg in &self.args {
            if arg.chars().any(|c| matches!(c, ';' | '&' | '|' | '$' | '`' | '(' | ')' | '<' | '>' | '#' | '*' | '?' | '[' | ']' | '{' | '}' | '\\' | '"' | '\'' | '\n' | '\r')) {
                anyhow::bail!("Argument contains shell metacharacters: {}", arg);
            }
        }

        let mut args = self.args.clone();
        if let Some(extra) = extra_args {
            for arg in &extra {
                if arg.chars().any(|c| matches!(c, ';' | '&' | '|' | '$' | '`' | '(' | ')' | '<' | '>' | '#' | '*' | '?' | '[' | ']' | '{' | '}' | '\\' | '"' | '\'' | '\n' | '\r')) {
                    anyhow::bail!("Extra argument contains shell metacharacters: {}", arg);
                }
            }
            args.extend(extra);
        }

        let output = std::process::Command::new(&self.command)
            .args(&args)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            anyhow::bail!(stderr);
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(stdout)
    }
}

#[derive(Debug, Deserialize, Serialize, ConfigParse, Clone)]
/// Application device configurations
pub struct DeviceConfig {
    pub name: String,
    pub device_type: String,
    pub volume: u8,
    pub bitrate: u16,
    pub audio_cache: bool,
    pub normalization: bool,
    pub autoplay: bool,
}

#[derive(Debug, Deserialize, Serialize, ConfigParse, Clone)]
#[cfg(feature = "notify")]
pub struct NotifyFormat {
    pub summary: String,
    pub body: String,
}

#[derive(Debug, Deserialize, Serialize, ConfigParse, Clone)]
// Application layout configurations
pub struct LayoutConfig {
    pub library: LibraryLayoutConfig,
    // Unused in GUI
    #[allow(dead_code)]
    pub playback_window_position: Position,
    // Unused in GUI
    #[allow(dead_code)]
    pub playback_window_height: usize,
}

#[derive(Debug, Deserialize, Serialize, ConfigParse, Clone)]
pub struct LibraryLayoutConfig {
    pub playlist_percent: u16,
    pub album_percent: u16,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(from = "StreamingTypeOrBool")]
pub enum StreamingType {
    Always,
    DaemonOnly,
    Never,
}
config_parser_impl!(StreamingType);

// For backward compatibility, to accept booleans for enable_streaming
#[derive(Deserialize)]
enum RawStreamingType {
    #[serde(alias = "always")]
    Always,
    #[serde(alias = "daemononly", alias = "daemon_only", alias = "daemon-only")]
    DaemonOnly,
    #[serde(alias = "never")]
    Never,
}

#[allow(dead_code)]
#[derive(Deserialize)]
#[serde(untagged)]
enum StreamingTypeOrBool {
    Bool(bool),
    Type(RawStreamingType),
}

impl From<StreamingTypeOrBool> for StreamingType {
    fn from(v: StreamingTypeOrBool) -> Self {
        match v {
            StreamingTypeOrBool::Bool(true)
            | StreamingTypeOrBool::Type(RawStreamingType::Always) => StreamingType::Always,
            StreamingTypeOrBool::Bool(false)
            | StreamingTypeOrBool::Type(RawStreamingType::Never) => StreamingType::Never,
            StreamingTypeOrBool::Type(RawStreamingType::DaemonOnly) => StreamingType::DaemonOnly,
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: "Spotify".to_owned(),
            // Use ncspot's client ID as a fallback for user-provided client ID
            //
            // Most of the time, using ncspot's client ID is better than user-provided one
            // because it is registered with [extended quota mode] and predates [spotify API changes]
            //
            // [extended quota mode]: https://developer.spotify.com/documentation/web-api/concepts/quota-modes
            // [spotify API changes]: https://developer.spotify.com/blog/2024-11-27-changes-to-the-web-api
            client_id: Some(NCSPOT_CLIENT_ID.to_string()),
            client_id_command: None,

            client_port: 8080,

            login_redirect_uri: "http://127.0.0.1:8989/login".to_string(),

            log_folder: None,

            tracks_playback_limit: 50,

            playback_format: String::from(
                "{status} {track} • {artists} {liked}\n{album} • {genres}\n{metadata}",
            ),
            playback_metadata_fields: vec![
                "repeat".to_string(),
                "shuffle".to_string(),
                "volume".to_string(),
                "device".to_string(),
            ],
            #[cfg(feature = "notify")]
            notify_format: NotifyFormat {
                summary: String::from("{track} • {artists}"),
                body: String::from("{album}"),
            },
            #[cfg(feature = "notify")]
            notify_timeout_in_secs: 0,
            #[cfg(feature = "notify")]
            #[cfg(all(unix, not(target_os = "macos")))]
            notify_transient: false,

            player_event_hook_command: None,

            proxy: None,
            ap_port: None,
            app_refresh_duration_in_ms: 32,
            playback_refresh_duration_in_ms: 5000,

            page_size_in_rows: 20,

            pause_icon: "▌▌".to_string(),
            play_icon: "▶".to_string(),
            liked_icon: "♥".to_string(),
            explicit_icon: "(E)".to_string(),

            border_type: BorderType::Plain,
            progress_bar_type: ProgressBarType::Rectangle,
            progress_bar_position: ProgressBarPosition::Bottom,

            layout: LayoutConfig::default(),

            genre_num: 2,

            #[cfg(feature = "image")]
            cover_img_length: 9,
            #[cfg(feature = "image")]
            cover_img_width: 5,
            #[cfg(feature = "image")]
            cover_img_scale: 1.0,
            #[cfg(feature = "pixelate")]
            cover_img_pixels: 16,

            // Because of the "creating new window and stealing focus" behaviour
            // when running the media control event loop on startup,
            // media control support is disabled by default for Windows and MacOS.
            // Users will need to explicitly enable this option in their configuration files.
            #[cfg(feature = "media-control")]
            #[cfg(any(target_os = "macos", target_os = "windows"))]
            enable_media_control: false,
            #[cfg(feature = "media-control")]
            #[cfg(all(unix, not(target_os = "macos")))]
            enable_media_control: true,

            enable_streaming: StreamingType::Always,

            #[cfg(feature = "streaming")]
            enable_audio_visualization: false,

            #[cfg(feature = "notify")]
            enable_notify: true,

            enable_cover_image_cache: true,

            default_device: "spotify-player".to_string(),

            device: DeviceConfig::default(),

            #[cfg(all(feature = "streaming", feature = "notify"))]
            notify_streaming_only: false,

            seek_duration_secs: 5,

            sort_artist_albums_by_type: false,

            volume_scroll_step: 5,
            enable_mouse_scroll_volume: true,

            custom_queue: true,
        }
    }
}

impl Default for DeviceConfig {
    fn default() -> Self {
        Self {
            name: "spotify-player".to_string(),
            device_type: "speaker".to_string(),
            volume: 70,
            bitrate: 320,
            audio_cache: false,
            normalization: false,
            autoplay: false,
        }
    }
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            library: LibraryLayoutConfig {
                playlist_percent: 40,
                album_percent: 40,
            },
            playback_window_position: Position::Top,
            playback_window_height: 6,
        }
    }
}

impl LayoutConfig {
    pub fn check_values(&self) -> anyhow::Result<()> {
        if self.library.album_percent + self.library.playlist_percent > 99 {
            anyhow::bail!("Invalid library layout: summation of album_percent and playlist_percent cannot be greater than 99!");
        }
        Ok(())
    }
}

impl AppConfig {
    pub fn new(path: &Path) -> Result<Self> {
        let mut config = Self::default();
        if !config.parse_config_file(path)? {
            config.write_config_file(path)?;
        }

        config.layout.check_values()?;
        Ok(config)
    }

    // parses configurations from an application config file in `path` folder,
    // then updates the current configurations accordingly.
    // returns false if no config file found and true otherwise
    fn parse_config_file(&mut self, path: &Path) -> Result<bool> {
        let file_path = path.join(APP_CONFIG_FILE);
        match std::fs::read_to_string(file_path) {
            Ok(content) => self
                .parse(toml::from_str::<toml::Value>(&content)?)
                .map(|()| true),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error.into()),
        }
    }

    fn write_config_file(&self, path: &Path) -> Result<()> {
        toml::to_string_pretty(&self)
            .map_err(From::from)
            .and_then(|content| {
                std::fs::write(path.join(APP_CONFIG_FILE), content).map_err(From::from)
            })
    }

    pub fn session_config(&self) -> SessionConfig {
        let proxy = self
            .proxy
            .as_ref()
            .and_then(|proxy| match Url::parse(proxy) {
                Err(err) => {
                    tracing::warn!("failed to parse proxy url {proxy}: {err:#}");
                    None
                }
                Ok(url) => Some(url),
            });
        SessionConfig {
            proxy,
            ap_port: self.ap_port,
            client_id: SPOTIFY_CLIENT_ID.to_string(),
            autoplay: Some(self.device.autoplay),
            ..Default::default()
        }
    }

    /// Returns stdout of `client_id_command` if set, otherwise it returns the the value of `client_id`
    pub fn get_user_client_id(&self) -> Result<Option<String>> {
        match self.client_id_command {
            Some(ref cmd) => cmd.execute(None).map(|out| Some(out.trim().to_string())),
            None => Ok(self.client_id.clone()),
        }
    }
}

/// gets the application's configuration folder path
pub fn get_config_folder_path() -> Result<PathBuf> {
    match dirs_next::home_dir() {
        Some(home) => Ok(home.join(DEFAULT_CONFIG_FOLDER)),
        None => Err(anyhow!("cannot find the $HOME folder")),
    }
}

/// gets the application's cache folder path
pub fn get_cache_folder_path() -> Result<PathBuf> {
    match dirs_next::home_dir() {
        Some(home) => Ok(home.join(DEFAULT_CACHE_FOLDER)),
        None => Err(anyhow!("cannot find the $HOME folder")),
    }
}

pub fn get_config() -> Configs {
    CONFIGS
        .get()
        .expect("configs is already initialized")
        .lock()
        .clone()
}
pub fn set_config(configs: Configs) {
    CONFIGS.get_or_init(|| parking_lot::Mutex::new(configs));
}

/// Reload configuration from disk. Called after settings are saved in the GUI.
pub fn reload_config() -> anyhow::Result<()> {
    let config_folder = get_config_folder_path()?;
    let cache_folder = get_cache_folder_path()?;
    let new_configs = Configs::new(&config_folder, &cache_folder)?;
    let lock = CONFIGS.get().expect("configs is already initialized");
    *lock.lock() = new_configs;
    Ok(())
}


