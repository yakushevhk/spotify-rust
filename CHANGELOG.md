# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Comprehensive documentation (README.md, ARCHITECTURE.md, API.md, CHANGELOG.md)
- Module-level documentation for all modules
- Public API documentation for ClientRequest, PlayerRequest, and Command types
- Configuration field documentation
- Theme and keymap documentation

## [0.1.0] - 2025-01-XX

### Added
- Initial release of Spotify Player GUI
- Modern dark GUI built with egui
- Full Spotify playback control (play, pause, skip, seek, shuffle, repeat, volume)
- Library browsing (playlists, albums, artists, saved tracks)
- Search functionality for tracks, albums, artists, playlists, and podcasts
- Browse categories and featured playlists
- Podcast/episode support
- Vim-inspired keyboard shortcuts with extensive customization
- Theme system with customizable colors
- Audio streaming via librespot (optional feature)
- System media key support (optional feature)
- Desktop notifications (optional feature)
- Lyrics display
- Queue management
- Device switching (Spotify Connect)
- Playlist creation and management
- Toast notification system
- In-app logging viewer
- Command palette for quick access
- Context menus for tracks and items
- Image caching for album artwork
- Multi-threaded architecture with Tokio
- Signal handling for graceful shutdown
- File-based caching for user data
- Memory caching with TTL for API responses
- Comprehensive test coverage

### Security
- OAuth PKCE authentication flow
- Credential caching with restricted permissions
- Image filename sanitization to prevent path traversal
- Command execution whitelist for security
- Shell metacharacter validation

### Features
- `streaming` - Local audio streaming (enabled by default)
- `media-control` - System media key support (enabled by default)
- `notify` - Desktop notifications (enabled by default)
- `image` - Image support (disabled by default)
- `pixelate` - Pixelated image effect (disabled by default)
- `fzf` - Fuzzy matching (disabled by default)

## Future Releases

### Planned for [0.2.0]
- [ ] Audio visualization improvements
- [ ] Mini player mode
- [ ] Lyrics synchronization improvements
- [ ] Playlist folder management
- [ ] Advanced search filters
- [ ] Keyboard shortcut cheat sheet overlay
- [ ] Performance optimizations

### Planned for [1.0.0]
- [ ] Full test coverage
- [ ] CI/CD pipeline
- [ ] Automated releases
- [ ] Cross-platform testing
- [ ] Plugin system
- [ ] Advanced theming
- [ ] Accessibility improvements

## Breaking Changes

### [0.1.0]
- Initial release, no breaking changes

## Migration Guide

### Migrating to 0.2.0 (when released)
No migration steps required yet.

## Known Issues

### Current
- None reported

### Fixed
- See individual release notes

## Contributors

Thanks to all contributors who have helped shape this project!

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
