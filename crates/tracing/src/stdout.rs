use std::fmt;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use opentelemetry::KeyValue;
use opentelemetry::trace::{Event, Status};
use opentelemetry_sdk::error::{OTelSdkError, OTelSdkResult};
use opentelemetry_sdk::trace::{SpanData, SpanExporter};

/// Exports user-facing spans and events as compact stdout log lines.
///
/// This is intentionally implemented as an OpenTelemetry span exporter instead of a `tracing`
/// formatting layer. The tradeoff is that span logs are emitted when spans close, which is
/// acceptable for the local operator view this exporter serves.
pub(crate) struct UserFacingStdoutExporter {
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
}

impl UserFacingStdoutExporter {
    /// Creates an exporter that writes to process stdout.
    pub(crate) fn stdout() -> Self {
        Self::new(std::io::stdout())
    }

    /// Creates an exporter that writes to `writer`.
    pub(crate) fn new(writer: impl Write + Send + 'static) -> Self {
        Self {
            writer: Arc::new(Mutex::new(Box::new(writer))),
        }
    }
}

impl fmt::Debug for UserFacingStdoutExporter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UserFacingStdoutExporter").finish_non_exhaustive()
    }
}

impl SpanExporter for UserFacingStdoutExporter {
    async fn export(&self, batch: Vec<SpanData>) -> OTelSdkResult {
        let mut lines = Vec::new();

        for span in batch {
            if is_user_facing(&span.attributes) {
                lines.push(format_span(&span));
            }

            for event in span.events.events.iter().filter(|event| is_user_facing(&event.attributes))
            {
                lines.push(format_event(event));
            }
        }

        if lines.is_empty() {
            return Ok(());
        }

        let mut writer = self.writer.lock().map_err(|_| {
            OTelSdkError::InternalFailure("user-facing stdout exporter lock poisoned".to_owned())
        })?;
        for line in lines {
            writeln!(writer, "{line}").map_err(|err| {
                OTelSdkError::InternalFailure(format!(
                    "failed to write user-facing stdout log: {err}"
                ))
            })?;
        }

        Ok(())
    }
}

fn format_span(span: &SpanData) -> String {
    let attributes = format_attributes(&span.attributes, &[]);
    let duration = span.end_time.duration_since(span.start_time).ok();

    match &span.status {
        Status::Error { description } => {
            format!(
                "ERROR {} failed: {}{}{}",
                span.name,
                description,
                format_duration_suffix(duration),
                attributes
            )
        },
        Status::Ok | Status::Unset => {
            format!(
                "INFO {} completed{}{}",
                span.name,
                format_duration_suffix(duration),
                attributes
            )
        },
    }
}

fn format_event(event: &Event) -> String {
    let level = attribute_value(&event.attributes, "level").unwrap_or_else(|| "INFO".to_owned());
    let attributes = format_attributes(&event.attributes, &["level", "target"]);

    format!("{level} {}{}", event.name, attributes)
}

fn is_user_facing(attributes: &[KeyValue]) -> bool {
    attributes.iter().any(|attribute| {
        attribute.key.as_str() == crate::user::ATTRIBUTE_KEY
            && matches!(attribute.value, opentelemetry::Value::Bool(true))
    })
}

fn attribute_value(attributes: &[KeyValue], key: &str) -> Option<String> {
    attributes.iter().find_map(|attribute| {
        (attribute.key.as_str() == key).then(|| attribute.value.as_str().into_owned())
    })
}

fn format_attributes(attributes: &[KeyValue], hidden: &[&str]) -> String {
    let mut rendered = String::new();

    for attribute in attributes {
        let key = attribute.key.as_str();
        if hidden.contains(&key) {
            continue;
        }
        let Some(key) = key.strip_prefix(crate::user::FIELD_PREFIX) else {
            continue;
        };

        rendered.push(' ');
        rendered.push_str(key);
        rendered.push('=');
        rendered.push_str(&attribute.value.as_str());
    }

    rendered
}

fn format_duration_suffix(duration: Option<Duration>) -> String {
    let Some(duration) = duration else {
        return String::new();
    };

    format!(" duration_ms={}", duration.as_millis())
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::sync::{Arc, Mutex};

    use opentelemetry::trace::TracerProvider as _;
    use opentelemetry_sdk::trace::SdkTracerProvider;
    use tracing_subscriber::prelude::*;

    use super::UserFacingStdoutExporter;
    use crate::OpenTelemetryField;

    #[derive(Clone, Default)]
    struct TestWriter {
        output: Arc<Mutex<Vec<u8>>>,
    }

    impl io::Write for TestWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.output.lock().expect("test writer lock poisoned").extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    struct TestField;

    impl OpenTelemetryField for TestField {
        const DEFAULT_KEY: &'static str = "test.field";
        const DEFAULT_KEY_SUFFIX: &'static str = "field";

        fn to_otel_value(&self) -> opentelemetry::Value {
            "value".into()
        }
    }

    #[test]
    fn exporter_logs_user_spans_and_events() {
        let writer = TestWriter::default();
        let output = writer.output.clone();
        let exporter = UserFacingStdoutExporter::new(writer);
        let provider = SdkTracerProvider::builder().with_simple_exporter(exporter).build();
        let tracer = provider.tracer("miden-node-tracing-stdout-test");
        let subscriber =
            tracing_subscriber::registry().with(tracing_opentelemetry::layer().with_tracer(tracer));

        tracing::subscriber::with_default(subscriber, || {
            let span = crate::info_span!(rpc, "sync block", user);
            span.record_field_as(&TestField, "trace.only");
            span.record_user_field(&TestField);
            let _guard = span.entered();

            let event = crate::info!(
                rpc,
                "block accepted",
                user,
                justification = "tests user-facing event stdout output"
            );
            event.record_field_as(&TestField, "trace.event");
            event.record_user_field_as(&TestField, "event.field");
            event.emit();
        });

        drop(provider);
        let output = output.lock().expect("test writer lock poisoned");
        let output = String::from_utf8(output.clone()).expect("stdout output should be utf8");

        assert!(output.contains("INFO sync block completed"));
        assert!(output.contains("test.field=value"));
        assert!(output.contains("INFO block accepted"));
        assert!(output.contains("event.field=value"));
        assert!(!output.contains("trace.only"));
        assert!(!output.contains("trace.event"));
    }

    #[test]
    fn exporter_ignores_non_user_spans_and_events() {
        let writer = TestWriter::default();
        let output = writer.output.clone();
        let exporter = UserFacingStdoutExporter::new(writer);
        let provider = SdkTracerProvider::builder().with_simple_exporter(exporter).build();
        let tracer = provider.tracer("miden-node-tracing-stdout-test");
        let subscriber =
            tracing_subscriber::registry().with(tracing_opentelemetry::layer().with_tracer(tracer));

        tracing::subscriber::with_default(subscriber, || {
            let span = crate::info_span!(rpc, "internal span");
            let _guard = span.entered();

            let event = crate::info!(
                rpc,
                "internal event",
                justification = "tests non-user event stdout suppression"
            );
            event.emit();
        });

        drop(provider);
        let output = output.lock().expect("test writer lock poisoned");

        assert!(output.is_empty());
    }
}
