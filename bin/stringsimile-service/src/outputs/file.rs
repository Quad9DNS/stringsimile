use std::path::PathBuf;

use futures::Stream;
use tokio::{fs::File, io::BufWriter};
use tracing::error;

use crate::message::StringsimileMessage;

use super::{OutputStreamBuilder, bufwriter::BufWriterWithMetrics, metrics::OutputMetrics};

pub struct FileStream(pub PathBuf);

impl OutputStreamBuilder for FileStream {
    async fn consume_stream(
        self,
        stream: std::pin::Pin<Box<dyn Stream<Item = StringsimileMessage> + Send>>,
    ) -> crate::Result<()> {
        let file = match File::create(self.0).await {
            Ok(file) => file,
            Err(error) => {
                error!(message = "Opening output file failed!", error = %error);
                return Err(Box::new(error));
            }
        };
        BufWriterWithMetrics {
            writer: BufWriter::new(file),
            metrics: OutputMetrics::for_output_type("file"),
        }
        .consume_stream(stream)
        .await
    }
}
