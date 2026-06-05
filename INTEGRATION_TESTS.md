# Integration Test Scenarios

This document describes comprehensive integration test scenarios for the Spotify Player GUI application. These tests validate end-to-end functionality from user interaction through API communication to state updates.

## Table of Contents

1. [Authentication Flow](#1-authentication-flow)
2. [Playback Flow](#2-playback-flow)
3. [Library Flow](#3-library-flow)
4. [Search Flow](#4-search-flow)
5. [Error Scenarios](#5-error-scenarios)
6. [Test Environment Setup](#test-environment-setup)
7. [Logging Guidelines](#logging-guidelines)

---

## 1. Authentication Flow

Authentication is the foundation of all Spotify API interactions. These scenarios test the OAuth PKCE flow, credential caching, and token management.

### 1.1 Fresh Start (No Credentials)

**Objective**: Verify authentication flow works when no cached credentials exist.

**Preconditions**:
- `~/.cache/spotify-player/credentials.json` does not exist
- `~/.cache/spotify-player/` directory may or may not exist
- No existing Spotify session

**Test Procedure**:
1. Start the application: `cargo run`
2. Observe console output for authentication prompt
3. Verify browser opens automatically to Spotify OAuth page
4. Verify redirect URI is `http://127.0.0.1:8989/login`
5. Complete OAuth flow in browser (login and authorize)
6. Observe application receives callback and stores credentials
7. Verify application starts successfully with authenticated state
8. Verify user data loads (check user profile displays)

**Expected Behavior**:
- Console shows "No cached credentials found, please authenticate the application first."
- Browser opens OAuth page with correct scopes (see `OAUTH_SCOPES` in auth.rs)
- Application creates cache directory with permissions 0o700
- Application creates credentials.json with permissions 0o600
- User data is fetched and displayed
- Application state shows authenticated session

**Log Checks**:
```
INFO  Using cached credentials  # Should NOT appear
INFO  Starting OAuth authentication...
INFO  Opening browser for authentication...
INFO  OAuth callback received
INFO  Credentials stored successfully
INFO  Fetching current user data...
```

**Common Failure Points**:
- **Browser fails to open**: Check `open` command availability on system
- **Port 8989 in use**: Another process may be using the redirect port
- **OAuth timeout**: User takes too long to complete browser flow
- **Permission denied**: Cannot create cache directory (check parent permissions)
- **Network error**: Cannot reach Spotify OAuth endpoints

---

### 1.2 Cached Credentials

**Objective**: Verify application uses cached credentials on subsequent starts.

**Preconditions**:
- Previous successful authentication
- `~/.cache/spotify-player/credentials.json` exists with valid token
- Token has not expired

**Test Procedure**:
1. Verify `~/.cache/spotify-player/credentials.json` exists
2. Check file permissions: `ls -la ~/.cache/spotify-player/credentials.json`
   - Should show `-rw-------` (0o600)
3. Start the application
4. Observe startup logs
5. Verify no browser opens for authentication
6. Verify application starts immediately
7. Verify user data loads correctly

**Expected Behavior**:
- Console shows "Using cached credentials"
- No browser popup
- Immediate application startup
- Valid session established
- All user data accessible

**Log Checks**:
```
INFO  Using cached credentials
INFO  Session established
INFO  Fetching current user data...
INFO  User: [username]
INFO  Loading playlists...
INFO  Loading saved albums...
INFO  Loading followed artists...
```

**Common Failure Points**:
- **Token expired**: Cached token no longer valid (test token refresh)
- **Corrupted credentials file**: Invalid JSON format
- **Wrong file permissions**: File readable by others (security risk)
- **Cache directory moved**: Application can't find cached file

---

### 1.3 Token Refresh

**Objective**: Verify automatic token refresh when token expires.

**Preconditions**:
- Valid refresh token in cache
- Access token has expired (or is about to expire)
- Network connectivity available

**Test Procedure**:
1. Modify cached credentials to simulate expired access token:
   - Edit `~/.cache/spotify-player/credentials.json`
   - Set `expires_at` to a past timestamp
2. Start the application
3. Attempt any API operation (e.g., load playlists, search)
4. Verify token is refreshed automatically
5. Verify operation completes successfully
6. Check credentials file for new token

**Expected Behavior**:
- Application detects expired token
- Automatic token refresh via librespot session
- API operations continue without interruption
- No user intervention required
- New access token stored in cache

**Log Checks**:
```
INFO  Token expired, refreshing...
INFO  Requesting new token from Spotify...
INFO  Token refresh successful
INFO  Updated credentials in cache
```

**Common Failure Points**:
- **Session invalid**: librespot session is disconnected
- **Network timeout**: Token refresh request times out (check log for "timed out" message)
- **Refresh token revoked**: User revoked app access in Spotify account
- **Rate limit on token endpoint**: Too many refresh attempts

**Special Handling** (see `spotify.rs:refetch_token`):
```
WARN  Token refresh timed out, keeping existing token
```
This indicates timeout handling is working - app keeps existing token temporarily.

---

### 1.4 Invalid Token

**Objective**: Verify handling of invalid or revoked tokens.

**Preconditions**:
- Cached credentials exist but are invalid
- Scenarios: manually corrupted, revoked by user, or from different client

**Test Procedure**:
1. **Scenario A - Corrupted Token**:
   - Edit `~/.cache/spotify-player/credentials.json`
   - Replace `access_token` with invalid string
   - Start application, attempt API operation

2. **Scenario B - Revoked Access**:
   - Go to Spotify account settings → Apps
   - Revoke access for "spotify-player-gui"
   - Start application, attempt API operation

3. **Scenario C - Different Client ID**:
   - Modify client_id in credentials to different value
   - Start application, attempt API operation

**Expected Behavior**:
- API calls fail with authentication error
- Session marked as invalid
- Re-authentication flow initiated automatically
- User notified of authentication issue
- Credentials cache cleared
- Browser opens for new OAuth flow

**Log Checks**:
```
ERROR Authentication failed: invalid_token
ERROR Session is invalid, cannot refresh token
WARN  Clearing invalid credentials...
INFO  Initiating re-authentication...
INFO  Opening browser for authentication...
```

**Common Failure Points**:
- **Silent failure**: App continues with invalid token (should not happen)
- **Infinite retry loop**: Token refresh keeps failing without re-auth
- **Crash**: Unhandled exception when session is invalid
- **Cache not cleared**: Old credentials persist causing repeated failures

---

## 2. Playback Flow

Playback control is core functionality. These scenarios test all playback-related operations.

### 2.1 Play Track

**Objective**: Verify track playback initiation from various contexts.

**Preconditions**:
- Valid authenticated session
- At least one active playback device (Spotify Connect device or integrated player)
- Test playlist or album with known tracks

**Test Procedure**:
1. **From Playlist**:
   - Navigate to library → select playlist
   - Press Enter on any track
   - Verify playback starts

2. **From Album**:
   - Navigate to library → select album
   - Press Enter on any track
   - Verify playback starts

3. **From Search**:
   - Press `g s` to open search
   - Search for a track
   - Press Enter on result
   - Verify playback starts

4. **From Queue**:
   - Add track to queue (`Z` or `Ctrl+z`)
   - Navigate to queue view (`z` or `Ctrl+q`)
   - Verify track appears in queue

**Expected Behavior**:
- Playback state updates immediately in UI
- Track metadata displays correctly (title, artist, album, duration)
- Progress bar starts moving
- Album art loads and displays
- Device shows as active in device list

**Log Checks**:
```
INFO  Starting playback: track_uri=[uri]
INFO  Playback started on device: [device_name]
INFO  Current playback: is_playing=true, track=[track_name]
INFO  Buffered playback metadata updated
```

**Common Failure Points**:
- **No active device**: All devices are inactive
- **Device not found**: Selected device disconnected
- **Premium required**: User has free account (some features restricted)
- **Track unavailable**: Track not available in user's region
- **Context loading**: Playlist/album tracks not loaded yet

---

### 2.2 Pause/Resume

**Objective**: Verify play/pause toggle functionality.

**Preconditions**:
- Valid authenticated session
- Active playback (track currently playing)

**Test Procedure**:
1. **Pause While Playing**:
   - Start playback of any track
   - Press `Space` to pause
   - Verify playback stops
   - Verify progress bar freezes

2. **Resume While Paused**:
   - With playback paused
   - Press `Space` to resume
   - Verify playback continues from pause point

3. **Rapid Toggle**:
   - Press `Space` rapidly 5-10 times
   - Verify no UI glitches
   - Verify final state matches button state

4. **From Playback Bar**:
   - Click play/pause button in playback bar
   - Verify consistent behavior with keyboard

**Expected Behavior**:
- Immediate UI feedback (no delay)
- Progress bar pauses/resumes correctly
- Server state syncs within `playback_refresh_duration_in_ms`
- Buffered playback state updates immediately
- Playback metadata preserved across pause/resume

**Log Checks**:
```
INFO  Player request: ResumePause
INFO  Playback toggled: is_playing=false
INFO  Buffered playback: is_playing=false
INFO  Server playback state synced
```

**Common Failure Points**:
- **State desync**: UI shows different state than server
- **Delay in response**: Long delay before playback pauses
- **Progress bar jumps**: Progress estimation incorrect
- **Seek position lost**: Position resets on pause/resume

---

### 2.3 Next/Previous Track

**Objective**: Verify track navigation functionality.

**Preconditions**:
- Valid authenticated session
- Active playback context (playlist/album with multiple tracks)
- Queue with multiple tracks (optional)

**Test Procedure**:
1. **Next Track (→)**:
   - Start playback from playlist
   - Press right arrow key
   - Verify next track in context plays
   - Verify track index increments

2. **Previous Track (←)**:
   - With playback past first track
   - Press left arrow key
   - Verify previous track plays

3. **Previous at Start**:
   - Start playback at first track
   - Press left arrow immediately
   - Verify track restarts (doesn't go negative)

4. **With Shuffle Enabled**:
   - Enable shuffle (`Ctrl+s`)
   - Press next/previous multiple times
   - Verify random track selection
   - Verify no immediate repeats

5. **From Queue**:
   - Add multiple tracks to queue
   - Navigate through queue with next/prev
   - Verify queue order respected

**Expected Behavior**:
- Track changes immediately
- Track metadata updates in UI
- Progress resets to 0:00
- Queue updates accordingly
- History navigation works

**Log Checks**:
```
INFO  Player request: NextTrack
INFO  Track changed: from=[track1] to=[track2]
INFO  Playback position reset
INFO  Queue updated
```

**Common Failure Points**:
- **Track skipped twice**: API call executed twice
- **Wrong track plays**: Context/queue mismatch
- **Shuffle not random**: Same track plays repeatedly
- **Queue ignored**: Next track not from queue
- **Boundary issues**: Previous at track 0 causes error

---

### 2.4 Seek

**Objective**: Verify seek functionality (position changes within track).

**Preconditions**:
- Valid authenticated session
- Active playback of track with sufficient duration (>1 minute)

**Test Procedure**:
1. **Seek Forward (>)**:
   - Start playback of long track
   - Press `>` to seek forward
   - Default seek duration: `seek_duration_secs` from config (default 5s)
   - Verify progress bar jumps forward
   - Verify audio position matches

2. **Seek Backward (<)**:
   - Press `<` to seek backward
   - Verify progress bar jumps backward
   - Verify audio position matches

3. **Seek to Start (^)**:
   - Press `^` to seek to track start
   - Verify position resets to 0:00
   - Verify playback continues from start

4. **Seek to End**:
   - Seek past track duration
   - Verify next track plays automatically

5. **While Paused**:
   - Pause playback
   - Seek forward/backward
   - Verify position updates
   - Resume and verify position

6. **Rapid Seeks**:
   - Press `>` or `<` rapidly
   - Verify seek_deadline prevents race conditions
   - Verify final position is correct

**Expected Behavior**:
- Progress bar updates immediately in UI (buffered)
- Seek deadline prevents progress estimation race
- Actual audio position syncs within 500ms
- Progress bar doesn't race ahead during seek

**Log Checks**:
```
INFO  Player request: SeekTrack(position=[duration])
INFO  Seek deadline set: 500ms
INFO  Buffered playback: progress=[new_progress]
INFO  Seek deadline expired
INFO  Server playback state synced
```

**Common Failure Points**:
- **Progress bar jumps ahead**: seek_deadline not working
- **Seek ignored**: API call failed silently
- **Wrong position**: Progress calculation incorrect
- **Audio desync**: UI position doesn't match audio

**Special Handling** (see `player.rs`):
- `seek_deadline` prevents progress estimation for 500ms after seek
- Progress clamped to track duration to prevent overflow

---

### 2.5 Volume Change

**Objective**: Verify volume control functionality.

**Preconditions**:
- Valid authenticated session
- Active playback device with volume control

**Test Procedure**:
1. **Volume Up (Ctrl+i)**:
   - Note current volume
   - Press `Ctrl+i`
   - Verify volume increments by `volume_scroll_step` (default 5%)
   - Verify device volume updates

2. **Volume Down (Ctrl+d)**:
   - Press `Ctrl+d`
   - Verify volume decrements
   - Verify minimum is 0%

3. **Mute Toggle (_)**:
   - Press `_` to mute
   - Verify volume shows 0%
   - Verify `mute_state` stores previous volume
   - Press `_` again
   - Verify volume restored

4. **Mute at 0%**:
   - Set volume to 0%
   - Press `_` to unmute
   - Verify volume set to 50% (default)

5. **Volume Clamp**:
   - Attempt to set volume > 100%
   - Attempt to set volume < 0%
   - Verify clamped to valid range

**Expected Behavior**:
- Immediate UI feedback on volume change
- Volume persists between tracks
- Mute state preserved across track changes
- Device volume syncs with Spotify Connect
- Volume change reflected in buffered_playback

**Log Checks**:
```
INFO  Player request: Volume(volume=[new_volume])
INFO  Buffered playback: volume=[new_volume]
INFO  Mute toggled: is_muted=[true/false], restore_volume=[volume]
```

**Common Failure Points**:
- **Volume not applied**: Device doesn't support volume control
- **Mute state lost**: Previous volume not stored
- **Spotify Connect desync**: Device volume different from app
- **Integer overflow**: Volume calculation produces invalid value

---

### 2.6 Shuffle/Repeat Toggle

**Objective**: Verify shuffle and repeat mode toggling.

**Preconditions**:
- Valid authenticated session
- Active playback context (playlist/album with multiple tracks)

**Test Procedure**:
1. **Shuffle Off → On**:
   - Start playback with shuffle off
   - Press `Ctrl+s`
   - Verify shuffle icon appears in UI
   - Press next track multiple times
   - Verify tracks play in random order

2. **Shuffle On → Off**:
   - With shuffle enabled
   - Press `Ctrl+s`
   - Verify shuffle icon disappears
   - Press next track
   - Verify tracks play in original order

3. **Repeat Cycle**:
   - Note current repeat state
   - Press `Ctrl+r` multiple times
   - Verify cycle: Off → Track → Context → Off
   - Verify repeat icon changes accordingly

4. **Repeat Track**:
   - Set repeat to "Track"
   - Let current track finish
   - Verify same track restarts

5. **Repeat Context**:
   - Set repeat to "Context"
   - Play last track in context
   - Let track finish
   - Verify first track in context plays

**Expected Behavior**:
- Immediate UI state update
- Server state syncs
- Playback mode affects track selection
- Repeat modes cycle correctly
- State persists across playback sessions

**Log Checks**:
```
INFO  Player request: Shuffle
INFO  Shuffle toggled: shuffle_state=[true/false]
INFO  Player request: Repeat
INFO  Repeat changed: repeat_state=[Off/Track/Context]
INFO  Buffered playback updated
```

**Common Failure Points**:
- **State desync**: UI shows different state than server
- **Shuffle not random**: Same pattern each time
- **Repeat ignored**: Track/context not repeated
- **State not persisted**: Reverts on next track

---

### 2.7 Device Switch

**Objective**: Verify playback device switching.

**Preconditions**:
- Valid authenticated session
- Multiple playback devices available (Spotify Connect devices)
- At least one device active or available

**Test Procedure**:
1. **List Devices**:
   - Press `D` to open device switcher
   - Verify all available devices listed
   - Note current active device

2. **Switch to Another Device**:
   - Select different device
   - Press Enter
   - Verify playback transfers
   - Verify playback position preserved (if supported)

3. **Switch While Playing**:
   - Start playback on current device
   - Switch to different device mid-track
   - Verify playback continues on new device
   - Verify position preserved

4. **No Active Device**:
   - When no device is active
   - Attempt to start playback
   - Verify device switcher opens automatically
   - Select device and verify playback starts

5. **Device Disconnects**:
   - During playback, disconnect device
   - Verify playback stops gracefully
   - Verify error shown to user

**Expected Behavior**:
- Device list refreshes when opened
- Switching devices preserves playback state
- Track position preserved (within limits)
- Clear indication of active device
- Toast notification on successful transfer

**Log Checks**:
```
INFO  Player request: TransferPlayback(device_id=[id], play=[true/false])
INFO  Fetching devices...
INFO  Devices found: [count]
INFO  Playback transferred to: [device_name]
INFO  Current playback updated
```

**Common Failure Points**:
- **Device not found**: Device went offline between list and select
- **Transfer failed**: Device rejected transfer
- **Position lost**: Playback restarts from beginning
- **Multiple devices active**: Confusing state

---

## 3. Library Flow

Library management includes loading, displaying, and modifying user's music collection.

### 3.1 Load Playlists

**Objective**: Verify playlist loading from user library.

**Preconditions**:
- Valid authenticated session
- User has playlists (owned and followed)
- Network connectivity available

**Test Procedure**:
1. **Initial Load**:
   - Start application
   - Navigate to library view (`g l`)
   - Verify playlists appear in sidebar
   - Verify loading indicator shown then hidden

2. **Playlist Folders**:
   - If user has playlist folders
   - Verify folder structure displayed
   - Expand/collapse folders
   - Verify correct playlists inside folders

3. **Playlist Details**:
   - Select playlist
   - Press Enter
   - Verify playlist tracks load
   - Verify playlist metadata (name, description, cover)

4. **Pagination**:
   - Load user with 50+ playlists
   - Scroll through all playlists
   - Verify all load correctly
   - Verify no duplicates

5. **Offline Cache**:
   - Load playlists once
   - Restart application
   - Verify playlists load from cache first
   - Verify background refresh from API

**Expected Behavior**:
- Playlists load within reasonable time (<3s)
- Loading state indicated in UI
- Folders expandable/collapsible
- Playlist metadata complete
- Cache used on subsequent loads
- Background refresh updates list

**Log Checks**:
```
INFO  Loading playlists from cache...
INFO  Found [n] cached playlists
INFO  Fetching playlists from API...
INFO  Playlists fetched: [count] total
INFO  Storing playlists to cache
INFO  Playlist folders processed: [count] folders
```

**Common Failure Points**:
- **Partial load**: Only first 50 playlists loaded (pagination issue)
- **Missing folders**: Folder structure not parsed
- **Stale cache**: Cache not updating with new playlists
- **Permission denied**: Can't read playlist details
- **Rate limiting**: Too many API calls

---

### 3.2 Load Albums

**Objective**: Verify saved albums loading.

**Preconditions**:
- Valid authenticated session
- User has saved albums

**Test Procedure**:
1. **Initial Load**:
   - Navigate to library
   - Switch to albums view (`u A`)
   - Verify saved albums appear
   - Verify album art loads

2. **Album Details**:
   - Select album
   - Press Enter
   - Verify album tracks load
   - Verify album metadata (artist, year, tracks)

3. **Pagination**:
   - Load user with 50+ saved albums
   - Scroll through all albums
   - Verify all load correctly

4. **Cache Behavior**:
   - Load albums once
   - Verify cache file created
   - Restart application
   - Verify albums load from cache

**Expected Behavior**:
- Albums load in library view
- Album art displays correctly
- Track list accessible
- Pagination handles large libraries
- Cache persists between sessions

**Log Checks**:
```
INFO  Loading saved albums from cache...
INFO  Found [n] cached albums
INFO  Fetching saved albums from API...
INFO  Albums fetched: [count]
INFO  Storing albums to cache
```

**Common Failure Points**:
- **Missing album art**: Image URLs not loaded
- **Partial load**: Pagination not handled
- **Slow loading**: Large album art blocking UI

---

### 3.3 Load Artists

**Objective**: Verify followed artists loading.

**Preconditions**:
- Valid authenticated session
- User follows artists

**Test Procedure**:
1. **Initial Load**:
   - Navigate to library
   - Switch to artists view (`u a`)
   - Verify followed artists appear

2. **Artist Details**:
   - Select artist
   - Press Enter
   - Verify artist page loads
   - Verify top tracks appear
   - Verify albums/discography accessible

3. **Pagination**:
   - Load user with 50+ followed artists
   - Scroll through all artists
   - Verify all load correctly

4. **Follow/Unfollow**:
   - Open artist context menu (`g a`)
   - Select "Unfollow"
   - Verify artist removed from list
   - Follow again, verify added back

**Expected Behavior**:
- Artists load in library view
- Artist details accessible
- Top tracks displayed
- Follow/unfollow works
- Cache persists

**Log Checks**:
```
INFO  Loading followed artists from cache...
INFO  Fetching followed artists from API...
INFO  Artists fetched: [count]
INFO  Storing artists to cache
```

**Common Failure Points**:
- **Empty list**: No followed artists (expected if none)
- **Missing top tracks**: Artist API failed
- **Follow state not synced**: Local state different from server

---

### 3.4 Load Tracks

**Objective**: Verify saved/liked tracks loading.

**Preconditions**:
- Valid authenticated session
- User has liked tracks

**Test Procedure**:
1. **Liked Tracks View**:
   - Press `g y` to open liked tracks
   - Verify all saved tracks appear
   - Verify track metadata (title, artist, album, duration)

2. **Track Sorting**:
   - Test sort by title (`s t`)
   - Test sort by artist (`s a`)
   - Test sort by album (`s A`)
   - Test sort by duration (`s d`)
   - Test sort by added date (`s D`)
   - Test reverse order (`s r`)

3. **Pagination**:
   - Load user with 500+ liked tracks
   - Scroll through all tracks
   - Verify lazy loading or pagination

4. **Play from Liked Tracks**:
   - Select track
   - Press Enter
   - Verify playback starts from liked tracks context
   - Verify subsequent tracks are from liked tracks

5. **Cache Behavior**:
   - Load tracks once
   - Verify cache file created
   - Modify liked tracks on another device
   - Verify app refreshes cache

**Expected Behavior**:
- All liked tracks load
- Sorting works correctly
- Playback from liked tracks works
- Cache updates on changes
- Performance acceptable with large libraries

**Log Checks**:
```
INFO  Loading saved tracks from cache...
INFO  Fetching saved tracks from API...
INFO  Tracks fetched: [count]
INFO  Storing tracks to cache
INFO  Loading liked tracks context...
INFO  Context loaded with [count] tracks
```

**Common Failure Points**:
- **Incomplete load**: Not all tracks loaded (pagination issue)
- **Sort not working**: Sort state not applied
- **Performance issue**: UI lag with many tracks
- **Cache too large**: File size excessive

---

### 3.5 Create Playlist

**Objective**: Verify playlist creation functionality.

**Preconditions**:
- Valid authenticated session
- User has permission to create playlists

**Test Procedure**:
1. **Create Basic Playlist**:
   - Press `N` to create playlist
   - Enter playlist name
   - Leave description empty
   - Set public/private
   - Press Enter to create
   - Verify success toast
   - Verify playlist appears in library

2. **Create with Description**:
   - Press `N`
   - Enter name and description
   - Create playlist
   - Verify description saved

3. **Create Collaborative Playlist**:
   - Press `N`
   - Enable collaborative option
   - Create playlist
   - Verify collaborative icon appears

4. **Create and Add Tracks**:
   - Create new playlist
   - Navigate to album or other playlist
   - Select tracks
   - Add to new playlist
   - Verify tracks added

5. **Error Handling**:
   - Try to create playlist with empty name
   - Verify validation error
   - Try to create with very long name (>100 chars)
   - Verify truncation or error

**Expected Behavior**:
- Playlist created immediately
- Appears in library sidebar
- All settings applied correctly
- Toast notification confirms creation
- Playlist is editable by user

**Log Checks**:
```
INFO  Creating playlist: name=[name], public=[bool], collaborative=[bool]
INFO  Playlist created: id=[playlist_id]
INFO  Fetching updated playlists...
INFO  New playlist added to library
```

**Common Failure Points**:
- **Name validation**: Empty or invalid characters
- **Permission denied**: User can't create playlists
- **Network timeout**: API call times out
- **Playlist not visible**: Cache not updated

---

### 3.6 Add to Playlist

**Objective**: Verify adding tracks to playlists.

**Preconditions**:
- Valid authenticated session
- User has at least one playlist they can modify
- Tracks available to add (from album, playlist, search)

**Test Procedure**:
1. **Add Single Track**:
   - Navigate to any track list
   - Select track
   - Open context menu (`g a` or `Ctrl+Space`)
   - Select "Add to playlist"
   - Choose playlist
   - Verify success toast
   - Navigate to playlist
   - Verify track appears at end

2. **Add Multiple Tracks**:
   - Navigate to album
   - Use visual select (if supported) or multiple selections
   - Add all selected to playlist
   - Verify all added

3. **Add to Collaborative Playlist**:
   - Add to collaborative playlist
   - Verify success
   - Verify other collaborators can see

4. **Add Duplicate Track**:
   - Add track that already exists in playlist
   - Verify duplicate added (Spotify allows this)
   - Or verify warning shown

5. **Add to Full Playlist**:
   - Attempt to add to playlist at 10,000 track limit
   - Verify appropriate error shown

**Expected Behavior**:
- Track added to end of playlist
- Playlist snapshot updated
- Success notification shown
- Playlist track count increases
- No duplicates warning (if implemented)

**Log Checks**:
```
INFO  Adding track [track_uri] to playlist [playlist_id]
INFO  Add to playlist request sent
INFO  Playlist updated, new snapshot: [snapshot_id]
INFO  Toast: Track added to playlist
```

**Common Failure Points**:
- **Playlist not modifiable**: User doesn't own playlist
- **Network error**: API call failed
- **Wrong playlist**: Selected wrong one in UI
- **Snapshot conflict**: Playlist modified concurrently

---

### 3.7 Remove from Playlist

**Objective**: Verify removing tracks from playlists.

**Preconditions**:
- Valid authenticated session
- User owns or has collaborative access to playlist
- Playlist has multiple tracks

**Test Procedure**:
1. **Remove Single Track**:
   - Navigate to owned playlist
   - Select track
   - Open context menu (`g a`)
   - Select "Remove from playlist"
   - Verify track removed
   - Verify order of remaining tracks preserved

2. **Remove Multiple Tracks**:
   - Navigate to playlist
   - Select multiple tracks (if supported)
   - Remove all at once
   - Verify all removed

3. **Reorder Before Remove**:
   - Move track to different position
   - Then remove it
   - Verify correct track removed

4. **Undo Removal** (if supported):
   - Remove track
   - Check if undo available
   - Verify track restored

5. **Error Cases**:
   - Try to remove from followed playlist (not owned)
   - Verify error shown
   - Try to remove from collaborative playlist (if not collaborator)
   - Verify error shown

**Expected Behavior**:
- Track removed immediately from UI
- Playlist snapshot updated
- Remaining tracks keep positions
- Success notification shown
- Undo available (if implemented)

**Log Checks**:
```
INFO  Removing track at position [index] from playlist [playlist_id]
INFO  Playlist items reordered
INFO  New snapshot: [snapshot_id]
INFO  Toast: Track removed from playlist
```

**Common Failure Points**:
- **Wrong index**: Track positions shifted during operation
- **Snapshot conflict**: Playlist modified by someone else
- **Permission denied**: Can't modify playlist
- **Network error**: API call failed

---

## 4. Search Flow

Search functionality allows finding content across Spotify's catalog.

### 4.1 Search Tracks

**Objective**: Verify track search functionality.

**Preconditions**:
- Valid authenticated session
- Network connectivity available

**Test Procedure**:
1. **Basic Search**:
   - Press `g s` to open search
   - Enter search query (e.g., "radiohead")
   - Press Enter
   - Verify track results appear
   - Verify track metadata displayed

2. **Empty Search**:
   - Clear search field
   - Press Enter
   - Verify no results or recent searches

3. **Search with Filters**:
   - Search with filter: "artist:radiohead"
   - Search with filter: "album:ok computer"
   - Verify filtered results

4. **Special Characters**:
   - Search for tracks with special characters
   - Verify handling of accents, unicode
   - Verify no errors

5. **No Results**:
   - Search for nonsense string: "xyzabc123notfound"
   - Verify empty results shown
   - Verify no crash

6. **Play from Search**:
   - Search for track
   - Select result
   - Press Enter
   - Verify playback starts

**Expected Behavior**:
- Search results appear within 1-2 seconds
- Relevant tracks displayed
- Album art loads
- Can play directly from results
- Search cached for session

**Log Checks**:
```
INFO  Search request: query=[query]
INFO  Search API called
INFO  Tracks found: [count]
INFO  Search results cached
```

**Common Failure Points**:
- **Slow search**: API response timeout
- **No results**: Search API failed silently
- **Rate limited**: Too many searches in short time
- **Encoding issues**: Special characters not handled

---

### 4.2 Search Albums

**Objective**: Verify album search functionality.

**Test Procedure**:
1. **Search for Album**:
   - Open search (`g s`)
   - Enter album name
   - Switch to albums tab or view albums in results
   - Verify album results

2. **Album with Same Name**:
   - Search for common album name (e.g., "Greatest Hits")
   - Verify multiple artists shown
   - Verify album covers differ

3. **Album Details from Search**:
   - Click album result
   - Verify album page opens
   - Verify tracks listed

4. **Play Album from Search**:
   - Select album
   - Press Enter or play
   - Verify album plays from start

**Expected Behavior**:
- Album results relevant
- Album art displayed
- Artist name shown
- Year of release shown
- Can navigate to album

**Log Checks**:
```
INFO  Search request: query=[query], type=albums
INFO  Albums found: [count]
```

**Common Failure Points**:
- Same as track search

---

### 4.3 Search Artists

**Objective**: Verify artist search functionality.

**Test Procedure**:
1. **Search for Artist**:
   - Open search (`g s`)
   - Enter artist name
   - View artist results

2. **Artist Profile**:
   - Click artist result
   - Verify artist page opens
   - Verify top tracks shown
   - Verify discography accessible

3. **Follow Artist**:
   - From artist page
   - Open context menu
   - Follow artist
   - Verify appears in followed artists

4. **Similar Artist Names**:
   - Search for artist with common name
   - Verify multiple artists shown
   - Verify profile images help distinguish

**Expected Behavior**:
- Artist results relevant
- Profile image shown
- Follower count shown
- Can navigate to artist page

**Log Checks**:
```
INFO  Search request: query=[query], type=artists
INFO  Artists found: [count]
```

---

### 4.4 Search Playlists

**Objective**: Verify playlist search functionality.

**Test Procedure**:
1. **Search for Playlist**:
   - Open search (`g s`)
   - Enter playlist name or keywords
   - View playlist results

2. **Playlist Details**:
   - Click playlist result
   - Verify playlist opens
   - Verify tracks listed
   - Verify playlist metadata (name, creator, description)

3. **Follow Playlist**:
   - From search result
   - Add playlist to library
   - Verify appears in library

4. **Play Playlist from Search**:
   - Select playlist
   - Press Enter
   - Verify playlist plays

**Expected Behavior**:
- Playlist results relevant
- Playlist cover shown
- Creator name shown
- Track count shown
- Can navigate to playlist

**Log Checks**:
```
INFO  Search request: query=[query], type=playlists
INFO  Playlists found: [count]
```

---

### 4.5 Play from Search

**Objective**: Verify playback initiation from search results.

**Preconditions**:
- Active search results displayed
- Active playback device available

**Test Procedure**:
1. **Play Track Result**:
   - Search for track
   - Select track result
   - Press Enter
   - Verify playback starts immediately
   - Verify correct track plays

2. **Play Album Result**:
   - Search for album
   - Select album result
   - Press Enter
   - Verify album plays from first track

3. **Play Artist Top Track**:
   - Search for artist
   - Navigate to artist page
   - Play top track
   - Verify playback starts

4. **Play Playlist Result**:
   - Search for playlist
   - Select playlist
   - Press Enter
   - Verify playlist plays from start

5. **Add to Queue from Search**:
   - Search for track
   - Select track
   - Press `Z` (or `Ctrl+z`)
   - Verify track added to queue

**Expected Behavior**:
- Playback starts immediately on selection
- Correct context plays (track, album, playlist)
- Queue updates appropriately
- Metadata displays correctly

**Log Checks**:
```
INFO  Playback started from search: context=[type], uri=[uri]
INFO  Queue updated
```

**Common Failure Points**:
- **No active device**: Can't start playback
- **Wrong context**: Album plays as single track
- **Queue not updated**: Add to queue failed

---

## 5. Error Scenarios

These scenarios test error handling and recovery.

### 5.1 Network Failure

**Objective**: Verify graceful handling of network issues.

**Preconditions**:
- Application running
- Ability to simulate network failure

**Test Procedure**:
1. **Network Disconnect During Load**:
   - Start loading playlists
   - Disconnect network
   - Verify error message shown
   - Verify app doesn't crash
   - Reconnect network
   - Verify retry works

2. **Network Disconnect During Playback**:
   - Start playback
   - Disconnect network
   - Verify playback continues if streaming
   - Or verify appropriate error if Spotify Connect
   - Reconnect and verify recovery

3. **Slow Network**:
   - Throttle network to 500ms latency
   - Load playlists
   - Verify loading indicator shown
   - Verify data eventually loads

4. **API Timeout**:
   - Block Spotify API endpoints
   - Attempt any operation
   - Verify timeout error shown
   - Verify app remains responsive

5. **Recovery**:
   - After network restored
   - Attempt same operation
   - Verify success
   - Verify no cached error state

**Expected Behavior**:
- User-friendly error messages
- No application crash
- Loading indicators during network operations
- Automatic retry (where appropriate)
- Clear recovery path for user

**Log Checks**:
```
ERROR Request failed: network error
ERROR Network timeout after [duration]ms
WARN  Retrying request (attempt [n]/[max])
INFO  Network recovered
INFO  Request succeeded after retry
```

**Common Failure Points**:
- **Silent failure**: Error not shown to user
- **Infinite loading**: Loading indicator never clears
- **Crash**: Unhandled network exception
- **Stuck state**: UI frozen waiting for response
- **No retry**: User must restart app

---

### 5.2 API Rate Limit

**Objective**: Verify handling of Spotify API rate limits.

**Preconditions**:
- Valid authenticated session
- Ability to trigger rate limiting

**Test Procedure**:
1. **Trigger Rate Limit**:
   - Rapidly perform many operations
   - Search multiple times quickly
   - Load many playlists rapidly
   - Verify rate limit error handled

2. **Rate Limit During Playback**:
   - Start playback
   - Trigger rate limit with other operations
   - Verify playback continues (separate from API)

3. **Rate Limit Recovery**:
   - After hitting rate limit
   - Wait for limit reset
   - Verify operations succeed

4. **Queue Multiple Requests**:
   - Queue many client requests quickly
   - Verify channel doesn't overflow
   - Verify requests processed eventually

**Expected Behavior**:
- Rate limit error shown to user
- Requests queued, not dropped
- Automatic retry after delay
- No crash or frozen UI
- Operations eventually succeed

**Log Checks**:
```
WARN  Rate limited by Spotify API
ERROR API error: 429 Too Many Requests
INFO  Retrying after [seconds] seconds
INFO  Request succeeded after rate limit wait
```

**Common Failure Points**:
- **Requests dropped**: Channel overflow
- **No retry**: Operation fails permanently
- **Wrong delay**: Doesn't wait for reset
- **Crash**: Unhandled 429 response

**Special Note** (see handlers.rs):
- Channel has MAX_CHANNEL_SIZE of 1024
- Semaphore limits concurrent handlers to 16

---

### 5.3 Invalid Response

**Objective**: Verify handling of malformed or unexpected API responses.

**Preconditions**:
- Valid authenticated session
- Ability to intercept/modify responses (for testing)

**Test Procedure**:
1. **Missing Required Fields**:
   - Simulate API response with missing fields
   - Verify app doesn't crash
   - Verify error logged
   - Verify fallback/default shown

2. **Unexpected Data Types**:
   - Simulate response with wrong types
   - Verify parsing error handled
   - Verify app continues

3. **Empty Response**:
   - Simulate empty array/object
   - Verify UI shows "no results"
   - Verify no crash

4. **Null Values**:
   - Simulate response with null where value expected
   - Verify handled gracefully

5. **Large Response**:
   - Request very large playlist (>10,000 tracks)
   - Verify memory doesn't explode
   - Verify UI remains responsive

**Expected Behavior**:
- No application crash
- Error logged with details
- User-friendly error message
- Partial data shown if available
- Graceful degradation

**Log Checks**:
```
ERROR Failed to parse API response: [error]
WARN  Missing field [field_name] in response
ERROR Deserialization error: [details]
```

**Common Failure Points**:
- **Panic on unwrap**: Code assumes Some(value)
- **Index out of bounds**: Array access without check
- **Memory exhaustion**: Large response not chunked
- **Silent failure**: Error swallowed, no indication

---

### 5.4 Missing Device

**Objective**: Verify handling when no playback device is available.

**Preconditions**:
- Valid authenticated session
- No active playback devices

**Test Procedure**:
1. **Playback with No Device**:
   - Attempt to start playback
   - Verify device switcher opens
   - Verify "no devices" message if empty

2. **Device Disconnects During Playback**:
   - Start playback on device
   - Disconnect/turn off device
   - Verify playback stops
   - Verify error message shown

3. **Switch to Missing Device**:
   - Open device list
   - Select device that just went offline
   - Verify appropriate error

4. **Integrated Player (if streaming feature)**:
   - Test with integrated librespot player
   - Verify player starts
   - Simulate player crash
   - Verify recovery attempt

**Expected Behavior**:
- Device switcher shows when needed
- Clear "no devices" message
- Playback attempt fails gracefully
- User prompted to open Spotify on a device
- Toast notification explains issue

**Log Checks**:
```
WARN  No active device found
ERROR Playback failed: no active device
INFO  Opening device switcher
ERROR Device transfer failed: device not found
```

**Common Failure Points**:
- **Silent failure**: No indication of problem
- **Crash**: Unhandled None for device
- **Wrong device**: Stale device list
- **Infinite loading**: Waiting for device that doesn't exist

---

### 5.5 No Playback

**Objective**: Verify handling when there's no active playback context.

**Preconditions**:
- Valid authenticated session
- No active playback (nothing playing or paused)

**Test Procedure**:
1. **Commands with No Playback**:
   - Press pause/play
   - Press next/previous
   - Press shuffle/repeat
   - Verify appropriate feedback

2. **Volume with No Playback**:
   - Press volume up/down
   - Verify nothing happens (or error)

3. **Lyrics with No Playback**:
   - Press `g L` for lyrics
   - Verify "no playback" message

4. **Queue with No Playback**:
   - Press `z` for queue
   - Verify empty queue shown
   - Or verify "no queue" message

5. **Current Context with No Playback**:
   - Press `g space` for current context
   - Verify appropriate message

**Expected Behavior**:
- Commands handled gracefully
- Clear feedback to user
- No crash or error for user
- UI shows "no playback" state
- Playback bar shows empty state

**Log Checks**:
```
WARN  No active playback, ignoring command: [command]
INFO  Playback state: None
```

**Common Failure Points**:
- **Crash on unwrap**: Code assumes playback exists
- **UI freeze**: Waiting for playback that doesn't exist
- **Wrong state**: UI shows stale playback state
- **Error spam**: Repeated errors in log

---

## Test Environment Setup

### Prerequisites

1. **Spotify Account**:
   - Premium account recommended for full feature testing
   - Test account separate from personal account
   - Multiple test playlists, albums saved

2. **Environment Variables**:
   ```bash
   export RUST_LOG=debug,spotify_player_gui=trace
   export RUST_BACKTRACE=1
   ```

3. **Configuration**:
   - Copy `app.toml` template
   - Adjust timeouts for testing: `playback_refresh_duration_in_ms = 1000`
   - Enable logging: Set appropriate log level

4. **Test Data**:
   - Create test playlists with known content
   - Save test albums and artists
   - Prepare test search queries

5. **Network Tools**:
   - Network simulator (e.g., `tc`, Network Link Conditioner)
   - Proxy for intercepting requests (e.g., Charles, mitmproxy)

### Running Tests

```bash
# Run application with debug logging
RUST_LOG=debug cargo run

# Run with trace logging for specific modules
RUST_LOG=spotify_player_gui::client=trace cargo run

# Run unit tests
cargo test

# Run specific test
cargo test test_auth_config_default

# Run with all features
cargo test --all-features
```

### Test Checklist

Use this checklist when running integration tests:

- [ ] Authentication
  - [ ] Fresh start works
  - [ ] Cached credentials work
  - [ ] Token refresh works
  - [ ] Invalid token handled

- [ ] Playback
  - [ ] Play track works
  - [ ] Pause/resume works
  - [ ] Next/previous works
  - [ ] Seek works
  - [ ] Volume works
  - [ ] Shuffle/repeat works
  - [ ] Device switch works

- [ ] Library
  - [ ] Playlists load
  - [ ] Albums load
  - [ ] Artists load
  - [ ] Tracks load
  - [ ] Create playlist works
  - [ ] Add to playlist works
  - [ ] Remove from playlist works

- [ ] Search
  - [ ] Search tracks works
  - [ ] Search albums works
  - [ ] Search artists works
  - [ ] Search playlists works
  - [ ] Play from search works

- [ ] Error Handling
  - [ ] Network failure handled
  - [ ] Rate limit handled
  - [ ] Invalid response handled
  - [ ] Missing device handled
  - [ ] No playback handled

---

## Logging Guidelines

### Log Levels

- **ERROR**: Serious problems that prevent functionality
- **WARN**: Unexpected but handled situations
- **INFO**: Normal operation milestones
- **DEBUG**: Detailed operation information
- **TRACE**: Very detailed debugging info

### What to Check in Logs

1. **Authentication Logs**:
   - "Using cached credentials" or "Starting OAuth"
   - "Token refresh successful" or failures
   - Session establishment

2. **Playback Logs**:
   - Player requests being sent
   - Playback state changes
   - Device transfers
   - Error messages

3. **API Logs**:
   - Request URLs and methods
   - Response status codes
   - Rate limit warnings
   - Network errors

4. **State Logs**:
   - State updates
   - Cache operations
   - Background refresh triggers

### Log Analysis Commands

```bash
# Watch logs in real-time
RUST_LOG=debug cargo run 2>&1 | tee test.log

# Filter for errors
grep ERROR test.log

# Filter for authentication issues
grep -i "auth\|token\|credential" test.log

# Filter for API issues
grep -i "api\|request\|response" test.log

# Count errors by type
grep ERROR test.log | cut -d':' -f3- | sort | uniq -c
```

---

## Appendix A: Test User Account Setup

### Recommended Test Data

1. **Playlists**:
   - Small playlist (5-10 tracks)
   - Large playlist (100+ tracks)
   - Collaborative playlist
   - Folder with nested playlists

2. **Library**:
   - 50+ saved albums
   - 50+ followed artists
   - 500+ liked tracks

3. **For Search Testing**:
   - Tracks with special characters
   - Albums with same name, different artists
   - Artists with common names

### Test Scenarios Matrix

| Scenario | Auth Required | Premium Required | Network | Device |
|----------|--------------|------------------|---------|--------|
| Fresh Auth | No | No | Yes | No |
| Cached Auth | No | No | No | No |
| Token Refresh | No | No | Yes | No |
| Play Track | Yes | Yes* | Yes | Yes |
| Pause/Resume | Yes | Yes* | Yes | Yes |
| Search | Yes | No | Yes | No |
| Library | Yes | No | Yes | No |

*Some playback features require Premium

---

## Appendix B: Common Error Codes

| Code | Description | Recovery Action |
|------|-------------|-----------------|
| 401 | Unauthorized | Re-authenticate |
| 403 | Forbidden | Check permissions/scopes |
| 404 | Not Found | Resource doesn't exist |
| 429 | Rate Limited | Wait and retry |
| 500 | Server Error | Retry later |
| 502 | Bad Gateway | Retry later |
| 503 | Service Unavailable | Retry later |

---

## Appendix C: Performance Benchmarks

Expected performance metrics for integration tests:

| Operation | Expected Time | Timeout |
|-----------|---------------|---------|
| Authentication | 3-10s | 60s |
| Load Playlists | 1-3s | 30s |
| Load Albums | 1-3s | 30s |
| Search | 1-2s | 10s |
| Play Track | <1s | 5s |
| Pause/Resume | <0.5s | 5s |
| Next/Previous | <1s | 5s |
| Seek | <0.5s | 5s |
| Volume Change | <0.5s | 5s |

If operations exceed these times, investigate:
- Network connectivity
- API response times
- Application performance
- Device performance