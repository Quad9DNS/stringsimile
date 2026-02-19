use std::path::PathBuf;

use tokio::{io::BufReader, net::unix::pipe::OpenOptions, sync::broadcast::Receiver};
use tracing::error;

use crate::message::StringsimileMessage;

use super::{InputStreamBuilder, bufreader::BufReaderWithMetrics, metrics::InputMetrics};

pub struct PipeStream(pub PathBuf);

impl InputStreamBuilder for PipeStream {
    async fn into_stream(
        self,
        shutdown: Receiver<()>,
    ) -> crate::Result<std::pin::Pin<Box<dyn futures::Stream<Item = StringsimileMessage> + Send>>>
    {
        let receiver = OpenOptions::new()
            .read_write(true)
            .open_receiver(self.0)
            .inspect_err(|error| error!(message = "Opening input pipe failed!", error = %error))?;
        BufReaderWithMetrics {
            reader: BufReader::new(receiver),
            metrics: InputMetrics::for_input_type("pipe"),
        }
        .into_stream(shutdown)
        .await
    }
}
