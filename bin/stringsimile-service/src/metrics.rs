use std::{
    panic,
    time::{Duration, Instant},
};

use futures::TryFutureExt;
use metrics::gauge;
use metrics_exporter_prometheus::PrometheusHandle;
use stringsimile_matcher::ruleset::StringGroup;
use tokio::{sync::broadcast::Receiver, task::JoinSet};
use tracing::{error, info};

use crate::{
    config::ServiceConfig, metrics_exporters::MetricsExporterTaskBuilder, signal::ServiceSignal,
    system_metrics::SystemMetrics,
};

pub struct MetricsProcessor {
    config: ServiceConfig,
    metrics_handle: PrometheusHandle,
    sytem_metrics: SystemMetrics,
    process_init_time: Instant,
}

impl MetricsProcessor {
    pub fn from_config(
        config: ServiceConfig,
        metrics_handle: PrometheusHandle,
        process_init_time: Instant,
    ) -> Self {
        Self {
            config,
            metrics_handle,
            sytem_metrics: SystemMetrics::new(),
            process_init_time,
        }
    }

    pub async fn run(self, mut signals: Receiver<ServiceSignal>) {
        let mut export_tasks = JoinSet::new();

        let uptime_metric = gauge!("process_uptime_secs");
        let init_time = self.process_init_time;
        let mut system_metrics = self.sytem_metrics;

        // Initial metrics emission
        uptime_metric.set(Instant::now().duration_since(init_time).as_secs_f64());
        system_metrics.emit_system_metrics();

        let upkeep_handle = self.metrics_handle.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(5)).await;
                system_metrics.emit_system_metrics();
                uptime_metric.set(Instant::now().duration_since(init_time).as_secs_f64());
                upkeep_handle.run_upkeep();
            }
        });

        for metrics_exporter in self.config.metrics.exporters.clone() {
            export_tasks.spawn(
                metrics_exporter
                    .start_exporting(self.metrics_handle.clone())
                    .map_err(|err| {
                        error!("Metrics exporter task has failed with an error: {}", err);
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

pub trait ExportMetrics {
    fn export_metrics(&self);
}

impl ExportMetrics for Vec<StringGroup> {
    fn export_metrics(&self) {
        gauge!("string_groupnames").set(self.len() as f64);
        let (rule_sets, rules) = self
            .iter()
            .map(|g| {
                (
                    g.rule_sets.len(),
                    g.rule_sets.iter().map(|s| s.rules.len()).sum::<usize>(),
                )
            })
            .reduce(|(sets, rules), g| (sets + g.0, rules + g.1))
            .unwrap_or((0, 0));
        gauge!("rule_sets").set(rule_sets as f64);
        gauge!("rules").set(rules as f64);
    }
}
