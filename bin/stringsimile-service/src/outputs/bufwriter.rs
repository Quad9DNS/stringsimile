use std::pin::Pin;

use tokio::io::{AsyncWrite, AsyncWriteExt, BufWriter};
use tokio_stream::StreamExt;
use tracing::error;

use super::{OutputStreamBuilder, metrics::OutputMetrics, serialization::json_serialize_value};

pub struct BufWriterWithMetrics<W> {
    pub writer: BufWriter<W>,
    pub metrics: OutputMetrics,
}

async fn write_string<W: AsyncWrite>(
    writer: &mut Pin<Box<BufWriter<W>>>,
    metrics: &OutputMetrics,
    value: String,
) -> crate::Result<()> {
    if let Err(err) = writer.write_all(value.as_bytes()).await {
        metrics.write_errors.increment(1);
        error!(message = "Output write failed", error = %err);
        return Err(err.into());
    }
    metrics.objects.increment(1);
    metrics.bytes.increment(value.len() as u64);
    Ok(())
}

impl<W: AsyncWrite> OutputStreamBuilder for BufWriterWithMetrics<W> {
    async fn consume_stream(
        self,
        mut stream: std::pin::Pin<
            Box<dyn futures::Stream<Item = (String, Option<serde_json::Value>)> + Send>,
        >,
    ) -> crate::Result<()> {
        let mut writer = Box::pin(self.writer);
        while let Some((original_input, object)) = stream.next().await {
            let value_to_write = if let Some(value) = object {
                json_serialize_value(original_input, &value, &self.metrics).await
            } else {
                original_input
            };
            if write_string(&mut writer, &self.metrics, value_to_write + "\n")
                .await
                .is_err()
            {
                continue;
            }
            if let Err(err) = writer.flush().await {
                self.metrics.write_errors.increment(1);
                error!(message = "Output flush failed!", error = %err);
            }
        }

        Ok(())
    }
}
