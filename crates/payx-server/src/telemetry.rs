use anyhow::Result;
use opentelemetry::trace::TracerProvider;
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::runtime::Tokio;
use opentelemetry_sdk::Resource;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::config::Config;

pub fn init(config: &Config) -> Result<()> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let fmt_layer = tracing_subscriber::fmt::layer().json();

    let registry = tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer);

    if let Some(endpoint) = &config.otlp_endpoint {
        let resource = Resource::new(vec![KeyValue::new("service.name", "payx")]);

        let provider = opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(
                opentelemetry_otlp::new_exporter()
                    .tonic()
                    .with_endpoint(endpoint),
            )
            .with_trace_config(opentelemetry_sdk::trace::Config::default().with_resource(resource))
            .install_batch(Tokio)?;

        let tracer = provider.tracer("payx");
        let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);
        registry.with(telemetry).init();
    } else {
        registry.init();
    }

    Ok(())
}
