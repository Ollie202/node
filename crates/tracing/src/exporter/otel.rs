use opentelemetry_sdk::trace::SdkTracerProvider;

use super::InstallError;

pub(super) fn grpc_trace_provider() -> Result<SdkTracerProvider, InstallError> {
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .build()
        .map_err(InstallError::OtlpExporter)?;

    Ok(SdkTracerProvider::builder().with_batch_exporter(exporter).build())
}
