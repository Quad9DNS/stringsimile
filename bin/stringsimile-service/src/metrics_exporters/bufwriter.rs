use std::time::Duration;

use futures::StreamExt;
use metrics_exporter_prometheus::PrometheusHandle;
use tokio::{
    io::{AsyncWrite, AsyncWriteExt, BufWriter},
    time::interval,
};
use tokio_stream::wrappers::IntervalStream;

use super::MetricsExporterTaskBuilder;

pub struct BufWriterMetricsExporter<W> {
    writer: BufWriter<W>,
    export_interval_secs: u64,
}

impl<W: AsyncWrite> BufWriterMetricsExporter<W> {
    pub fn new_with_interval(writer: BufWriter<W>, interval: u64) -> Self {
        Self {
            writer,
            export_interval_secs: interval,
        }
    }

    pub async fn export(self, handle: &PrometheusHandle) -> crate::Result<()> {
        let mut writer = Box::pin(self.writer);
        writer.write_all(handle.render().as_bytes()).await?;
        writer.flush().await?;
        Ok(())
    }
}

impl<W: AsyncWrite> MetricsExporterTaskBuilder for BufWriterMetricsExporter<W> {
    async fn start_exporting(self, handle: PrometheusHandle) -> crate::Result<()> {
        let mut intervals =
            IntervalStream::new(interval(Duration::from_secs(self.export_interval_secs)));

        let mut writer = Box::pin(self.writer);
        while (intervals.next().await).is_some() {
            writer
                .write_all(handle.render().as_bytes())
                .await
                .expect("Metrics write failed");
            writer.flush().await.expect("Flush failed");
        }

        Ok(())
    }
}
