use std::panic;

use futures::TryFutureExt;
use metrics_exporter_prometheus::PrometheusHandle;
use tokio::{sync::broadcast::Receiver, task::JoinSet};
use tracing::{error, info};

use crate::{
    config::ServiceConfig, metrics_exporters::MetricsExporterTaskBuilder, signal::ServiceSignal,
};

pub struct MetricsProcessor {
    config: ServiceConfig,
    metrics_handle: PrometheusHandle,
}

impl MetricsProcessor {
    pub fn from_config(config: ServiceConfig, metrics_handle: PrometheusHandle) -> Self {
        Self {
            config,
            metrics_handle,
        }
    }

    pub async fn run(self, mut signals: Receiver<ServiceSignal>) {
        let mut export_tasks = JoinSet::new();

        for metrics_exporter in self.config.metrics.clone() {
            export_tasks.spawn(
                metrics_exporter
                    .start_exporting(self.metrics_handle.clone())
                    .map_err(|err| {
                        error!(
                            message = "Metrics exporter task has failed with an error: {}",
                            err
                        );
                    }),
            );
        }

        loop {
            tokio::select! {
                Some(task) = export_tasks.join_next() => {
                    match task {
                        Ok(_t) => {
                            info!("Metrics exporter task completed successfully.");
                        }
                        Err(err) if err.is_panic() => panic::resume_unwind(err.into_panic()),
                        Err(err) => {
                            error!(message = "Metrics exporter task failed!", error = %err);
                        }
                    }
                },
                Ok(ServiceSignal::Shutdown | ServiceSignal::Quit) = signals.recv() => {
                    info!("Stopping metrics processor...");
                    break;
                }
            }
        }
    }
}
