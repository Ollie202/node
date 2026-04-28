use std::error::Error;
use std::fmt::Write;

pub(super) fn error_report<E>(error: &E) -> String
where
    E: Error + ?Sized,
{
    let mut report = error.to_string();
    let mut source = error.source();

    while let Some(error) = source {
        write!(report, "\ncaused by: {error}").expect("writing to String should not fail");
        source = error.source();
    }

    report
}
