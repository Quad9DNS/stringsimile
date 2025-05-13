use file::FileMetricsExporter;
use metrics_exporter_prometheus::PrometheusHandle;
use stdout::StdoutMetricsExporter;

mod bufwriter;
mod file;
mod stdout;

pub use file::FileExporterConfig;
pub use stdout::StdoutExporterConfig;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum MetricsExporter {
    Stdout(StdoutExporterConfig),
    File(FileExporterConfig),
}

impl MetricsExporterTaskBuilder for MetricsExporter {
    async fn start_exporting(self, handle: PrometheusHandle) -> crate::Result<()> {
        match self {
            MetricsExporter::Stdout(config) => {
                StdoutMetricsExporter::new(config)
                    .start_exporting(handle)
                    .await
            }
            MetricsExporter::File(config) => {
                FileMetricsExporter::new(config)
                    .start_exporting(handle)
                    .await
            }
        }
    }
}

impl MetricsExporterBuilder for MetricsExporter {
    fn name(&self) -> String {
        match self {
            MetricsExporter::Stdout(_config) => "stdout".to_string(),
            MetricsExporter::File(config) => {
                let file_name = config.file_path.to_string_lossy();
                format!("file{file_name}")
            }
        }
    }
}

pub(crate) trait MetricsExporterTaskBuilder {
    async fn start_exporting(self, handle: PrometheusHandle) -> crate::Result<()>;
}

pub(crate) trait MetricsExporterBuilder: MetricsExporterTaskBuilder {
    #[allow(unused)]
    fn name(&self) -> String;
}
