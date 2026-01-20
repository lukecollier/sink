use tokio::sync::mpsc::UnboundedReceiver;
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::registry::LookupSpan;

pub struct CaptureLayer {
    tx: tokio::sync::mpsc::UnboundedSender<String>,
}

impl CaptureLayer {
    pub fn new(tx: tokio::sync::mpsc::UnboundedSender<String>) -> Self {
        Self { tx }
    }
}

impl<S> Layer<S> for CaptureLayer
where
    S: tracing::Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: Context<'_, S>,
    ) {
        use std::fmt::Write;
        let mut buf = String::new();
        let metadata = event.metadata();

        // Format: [LEVEL] message (uppercase level)
        let level = metadata.level();
        let _ = write!(buf, "[{}] ", level.as_str().to_uppercase());

        // Use a visitor to extract the message
        struct MessageVisitor<'a>(&'a mut String);

        impl<'a> tracing::field::Visit for MessageVisitor<'a> {
            fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
                if field.name() == "message" {
                    let _ = write!(self.0, "{:?}", value);
                }
            }
        }

        let mut visitor = MessageVisitor(&mut buf);
        event.record(&mut visitor);

        let _ = self.tx.send(buf);
    }
}

/// Create a capture layer and receiver for tracing events.
/// The caller is responsible for initializing the global subscriber with this layer.
pub fn capture_layer() -> (CaptureLayer, UnboundedReceiver<String>) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let layer = CaptureLayer::new(tx);
    (layer, rx)
}
