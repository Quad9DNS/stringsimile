use axum::{Router, routing::get};
use std::{future::ready, net::SocketAddr};
use tracing::debug;

use metrics_exporter_prometheus::PrometheusHandle;
use serde::{Deserialize, Serialize};

use super::MetricsExporterTaskBuilder;

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScrapeExporterConfig {
    pub addr: SocketAddr,
}

pub struct ScrapeMetricsExporter {
    config: ScrapeExporterConfig,
}

impl ScrapeMetricsExporter {
    pub fn new(config: ScrapeExporterConfig) -> Self {
        Self { config }
    }
}

impl MetricsExporterTaskBuilder for ScrapeMetricsExporter {
    async fn start_exporting(self, handle: PrometheusHandle) -> crate::Result<()> {
        let app = Router::new().route("/metrics", get(move || ready(handle.render())));

        let listener = tokio::net::TcpListener::bind(self.config.addr).await?;
        debug!(
            "Prometheus metrics listening on {}",
            listener.local_addr().unwrap()
        );
        axum::serve(listener, app).await?;
        Ok(())
    }
}
