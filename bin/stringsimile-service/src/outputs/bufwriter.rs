use std::pin::Pin;

use tokio::io::{AsyncWrite, AsyncWriteExt, BufWriter};
use tokio_stream::StreamExt;
use tracing::error;

use super::{metrics::OutputMetrics, serialization::json_serialize_value, OutputStreamBuilder};

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

#[cfg(test)]
mod tests {
    use futures::{stream, Stream};
    use serde_json::{json, Value};

    use super::*;

    async fn run_with_stream<S: Stream<Item = (String, Option<Value>)> + Send + 'static>(
        input_stream: S,
    ) -> String {
        let mut buffer = Vec::default();
        let writer = BufWriterWithMetrics {
            writer: BufWriter::new(&mut buffer),
            metrics: OutputMetrics::for_output_type("test"),
        };

        writer
            .consume_stream(Box::pin(input_stream))
            .await
            .expect("Output failed");

        String::from_utf8(buffer).expect("Invalid UTF8")
    }

    #[tokio::test]
    async fn just_original_input() {
        let input = stream::iter(vec![("original_input".to_string(), None)]);
        let result = run_with_stream(input).await;

        assert_eq!(result, "original_input\n");
    }

    #[tokio::test]
    async fn serialized_output() {
        let input = stream::iter(vec![(
            r#"{"input":      "test", "metadata":          {}}"#.to_string(),
            Some(json!({
                "input": "test",
                "metadata": {}
            })),
        )]);

        let result = run_with_stream(input).await;

        assert_eq!(result, "{\"input\":\"test\",\"metadata\":{}}\n");
    }
}
