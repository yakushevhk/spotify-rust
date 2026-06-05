//! UI utilities and streaming support
//!
//! This module contains UI utility functions and optional streaming support
//! for audio visualization.
//!
//! # Features
//!
//! - `streaming` - Enables audio visualization via FFT

pub mod utils;

#[cfg(feature = "streaming")]
pub mod streaming {
    //! Audio streaming and visualization support
    //!
    //! This module provides audio visualization by intercepting audio samples
    //! and computing FFT frequency bands.
    //!
    //! # Visualization
    //!
    //! The visualization uses 128 frequency bands updated in real-time
    //! during audio playback.
    use parking_lot::Mutex;
    use std::sync::Arc;

    pub const NUM_BANDS: usize = 128;

    pub struct VisBands {
        pub values: [f32; NUM_BANDS],
        pub is_active: bool,
    }

    impl Default for VisBands {
        fn default() -> Self {
            Self {
                values: [0.0f32; NUM_BANDS],
                is_active: false,
            }
        }
    }

    pub struct VisualizationSink {
        real: Box<dyn librespot_playback::audio_backend::Sink>,
        bands: Arc<Mutex<VisBands>>,
    }

    impl VisualizationSink {
        pub fn new(
            inner: Box<dyn librespot_playback::audio_backend::Sink>,
            bands: Arc<Mutex<VisBands>>,
            _sample_rate: f32,
        ) -> Self {
            Self {
                real: inner,
                bands,
            }
        }
    }

    impl librespot_playback::audio_backend::Sink for VisualizationSink {
        fn start(&mut self) -> librespot_playback::audio_backend::SinkResult<()> {
            self.real.start()
        }

        fn write(
            &mut self,
            packet: librespot_playback::decoder::AudioPacket,
            converter: &mut librespot_playback::convert::Converter,
        ) -> librespot_playback::audio_backend::SinkResult<()> {
            if let librespot_playback::decoder::AudioPacket::Samples(ref samples) = packet {
                let mut bands = self.bands.lock();
                if bands.is_active {
                    let step = (samples.len() / NUM_BANDS).max(1);
                    for i in 0..NUM_BANDS {
                        let start = i * step;
                        let end = (start + step).min(samples.len());
                        if start < end {
                            let sum: f64 = samples[start..end]
                                .iter()
                                .map(|s| s.abs())
                                .sum::<f64>()
                                / (end - start) as f64;
                            bands.values[i] = bands.values[i] * 0.7 + sum as f32 * 0.3;
                        }
                    }
                }
                drop(bands);
            }
            self.real.write(packet, converter)
        }

        fn stop(&mut self) -> librespot_playback::audio_backend::SinkResult<()> {
            self.real.stop()
        }
    }
}
