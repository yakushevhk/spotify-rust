use rand::seq::SliceRandom;
use std::time::Instant;

use super::model::{ContextId, PlayableId};

/// Result of advancing the queue by one track.
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum AdvanceResult {
    /// The next track is still within the current batch — librespot handles it.
    SameBatch,
    /// The current batch is exhausted; here is the next batch of track URIs to
    /// send via `StartPlayback`.
    NewBatch(Vec<PlayableId<'static>>),
    /// The queue has reached the end and `autoplay` is enabled — the caller
    /// should fetch radio tracks and append them before continuing.
    NeedsRadioTracks,
    /// The queue is fully exhausted and autoplay is not enabled.
    EndOfQueue,
}

/// Result of retreating the queue by one track.
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum RetreatResult {
    /// The previous track is still within the current batch.
    SameBatch,
    /// Need to load the previous batch to reach the previous track.
    PreviousBatch(Vec<PlayableId<'static>>),
    /// Already at the very beginning of the queue.
    BeginningOfQueue,
}

/// Shuffle mode for the custom queue.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ShuffleMode {
    #[default]
    Off,
    /// Standard shuffle — randomize the full track order.
    Shuffle,
    /// Smart shuffle — shuffle + interleave radio recommendations.
    /// Carries the radio tracks used for interleaving.
    SmartShuffle(Vec<PlayableId<'static>>),
}

/// App-managed playback queue that replaces spirc-managed queueing.
///
/// The custom queue stores the **full** ordered track list for a context
/// (playlist, album, etc.) and sends batches of URIs to Spotify. It only
/// intervenes at batch boundaries — within a batch, librespot handles
/// next/previous natively.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct CustomQueue {
    /// Original ordered track list (from the context, respecting client-side sort).
    original_tracks: Vec<PlayableId<'static>>,
    /// The effective play order.
    /// When shuffle is off this is a clone of `original_tracks`; when on it's a
    /// permutation. When smart-shuffle is on, extra recommendation track IDs are
    /// interleaved.
    play_order: Vec<PlayableId<'static>>,
    /// Current position within `play_order`.
    position: usize,
    /// Start index (inclusive) of the current batch within `play_order`.
    batch_start: usize,
    /// End index (exclusive) of the current batch within `play_order`.
    /// Tracks `play_order[batch_start..batch_end]` are the current batch.
    /// Normally `batch_end = min(batch_start + max_batch_size, play_order.len())`,
    /// but `truncate_batch_to_current()` can shrink it to `position + 1`.
    batch_end: usize,
    /// Maximum number of tracks per Spotify API batch (= `tracks_playback_limit`).
    max_batch_size: usize,
    /// Original context (for "playing from" display and radio seed).
    source_context: Option<ContextId>,
    /// Local repeat state mirroring the player's repeat.
    repeat: rspotify::model::RepeatState,
    /// Current shuffle mode (Off / Shuffle / `SmartShuffle`).
    shuffle_mode: ShuffleMode,
    /// Whether to fetch and append radio tracks when the queue is exhausted.
    /// Sourced from `DeviceConfig.autoplay`.
    autoplay: bool,
    /// Timestamp of last batch transition, used for consistency-check cooldown.
    last_batch_transition: Option<Instant>,
}

#[allow(dead_code)]
impl CustomQueue {
    /// Create a new custom queue.
    ///
    /// - `tracks`: the full ordered track list (respecting any client-side sort).
    /// - `start_position`: index of the track the user selected to play first.
    /// - `max_batch_size`: maximum tracks per Spotify batch (typically `tracks_playback_limit`).
    /// - `source_context`: the originating context (playlist, album, etc.) for
    ///   "playing from" display and radio seed.
    /// - `autoplay`: whether to fetch radio tracks when the queue is exhausted
    ///   (sourced from device config).
    pub fn new(
        tracks: Vec<PlayableId<'static>>,
        start_position: usize,
        max_batch_size: usize,
        source_context: Option<ContextId>,
        autoplay: bool,
    ) -> Self {
        if tracks.is_empty() {
            return Self {
                original_tracks: tracks,
                play_order: vec![],
                position: 0,
                batch_start: 0,
                batch_end: 0,
                max_batch_size,
                source_context,
                repeat: rspotify::model::RepeatState::Off,
                shuffle_mode: ShuffleMode::Off,
                autoplay,
                last_batch_transition: None,
            };
        }

        let start_position = start_position.min(tracks.len().saturating_sub(1));
        let play_order = tracks.clone();
        let batch_start = start_position;
        let batch_end = (batch_start + max_batch_size).min(play_order.len());

        Self {
            original_tracks: tracks,
            play_order,
            position: start_position,
            batch_start,
            batch_end,
            max_batch_size,
            source_context,
            repeat: rspotify::model::RepeatState::Off,
            shuffle_mode: ShuffleMode::Off,
            autoplay,
            last_batch_transition: None,
        }
    }

    // ── Accessors ──────────────────────────────────────────────────────

    /// The track URIs that make up the current batch sent to Spotify.
    pub fn current_batch(&self) -> Option<&[PlayableId<'static>]> {
        if self.batch_start >= self.play_order.len() || self.batch_start >= self.batch_end {
            return None;
        }
        Some(&self.play_order[self.batch_start..self.batch_end])
    }

    /// The currently playing track.
    pub fn current_track(&self) -> Option<&PlayableId<'static>> {
        self.play_order.get(self.position)
    }

    /// All tracks after the current position (for queue UI display).
    pub fn remaining_tracks(&self) -> &[PlayableId<'static>] {
        if self.position + 1 >= self.play_order.len() {
            &[]
        } else {
            &self.play_order[self.position + 1..]
        }
    }

    /// The source context this queue was built from.
    pub fn source_context(&self) -> Option<&ContextId> {
        self.source_context.as_ref()
    }

    /// Current shuffle mode.
    pub fn shuffle_mode(&self) -> &ShuffleMode {
        &self.shuffle_mode
    }

    /// Current repeat state.
    pub fn repeat(&self) -> rspotify::model::RepeatState {
        self.repeat
    }

    /// Current position within the play order.
    pub fn position(&self) -> usize {
        self.position
    }

    /// Batch start index.
    pub fn batch_start(&self) -> usize {
        self.batch_start
    }

    /// Batch end index (exclusive).
    pub fn batch_end(&self) -> usize {
        self.batch_end
    }

    /// Total number of tracks in the queue.
    pub fn len(&self) -> usize {
        self.play_order.len()
    }

    /// Whether the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.play_order.is_empty()
    }

    /// Timestamp of last batch transition (for consistency-check cooldown).
    pub fn last_batch_transition(&self) -> Option<Instant> {
        self.last_batch_transition
    }

    /// The expected next track in the play order (if any and within the batch).
    /// Used for queue consistency checking.
    pub fn expected_next_track(&self) -> Option<&PlayableId<'static>> {
        let next = self.position + 1;
        if next < self.batch_end {
            Some(&self.play_order[next])
        } else {
            None
        }
    }

    /// Whether the current track is the last in the current batch.
    pub fn is_at_batch_end(&self) -> bool {
        !self.play_order.is_empty() && self.position + 1 >= self.batch_end
    }

    /// Whether the current track is the first in the current batch.
    pub fn is_at_batch_start(&self) -> bool {
        !self.play_order.is_empty() && self.position == self.batch_start
    }

    // ── Mutations ──────────────────────────────────────────────────────

    /// Advance to the next track. Returns what action the caller should take.
    ///
    /// This is called from the `EndOfTrack` handler — it is the **sole**
    /// mechanism for advancing position.
    pub fn advance(&mut self) -> AdvanceResult {
        let next = self.position + 1;

        // RepeatState::Track — don't advance; librespot loops the track.
        if self.repeat == rspotify::model::RepeatState::Track {
            return AdvanceResult::SameBatch;
        }

        if next < self.batch_end {
            // Still within the current batch.
            self.position = next;
            AdvanceResult::SameBatch
        } else if next < self.play_order.len() {
            // Current batch exhausted but more tracks remain — start next batch.
            self.position = next;
            self.batch_start = next;
            self.batch_end = (self.batch_start + self.max_batch_size).min(self.play_order.len());
            self.mark_batch_transition();
            AdvanceResult::NewBatch(self.current_batch().expect("batch indices just set; batch must exist").to_vec())
        } else if self.repeat == rspotify::model::RepeatState::Context {
            // End of queue with repeat-context — wrap to beginning.
            self.position = 0;
            self.batch_start = 0;
            self.batch_end = self.max_batch_size.min(self.play_order.len());
            self.mark_batch_transition();
            AdvanceResult::NewBatch(self.current_batch().expect("batch indices just set; batch must exist").to_vec())
        } else if self.autoplay {
            // End of queue, no repeat — autoplay is enabled, ask caller to
            // fetch radio tracks and append them.
            AdvanceResult::NeedsRadioTracks
        } else {
            AdvanceResult::EndOfQueue
        }
    }

    /// Retreat to the previous track. Returns what action the caller should take.
    pub fn retreat(&mut self) -> RetreatResult {
        if self.play_order.is_empty() {
            return RetreatResult::BeginningOfQueue;
        }

        if self.position == 0 {
            if self.repeat == rspotify::model::RepeatState::Context {
                // Wrap to end of queue.
                self.position = self.play_order.len().saturating_sub(1);
                self.batch_end = self.play_order.len();
                self.batch_start = self.batch_end.saturating_sub(self.max_batch_size);
                self.mark_batch_transition();
                RetreatResult::PreviousBatch(self.current_batch().expect("batch indices just set; batch must exist").to_vec())
            } else {
                RetreatResult::BeginningOfQueue
            }
        } else {
            let prev = self.position - 1;
            if prev >= self.batch_start {
                self.position = prev;
                RetreatResult::SameBatch
            } else {
                // Need to load the previous batch.
                self.position = prev;
                self.batch_end = self.batch_start;
                self.batch_start = self.batch_end.saturating_sub(self.max_batch_size);
                self.mark_batch_transition();
                RetreatResult::PreviousBatch(self.current_batch().expect("batch indices just set; batch must exist").to_vec())
            }
        }
    }

    /// Truncate the current batch so that the current track is the last entry.
    ///
    /// After calling this, the next `EndOfTrack` event will trigger a batch
    /// transition with the new state (shuffle permutation, repeat mode, etc.)
    /// **without interrupting the currently playing song**.
    ///
    /// This is the key mechanism for non-interrupting shuffle/repeat changes.
    pub fn truncate_batch_to_current(&mut self) {
        self.batch_end = self.position + 1;
    }

    /// Update the repeat state.
    pub fn set_repeat(&mut self, repeat: rspotify::model::RepeatState) {
        self.repeat = repeat;
    }

    /// Change the shuffle mode.
    ///
    /// - `Off`: restore `play_order` to `original_tracks` order; find the
    ///   current track's position in the original order.
    /// - `Shuffle`: Fisher-Yates permutation of `play_order`, keeping the
    ///   current track at front (`position` 0).
    /// - `SmartShuffle(radio_tracks)`: shuffle + interleave the provided radio
    ///   recommendation tracks every N songs.
    ///
    /// After permuting, calls `truncate_batch_to_current()` so the change
    /// takes effect at the next batch boundary without restarting the current
    /// track.
    pub fn set_shuffle_mode(&mut self, mode: ShuffleMode) {
        if self.play_order.is_empty() {
            return;
        }

        let Some(current_track) = self.play_order.get(self.position).cloned() else {
            return;
        };

        match &mode {
            ShuffleMode::Off => {
                // Restore original order.
                self.play_order = self.original_tracks.clone();
                // Find where the current track sits in the original order.
                self.position = self
                    .play_order
                    .iter()
                    .position(|t| *t == current_track)
                    .expect("current track must exist in play_order");
                self.batch_start = 0;
            }
            ShuffleMode::Shuffle => {
                // Build a shuffled order with current track at front.
                let mut rng = rand::rng();
                let mut order: Vec<PlayableId<'static>> = self
                    .original_tracks
                    .iter()
                    .filter(|t| **t != current_track)
                    .cloned()
                    .collect();
                order.shuffle(&mut rng);
                order.insert(0, current_track);
                self.play_order = order;
                self.position = 0;
                self.batch_start = 0;
            }
            ShuffleMode::SmartShuffle(radio_tracks) => {
                // Shuffle first, then interleave radio tracks.
                let mut rng = rand::rng();
                let mut order: Vec<PlayableId<'static>> = self
                    .original_tracks
                    .iter()
                    .filter(|t| **t != current_track)
                    .cloned()
                    .collect();
                order.shuffle(&mut rng);
                order.insert(0, current_track);

                if radio_tracks.is_empty() {
                    self.play_order = order;
                } else {
                    // Interleave one radio track every 4 original tracks.
                    let mut interleaved = Vec::with_capacity(order.len() + radio_tracks.len());
                    let mut radio_iter = radio_tracks.iter();
                    for (i, track) in order.into_iter().enumerate() {
                        interleaved.push(track);
                        if i > 0 && (i + 1) % 4 == 0 {
                            if let Some(rt) = radio_iter.next() {
                                interleaved.push(rt.clone());
                            }
                        }
                    }
                    // Append any remaining radio tracks.
                    interleaved.extend(radio_iter.cloned());
                    self.play_order = interleaved;
                }
                self.position = 0;
                self.batch_start = 0;
            }
        }

        self.shuffle_mode = mode;
        // Let the current song finish, then the next batch uses the new order.
        self.truncate_batch_to_current();
    }

    /// Append radio recommendation tracks for autoplay continuation.
    pub fn append_radio_tracks(&mut self, tracks: Vec<PlayableId<'static>>) {
        self.play_order.extend(tracks.clone());
        self.original_tracks.extend(tracks);
    }

    /// Compute and load the next batch. Returns the batch URIs to send to
    /// Spotify, or `None` if the queue is exhausted.
    pub fn next_batch(&mut self) -> Option<Vec<PlayableId<'static>>> {
        if self.batch_end >= self.play_order.len() {
            return None;
        }
        self.batch_start = self.batch_end;
        self.batch_end = (self.batch_start + self.max_batch_size).min(self.play_order.len());
        self.mark_batch_transition();
        Some(self.current_batch().expect("batch indices just set; batch must exist").to_vec())
    }

    /// Record that a batch transition just occurred (for consistency-check
    /// cooldown).
    pub fn mark_batch_transition(&mut self) {
        self.last_batch_transition = Some(Instant::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_track_id(n: u32) -> PlayableId<'static> {
        PlayableId::Track(
            rspotify::model::TrackId::from_id(format!("track{n:032}"))
                .unwrap()
                .into_static(),
        )
    }

    fn make_tracks(count: u32) -> Vec<PlayableId<'static>> {
        (0..count).map(make_track_id).collect()
    }

    #[test]
    fn new_queue_basic_properties() {
        let tracks = make_tracks(10);
        let q = CustomQueue::new(tracks.clone(), 0, 5, None, false);

        assert_eq!(q.len(), 10);
        assert_eq!(q.position(), 0);
        assert_eq!(q.batch_start(), 0);
        assert_eq!(q.batch_end(), 5);
        assert_eq!(q.current_batch().unwrap().len(), 5);
        assert_eq!(*q.current_track().unwrap(), tracks[0]);
    }

    #[test]
    fn new_queue_start_position_mid() {
        let tracks = make_tracks(10);
        let q = CustomQueue::new(tracks.clone(), 3, 5, None, false);

        assert_eq!(q.position(), 3);
        assert_eq!(q.batch_start(), 3);
        assert_eq!(q.batch_end(), 8);
        assert_eq!(*q.current_track().unwrap(), tracks[3]);
    }

    #[test]
    fn new_queue_batch_end_clamped() {
        let tracks = make_tracks(3);
        let q = CustomQueue::new(tracks, 0, 10, None, false);

        assert_eq!(q.batch_end(), 3);
        assert_eq!(q.current_batch().unwrap().len(), 3);
    }

    #[test]
    fn advance_within_batch() {
        let tracks = make_tracks(10);
        let mut q = CustomQueue::new(tracks.clone(), 0, 5, None, false);

        assert_eq!(q.advance(), AdvanceResult::SameBatch);
        assert_eq!(q.position(), 1);
        assert_eq!(*q.current_track().unwrap(), tracks[1]);
    }

    #[test]
    fn advance_across_batch_boundary() {
        let tracks = make_tracks(10);
        let mut q = CustomQueue::new(tracks, 0, 5, None, false);

        // Advance to position 4 (last in batch [0..5)).
        for _ in 0..4 {
            assert_eq!(q.advance(), AdvanceResult::SameBatch);
        }
        assert_eq!(q.position(), 4);

        // Next advance should trigger a new batch.
        let result = q.advance();
        assert!(matches!(result, AdvanceResult::NewBatch(_)));
        assert_eq!(q.position(), 5);
        assert_eq!(q.batch_start(), 5);
        assert_eq!(q.batch_end(), 10);
    }

    #[test]
    fn advance_end_of_queue() {
        let tracks = make_tracks(3);
        let mut q = CustomQueue::new(tracks, 0, 10, None, false);

        for _ in 0..2 {
            q.advance();
        }
        assert_eq!(q.advance(), AdvanceResult::EndOfQueue);
    }

    #[test]
    fn advance_needs_radio_tracks() {
        let tracks = make_tracks(3);
        let mut q = CustomQueue::new(tracks, 0, 10, None, true);

        for _ in 0..2 {
            q.advance();
        }
        assert_eq!(q.advance(), AdvanceResult::NeedsRadioTracks);
    }

    #[test]
    fn advance_repeat_context_wraps() {
        let tracks = make_tracks(3);
        let mut q = CustomQueue::new(tracks.clone(), 0, 10, None, false);
        q.set_repeat(rspotify::model::RepeatState::Context);

        for _ in 0..2 {
            q.advance();
        }
        let result = q.advance();
        assert!(matches!(result, AdvanceResult::NewBatch(_)));
        assert_eq!(q.position(), 0);
        assert_eq!(*q.current_track().unwrap(), tracks[0]);
    }

    #[test]
    fn advance_repeat_track_stays() {
        let tracks = make_tracks(3);
        let mut q = CustomQueue::new(tracks.clone(), 0, 10, None, false);
        q.set_repeat(rspotify::model::RepeatState::Track);

        assert_eq!(q.advance(), AdvanceResult::SameBatch);
        assert_eq!(q.position(), 0); // Didn't move.
        assert_eq!(*q.current_track().unwrap(), tracks[0]);
    }

    #[test]
    fn retreat_within_batch() {
        let tracks = make_tracks(10);
        let mut q = CustomQueue::new(tracks.clone(), 0, 5, None, false);

        // Advance to position 2 (still within batch [0..5)).
        q.advance();
        q.advance();
        assert_eq!(q.position(), 2);

        assert_eq!(q.retreat(), RetreatResult::SameBatch);
        assert_eq!(q.position(), 1);
        assert_eq!(*q.current_track().unwrap(), tracks[1]);
    }

    #[test]
    fn retreat_at_beginning() {
        let tracks = make_tracks(10);
        let mut q = CustomQueue::new(tracks, 0, 5, None, false);

        assert_eq!(q.retreat(), RetreatResult::BeginningOfQueue);
        assert_eq!(q.position(), 0);
    }

    #[test]
    fn retreat_across_batch_boundary() {
        let tracks = make_tracks(10);
        let mut q = CustomQueue::new(tracks, 0, 5, None, false);

        // Advance into the second batch.
        for _ in 0..4 {
            q.advance();
        }
        q.advance(); // Triggers new batch at position 5.

        // Now retreat back across the boundary.
        let result = q.retreat();
        assert!(matches!(result, RetreatResult::PreviousBatch(_)));
        assert_eq!(q.position(), 4);
    }

    #[test]
    fn truncate_batch_to_current() {
        let tracks = make_tracks(10);
        let mut q = CustomQueue::new(tracks, 0, 5, None, false);

        q.advance(); // position = 1
        q.advance(); // position = 2
        q.truncate_batch_to_current();

        assert_eq!(q.batch_end(), 3); // position + 1
        assert!(q.is_at_batch_end());
    }

    #[test]
    fn remaining_tracks_correct() {
        let tracks = make_tracks(5);
        let q = CustomQueue::new(tracks.clone(), 0, 10, None, false);

        assert_eq!(q.remaining_tracks().len(), 4);
        assert_eq!(q.remaining_tracks()[0], tracks[1]);
    }

    #[test]
    fn remaining_tracks_at_end() {
        let tracks = make_tracks(3);
        let mut q = CustomQueue::new(tracks, 0, 10, None, false);
        q.advance();
        q.advance();

        assert!(q.remaining_tracks().is_empty());
    }

    #[test]
    fn expected_next_track_within_batch() {
        let tracks = make_tracks(10);
        let q = CustomQueue::new(tracks.clone(), 0, 5, None, false);

        assert_eq!(q.expected_next_track(), Some(&tracks[1]));
    }

    #[test]
    fn expected_next_track_at_batch_end() {
        let tracks = make_tracks(10);
        let mut q = CustomQueue::new(tracks, 0, 5, None, false);

        // Advance to position 4 (last in batch).
        for _ in 0..4 {
            q.advance();
        }
        assert_eq!(q.expected_next_track(), None); // Next is outside batch.
    }

    #[test]
    fn append_radio_tracks() {
        let tracks = make_tracks(3);
        let mut q = CustomQueue::new(tracks, 0, 10, None, false);

        let radio = make_tracks(5);
        q.append_radio_tracks(radio);

        assert_eq!(q.len(), 8);
    }

    #[test]
    fn set_shuffle_mode_shuffle() {
        let tracks = make_tracks(10);
        let mut q = CustomQueue::new(tracks.clone(), 3, 5, None, false);

        q.set_shuffle_mode(ShuffleMode::Shuffle);

        // Current track should be at front.
        assert_eq!(*q.current_track().unwrap(), tracks[3]);
        assert_eq!(q.position(), 0);
        // All original tracks should be present.
        assert_eq!(q.len(), 10);
        assert_eq!(*q.shuffle_mode(), ShuffleMode::Shuffle);
        // Batch should be truncated to current.
        assert_eq!(q.batch_end(), 1);
    }

    #[test]
    fn set_shuffle_mode_off_restores_order() {
        let tracks = make_tracks(10);
        let mut q = CustomQueue::new(tracks.clone(), 3, 5, None, false);

        q.set_shuffle_mode(ShuffleMode::Shuffle);
        q.set_shuffle_mode(ShuffleMode::Off);

        // Should be back in original order.
        assert_eq!(q.play_order, tracks);
        assert_eq!(*q.current_track().unwrap(), tracks[3]);
        assert_eq!(q.position(), 3);
    }

    // ============ SmartShuffle Tests ============

    #[test]
    fn set_shuffle_mode_smart_shuffle_interleaves_radio() {
        let tracks = make_tracks(8);
        let radio = make_tracks(3); // Radio tracks
        let mut q = CustomQueue::new(tracks.clone(), 0, 10, None, false);

        q.set_shuffle_mode(ShuffleMode::SmartShuffle(radio.clone()));

        // Current track should be at front.
        assert_eq!(*q.current_track().unwrap(), tracks[0]);
        assert_eq!(q.position(), 0);
        
        // Smart shuffle should have more tracks than original (original + radio)
        assert!(q.len() >= tracks.len());
        
        // Verify shuffle mode is set correctly
        assert!(matches!(q.shuffle_mode(), ShuffleMode::SmartShuffle(_)));
    }

    #[test]
    fn smart_shuffle_empty_radio_tracks() {
        let tracks = make_tracks(5);
        let radio: Vec<PlayableId<'static>> = vec![];
        let mut q = CustomQueue::new(tracks.clone(), 0, 10, None, false);

        q.set_shuffle_mode(ShuffleMode::SmartShuffle(radio));

        // Should still have all original tracks
        assert_eq!(q.len(), tracks.len());
        assert_eq!(*q.current_track().unwrap(), tracks[0]);
    }

    #[test]
    fn smart_shuffle_with_many_radio_tracks() {
        let tracks = make_tracks(4);
        let radio = make_tracks(20); // More radio tracks than original
        let mut q = CustomQueue::new(tracks.clone(), 0, 10, None, false);

        q.set_shuffle_mode(ShuffleMode::SmartShuffle(radio.clone()));

        // Should have original tracks + all radio tracks
        assert_eq!(q.len(), tracks.len() + radio.len());
    }

    // ============ Batch Truncation Tests ============

    #[test]
    fn truncate_batch_to_current_at_start() {
        let tracks = make_tracks(10);
        let mut q = CustomQueue::new(tracks, 0, 5, None, false);

        q.truncate_batch_to_current();

        // At position 0, batch_end should be 1
        assert_eq!(q.batch_end(), 1);
        assert!(q.is_at_batch_end());
    }

    #[test]
    fn truncate_batch_to_current_mid_batch() {
        let tracks = make_tracks(10);
        let mut q = CustomQueue::new(tracks, 0, 5, None, false);

        // Advance to position 2
        q.advance();
        q.advance();
        assert_eq!(q.position(), 2);

        q.truncate_batch_to_current();

        // Batch should end right after current position
        assert_eq!(q.batch_end(), 3);
        assert!(q.is_at_batch_end());
    }

    #[test]
    fn truncate_batch_affects_next_batch() {
        let tracks = make_tracks(10);
        let mut q = CustomQueue::new(tracks, 0, 5, None, false);

        // Advance to position 2 and truncate
        q.advance();
        q.advance();
        q.truncate_batch_to_current();

        // After truncation at position 2, batch_end = 3
        // next_batch should return tracks from position 3
        let next = q.next_batch();
        assert!(next.is_some());
        // Should have remaining tracks (10 - 3 = 7, but limited by max_batch_size = 5)
        assert_eq!(next.unwrap().len(), 5); // tracks 3,4,5,6,7
    }

    // ============ Edge Cases ============

    #[test]
    fn empty_queue_operations() {
        let tracks: Vec<PlayableId<'static>> = vec![];
        let q = CustomQueue::new(tracks.clone(), 0, 5, None, false);

        assert_eq!(q.len(), 0);
        assert!(q.is_empty());
        assert!(q.current_track().is_none());
        assert!(q.current_batch().is_none());
        assert!(q.remaining_tracks().is_empty());
    }

    #[test]
    fn single_item_queue() {
        let tracks = make_tracks(1);
        let mut q = CustomQueue::new(tracks.clone(), 0, 5, None, false);

        assert_eq!(q.len(), 1);
        assert!(!q.is_empty());
        assert_eq!(*q.current_track().unwrap(), tracks[0]);
        assert_eq!(q.batch_end(), 1);

        // Advance should go to end of queue
        assert_eq!(q.advance(), AdvanceResult::EndOfQueue);
    }

    #[test]
    fn single_item_queue_with_autoplay() {
        let tracks = make_tracks(1);
        let mut q = CustomQueue::new(tracks.clone(), 0, 5, None, true);

        assert_eq!(q.advance(), AdvanceResult::NeedsRadioTracks);
    }

    #[test]
    fn batch_size_larger_than_queue() {
        let tracks = make_tracks(3);
        let q = CustomQueue::new(tracks.clone(), 0, 10, None, false);

        // Batch end should be clamped to queue length
        assert_eq!(q.batch_end(), 3);
        assert_eq!(q.current_batch().unwrap().len(), 3);
    }

    #[test]
    fn start_position_at_end() {
        let tracks = make_tracks(5);
        let q = CustomQueue::new(tracks.clone(), 4, 10, None, false);

        assert_eq!(q.position(), 4);
        assert_eq!(*q.current_track().unwrap(), tracks[4]);
        assert_eq!(q.batch_start(), 4);
        assert_eq!(q.batch_end(), 5);
    }

    #[test]
    fn remaining_tracks_empty_at_end() {
        let tracks = make_tracks(3);
        let mut q = CustomQueue::new(tracks, 0, 10, None, false);

        // Advance to last track
        q.advance();
        q.advance();
        
        assert!(q.remaining_tracks().is_empty());
    }

    #[test]
    fn expected_next_track_at_queue_end() {
        let tracks = make_tracks(3);
        let mut q = CustomQueue::new(tracks, 0, 10, None, false);

        // Advance to last position
        q.advance();
        q.advance();
        assert_eq!(q.position(), 2);

        // No next track at end of queue
        assert!(q.expected_next_track().is_none());
    }

    #[test]
    fn is_at_batch_end_with_empty_queue() {
        let tracks: Vec<PlayableId<'static>> = vec![];
        let q = CustomQueue::new(tracks, 0, 5, None, false);

        // Should return false for empty queue
        assert!(!q.is_at_batch_end());
    }

    #[test]
    fn is_at_batch_start_with_empty_queue() {
        let tracks: Vec<PlayableId<'static>> = vec![];
        let q = CustomQueue::new(tracks, 0, 5, None, false);

        // Should return false for empty queue
        assert!(!q.is_at_batch_start());
    }

    #[test]
    fn retreat_at_beginning_no_repeat() {
        let tracks = make_tracks(5);
        let mut q = CustomQueue::new(tracks, 0, 10, None, false);

        // Without repeat, should return BeginningOfQueue
        assert_eq!(q.retreat(), RetreatResult::BeginningOfQueue);
        assert_eq!(q.position(), 0);
    }

    #[test]
    fn advance_repeat_context_single_item() {
        let tracks = make_tracks(1);
        let mut q = CustomQueue::new(tracks.clone(), 0, 10, None, false);
        q.set_repeat(rspotify::model::RepeatState::Context);

        // With single item and repeat context, should wrap
        let result = q.advance();
        assert!(matches!(result, AdvanceResult::NewBatch(_)));
        assert_eq!(q.position(), 0);
    }

    #[test]
    fn batch_transition_timestamp() {
        let tracks = make_tracks(10);
        let mut q = CustomQueue::new(tracks, 0, 5, None, false);

        // Initially no transition
        assert!(q.last_batch_transition().is_none());

        // Advance to trigger batch transition
        for _ in 0..4 {
            q.advance();
        }
        let _ = q.advance(); // This triggers NewBatch

        // Should have recorded transition time
        assert!(q.last_batch_transition().is_some());
    }

    #[test]
    fn next_batch_at_end_returns_none() {
        let tracks = make_tracks(5);
        let mut q = CustomQueue::new(tracks, 0, 10, None, false);

        // At end of queue, next_batch should return None
        assert!(q.next_batch().is_none());
    }

    #[test]
    fn next_batch_returns_tracks() {
        let tracks = make_tracks(10);
        let mut q = CustomQueue::new(tracks, 0, 5, None, false);

        // First batch is [0..5), next should be [5..10)
        let next = q.next_batch();
        assert!(next.is_some());
        assert_eq!(next.unwrap().len(), 5);
        assert_eq!(q.batch_start(), 5);
        assert_eq!(q.batch_end(), 10);
    }

    #[test]
    fn append_radio_tracks_empty_queue() {
        let tracks: Vec<PlayableId<'static>> = vec![];
        let mut q = CustomQueue::new(tracks, 0, 5, None, false);

        let radio = make_tracks(3);
        q.append_radio_tracks(radio);

        assert_eq!(q.len(), 3);
    }

    #[test]
    fn mark_batch_transition_updates_timestamp() {
        let tracks = make_tracks(5);
        let mut q = CustomQueue::new(tracks, 0, 10, None, false);

        let before = q.last_batch_transition();
        q.mark_batch_transition();
        let after = q.last_batch_transition();

        assert!(before.is_none());
        assert!(after.is_some());
    }

    #[test]
    fn source_context_accessor() {
        use crate::state::model::TracksId;
        
        let tracks = make_tracks(5);
        let context = ContextId::Tracks(TracksId {
            uri: "test:uri".to_string(),
            kind: "test".to_string(),
        });
        
        let q = CustomQueue::new(tracks, 0, 5, Some(context.clone()), false);
        
        assert!(q.source_context().is_some());
        assert_eq!(q.source_context().unwrap().uri(), "test:uri");
    }

    #[test]
    fn shuffle_mode_accessor() {
        let tracks = make_tracks(5);
        let mut q = CustomQueue::new(tracks, 0, 5, None, false);

        assert_eq!(*q.shuffle_mode(), ShuffleMode::Off);

        q.set_shuffle_mode(ShuffleMode::Shuffle);
        assert_eq!(*q.shuffle_mode(), ShuffleMode::Shuffle);
    }

    #[test]
    fn repeat_accessor() {
        let tracks = make_tracks(5);
        let mut q = CustomQueue::new(tracks, 0, 5, None, false);

        assert_eq!(q.repeat(), rspotify::model::RepeatState::Off);

        q.set_repeat(rspotify::model::RepeatState::Track);
        assert_eq!(q.repeat(), rspotify::model::RepeatState::Track);
    }
}
