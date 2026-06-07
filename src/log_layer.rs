use std::{collections::VecDeque, sync::Arc};

use parking_lot::Mutex;
use tracing::Subscriber;
use tracing_subscriber::Layer;

pub struct BufferLayer {
    buffer: Arc<Mutex<VecDeque<String>>>,
    max_lines: usize,
    max_level: tracing::Level,
}

impl BufferLayer {
    pub fn new(buffer: Arc<Mutex<VecDeque<String>>>, max_lines: usize) -> Self {
        Self {
            buffer,
            max_lines,
            max_level: tracing::Level::INFO,
        }
    }
}

impl<S: Subscriber> Layer<S> for BufferLayer {
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        // M6: skip events below the configured level to avoid buffer flood
        if *event.metadata().level() < self.max_level {
            return;
        }

        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);

        let level = event.metadata().level();
        let target = event.metadata().target();
        // L_M1: truncate module path to last segment for readability
        let short_target = target.rsplit("::").next().unwrap_or(target);
        let line = format!(
            "{} {:>5} {}: {}",
            chrono::Local::now().format("%H:%M:%S"),
            level,
            short_target,
            visitor.message
        );

        let mut buf = self.buffer.lock();
        buf.push_back(line);
        while buf.len() > self.max_lines {
            buf.pop_front();
        }
    }
}

#[derive(Default)]
struct MessageVisitor {
    message: String,
}

impl MessageVisitor {
    fn append_field(&mut self, name: &str, value: impl std::fmt::Display) {
        if !self.message.is_empty() {
            self.message.push(' ');
        }
        self.message.push_str(&format!("{}={}", name, value));
    }
}

impl tracing::field::Visit for MessageVisitor {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        } else {
            self.append_field(field.name(), value);
        }
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn core::fmt::Debug) {
        if field.name() == "message" {
            // M5: strip outer Debug quotes to get Display-like formatting for strings
            let debug_str = format!("{value:?}");
            self.message = debug_str
                .strip_prefix('"')
                .and_then(|s| s.strip_suffix('"'))
                .unwrap_or(&debug_str)
                .to_string();
        } else {
            // Capture other fields as key=value pairs
            self.append_field(field.name(), format_args!("{value:?}"));
        }
    }
}
