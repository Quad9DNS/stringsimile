use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio_stream::{StreamExt, wrappers::LinesStream};
use tracing::{error, warn};

use super::{InputStreamBuilder, metrics::InputMetrics};

pub struct BufReaderWithMetrics<R> {
    pub reader: BufReader<R>,
    pub metrics: InputMetrics,
}

impl<R: AsyncRead + Send + 'static> InputStreamBuilder for BufReaderWithMetrics<R> {
    async fn into_stream(
        self,
    ) -> crate::Result<
        std::pin::Pin<Box<dyn futures::Stream<Item = (String, Option<serde_json::Value>)> + Send>>,
    > {
        Ok(Box::pin(LinesStream::new(self.reader.lines()).filter_map(
            move |line| match line {
                Ok(line) => match serde_json::from_str(&line) {
                    Ok(parsed) => {
                        self.metrics.objects.increment(1);
                        self.metrics.bytes.increment(line.len() as u64);
                        Some((line, Some(parsed)))
                    }
                    Err(error) => {
                        self.metrics.parse_errors.increment(1);
                        warn!(message = "Parsing input line failed.", error = %error);
                        Some((line, None))
                    }
                },
                Err(error) => {
                    self.metrics.read_errors.increment(1);
                    error!(message = "Reading failed", error = %error);
                    None
                }
            },
        )))
    }
}
