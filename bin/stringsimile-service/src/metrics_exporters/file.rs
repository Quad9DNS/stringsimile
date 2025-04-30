use std::{path::PathBuf, time::Duration};
use tokio_stream::{StreamExt, wrappers::IntervalStream};
use tracing::error;

use metrics_exporter_prometheus::PrometheusHandle;
use serde::{Deserialize, Serialize};
use tokio::{fs::File, io::BufWriter, time::interval};

use super::{MetricsExporterTaskBuilder, bufwriter::BufWriterMetricsExporter};

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileExporterConfig {
    pub file_path: PathBuf,
    #[serde(default = "default_file_export_interval")]
    pub export_interval_secs: u64,
}

const fn default_file_export_interval() -> u64 {
    15
}

pub struct FileMetricsExporter {
    config: FileExporterConfig,
}

impl FileMetricsExporter {
    pub fn new(config: FileExporterConfig) -> Self {
        Self { config }
    }
}

impl MetricsExporterTaskBuilder for FileMetricsExporter {
    async fn start_exporting(self, handle: PrometheusHandle) -> crate::Result<()> {
        let mut intervals = IntervalStream::new(interval(Duration::from_secs(
            self.config.export_interval_secs,
        )));

        while (intervals.next().await).is_some() {
            let file = match File::create(self.config.file_path.clone()).await {
                Ok(file) => file,
                Err(error) => {
                    error!(message = "Opening output file failed!", error = %error);
                    return Err(Box::new(error));
                }
            };
            BufWriterMetricsExporter::new_with_interval(
                BufWriter::new(file),
                self.config.export_interval_secs,
            )
            .export(&handle)
            .await?;
        }

        Ok(())
    }
}
