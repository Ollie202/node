use opentelemetry::Key;

// Keep the macro argument transport-agnostic. `user` means "safe to show to an operator"; the
// concrete exporter decides whether that becomes stdout, a UI notification, or something else.
pub(crate) const ATTRIBUTE_KEY: &str = "miden.user";

pub(crate) const FIELD_PREFIX: &str = "miden.user.";

pub(crate) fn field_key(key: impl Into<Key>) -> Key {
    let key = key.into();

    Key::new(format!("{FIELD_PREFIX}{}", key.as_str()))
}
