use futures::Stream;
use tokio::io::{self, BufWriter};

use super::OutputStreamBuilder;

pub struct StdoutOutput;

impl OutputStreamBuilder for StdoutOutput {
    async fn consume_stream(
        self,
        stream: std::pin::Pin<Box<dyn Stream<Item = (String, Option<serde_json::Value>)> + Send>>,
    ) -> crate::Result<()> {
        BufWriter::new(io::stdout()).consume_stream(stream).await
    }
}
