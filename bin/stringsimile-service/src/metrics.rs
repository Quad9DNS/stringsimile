use std::{
    panic,
    time::{Duration, Instant},
};

use futures::TryFutureExt;
use metrics::{Gauge, Unit, describe_gauge, gauge};
use metrics_exporter_prometheus::PrometheusHandle;
use stringsimile_matcher::ruleset::StringGroup;
use tokio::{
    runtime::{Handle, RuntimeMetrics},
    sync::broadcast::Receiver,
    task::JoinSet,
};
use tracing::{error, info};

use crate::{
    config::ServiceConfig, metrics_exporters::MetricsExporterTaskBuilder,
    system_metrics::SystemMetrics,
};

pub struct MetricsProcessor {
    config: ServiceConfig,
    metrics_handle: PrometheusHandle,
    sytem_metrics: SystemMetrics,
    process_init_time: Instant,
}

struct TokioRuntimeMetrics {
    alive_tasks: Gauge,
    pending_tasks: Gauge,
    workers_count: Gauge,
}

impl TokioRuntimeMetrics {
    fn new() -> Self {
        describe_gauge!(
            "service_alive_tasks",
            Unit::Count,
            "number of active asynchronous tasks"
        );
        describe_gauge!(
            "service_pending_tasks",
            Unit::Count,
            "number asynchronous tasks waiting in the queue"
        );
        describe_gauge!(
            "service_workers_count",
            Unit::Count,
            "number of workers available to process tasks"
        );

        Self {
            alive_tasks: gauge!("service_alive_tasks"),
            pending_tasks: gauge!("service_pending_tasks"),
            workers_count: gauge!("service_workers_count"),
        }
    }

    fn emit_metrics(&self, runtime_metrics: &RuntimeMetrics) {
        self.alive_tasks
            .set(runtime_metrics.num_alive_tasks() as f64);
        self.pending_tasks
            .set(runtime_metrics.global_queue_depth() as f64);
        self.workers_count.set(runtime_metrics.num_workers() as f64);
    }
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

    pub async fn run(self, runtime_handle: Handle, mut shutdown: Receiver<()>) {
        let mut export_tasks = JoinSet::new();

        describe_gauge!(
            "process_uptime_secs",
            Unit::Seconds,
            "Duration this stringsimile instance has been running in seconds"
        );
        let uptime_metric = gauge!("process_uptime_secs");
        let init_time = self.process_init_time;
        let mut system_metrics = self.sytem_metrics;
        let tokio_metrics = TokioRuntimeMetrics::new();
        let runtime_metrics = runtime_handle.metrics();

        // Initial metrics emission
        uptime_metric.set(Instant::now().duration_since(init_time).as_secs_f64());
        system_metrics.emit_system_metrics();

        let upkeep_handle = self.metrics_handle.clone();
        runtime_handle.spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(5)).await;

                tokio_metrics.emit_metrics(&runtime_metrics);

                system_metrics.emit_system_metrics();
                uptime_metric.set(Instant::now().duration_since(init_time).as_secs_f64());

                upkeep_handle.run_upkeep();
            }
        });

        for metrics_exporter in self.config.metrics.exporters.clone() {
            export_tasks.spawn_on(
                metrics_exporter
                    .start_exporting(self.metrics_handle.clone())
                    .map_err(|err| {
                        error!("Metrics exporter task has failed with an error: {}", err);
                    }),
                &runtime_handle,
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

                _ = shutdown.recv() => {
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
        describe_gauge!(
            "string_groupnames",
            Unit::Count,
            "Number of string groups in current rule configuration"
        );
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
        describe_gauge!(
            "rule_sets",
            Unit::Count,
            "Number of rule sets in current rule configuration"
        );
        gauge!("rule_sets").set(rule_sets as f64);
        describe_gauge!(
            "rules",
            Unit::Count,
            "Number of rules in current rule configuration"
        );
        gauge!("rules").set(rules as f64);
    }
}
