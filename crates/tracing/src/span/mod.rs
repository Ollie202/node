use opentelemetry::Key;

use crate::{OpenTelemetryField, OpenTelemetryObject, OpenTelemetryObjectRecorder};

/// Extension methods for recording OpenTelemetry fields and objects onto tracing spans.
pub trait OpenTelemetrySpanExt {
    /// Records `field` using its default key.
    fn record_field<F>(&self, field: &F)
    where
        F: OpenTelemetryField + ?Sized;

    /// Records `field` using `key` instead of its default key.
    fn record_field_as<F>(&self, field: &F, key: impl Into<Key>)
    where
        F: OpenTelemetryField + ?Sized;

    /// Records `object` using its default key prefix.
    fn record_object<O>(&self, object: &O)
    where
        O: OpenTelemetryObject + ?Sized;

    /// Records `object` using `key_prefix` instead of its default key prefix.
    fn record_object_as<O>(&self, object: &O, key_prefix: &str)
    where
        O: OpenTelemetryObject + ?Sized;
}

impl OpenTelemetrySpanExt for tracing::Span {
    fn record_field<F>(&self, field: &F)
    where
        F: OpenTelemetryField + ?Sized,
    {
        self.record_field_as(field, F::DEFAULT_KEY);
    }

    fn record_field_as<F>(&self, field: &F, key: impl Into<Key>)
    where
        F: OpenTelemetryField + ?Sized,
    {
        tracing_opentelemetry::OpenTelemetrySpanExt::set_attribute(
            self,
            key,
            field.to_otel_value(),
        );
    }

    fn record_object<O>(&self, object: &O)
    where
        O: OpenTelemetryObject + ?Sized,
    {
        self.record_object_as(object, O::DEFAULT_KEY_PREFIX);
    }

    fn record_object_as<O>(&self, object: &O, key_prefix: &str)
    where
        O: OpenTelemetryObject + ?Sized,
    {
        let mut recorder = OpenTelemetryObjectRecorder::new(self, key_prefix);
        object.record_otel_fields(&mut recorder);
    }
}

#[cfg(test)]
mod tests {
    use super::OpenTelemetrySpanExt;
    use crate::test_utils::{assert_attribute, exported_span};
    use crate::{OpenTelemetryField, OpenTelemetryObject, OpenTelemetryObjectRecorder};

    struct TestField;

    impl OpenTelemetryField for TestField {
        const DEFAULT_KEY: &'static str = "test.field";
        const DEFAULT_KEY_SUFFIX: &'static str = "field";

        fn to_otel_value(&self) -> opentelemetry::Value {
            "value".into()
        }
    }

    struct NestedObject;

    impl OpenTelemetryObject for NestedObject {
        const DEFAULT_KEY_PREFIX: &'static str = "nested";

        fn record_otel_fields(&self, recorder: &mut OpenTelemetryObjectRecorder<'_>) {
            recorder.record_field(&TestField);
        }
    }

    struct TestObject;

    impl OpenTelemetryObject for TestObject {
        const DEFAULT_KEY_PREFIX: &'static str = "test";

        fn record_otel_fields(&self, recorder: &mut OpenTelemetryObjectRecorder<'_>) {
            recorder.record_field(&TestField);
            recorder.record_object(&NestedObject);
        }
    }

    #[test]
    fn span_extension_records_fields_with_default_and_override_keys() {
        let span = exported_span(|span| {
            span.record_field(&TestField);
            span.record_field_as(&TestField, "custom.field");
        });

        assert_attribute(&span, "test.field", "value");
        assert_attribute(&span, "custom.field", "value");
    }

    #[test]
    fn span_extension_records_objects_with_default_and_override_prefixes() {
        let span = exported_span(|span| {
            span.record_object(&TestObject);
            span.record_object_as(&TestObject, "custom");
        });

        assert_attribute(&span, "test.field", "value");
        assert_attribute(&span, "test.nested.field", "value");
        assert_attribute(&span, "custom.field", "value");
        assert_attribute(&span, "custom.nested.field", "value");
    }
}
