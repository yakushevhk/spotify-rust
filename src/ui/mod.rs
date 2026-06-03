pub mod utils;
pub mod single_line_input;

pub mod streaming {
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

    pub struct VisualizationSink;

    impl VisualizationSink {
        pub fn new(
            _inner: Box<dyn librespot_playback::audio_backend::Sink>,
            _bands: Arc<Mutex<VisBands>>,
            _sample_rate: f32,
        ) -> Self {
            Self
        }
    }

    impl librespot_playback::audio_backend::Sink for VisualizationSink {
        fn start(&mut self) -> librespot_playback::audio_backend::SinkResult<()> {
            Ok(())
        }

        fn write(
            &mut self,
            _packet: librespot_playback::decoder::AudioPacket,
            _converter: &mut librespot_playback::convert::Converter,
        ) -> librespot_playback::audio_backend::SinkResult<()> {
            Ok(())
        }

        fn stop(&mut self) -> librespot_playback::audio_backend::SinkResult<()> {
            Ok(())
        }
    }
}
