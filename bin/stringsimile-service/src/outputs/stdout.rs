use futures::Stream;
use tokio::io::{self, BufWriter};

use super::{OutputStreamBuilder, bufwriter::BufWriterWithMetrics, metrics::OutputMetrics};

pub struct StdoutOutput;

impl OutputStreamBuilder for StdoutOutput {
    async fn consume_stream(
        self,
        stream: std::pin::Pin<Box<dyn Stream<Item = (String, Option<serde_json::Value>)> + Send>>,
    ) -> crate::Result<()> {
        BufWriterWithMetrics {
            writer: BufWriter::new(io::stdout()),
            metrics: OutputMetrics::for_output_type("stdout"),
        }
        .consume_stream(stream)
        .await
    }
}
