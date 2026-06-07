use disyn_core::ports::{SpanEvent, TelemetrySink};

pub struct TracingSink;

impl Default for TracingSink {
    fn default() -> Self {
        Self
    }
}

impl TracingSink {
    pub fn init() -> Self {
        Self
    }
}

impl TelemetrySink for TracingSink {
    fn emit(&self, event: &SpanEvent) {
        tracing::info!(
            trace_id = %event.trace_id,
            kind = ?event.kind,
            duration_ms = event.duration_ms,
            status = ?event.status,
            "span"
        );
    }
}
