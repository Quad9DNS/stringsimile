use futures::Stream;
use tokio::io::{self, BufWriter};

use crate::message::StringsimileMessage;

use super::{OutputStreamBuilder, bufwriter::BufWriterWithMetrics, metrics::OutputMetrics};

pub struct StdoutOutput;

impl OutputStreamBuilder for StdoutOutput {
    async fn consume_stream(
        self,
        stream: std::pin::Pin<Box<dyn Stream<Item = StringsimileMessage> + Send>>,
    ) -> crate::Result<()> {
        BufWriterWithMetrics {
            writer: BufWriter::new(io::stdout()),
            metrics: OutputMetrics::for_output_type("stdout"),
        }
        .consume_stream(stream)
        .await
    }
}
