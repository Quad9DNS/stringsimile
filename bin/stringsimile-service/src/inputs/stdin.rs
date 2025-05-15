use tokio::io::{self, BufReader};

use crate::message::StringsimileMessage;

use super::{InputStreamBuilder, bufreader::BufReaderWithMetrics, metrics::InputMetrics};

pub struct StdinStream;

impl InputStreamBuilder for StdinStream {
    async fn into_stream(
        self,
    ) -> crate::Result<std::pin::Pin<Box<dyn futures::Stream<Item = StringsimileMessage> + Send>>>
    {
        BufReaderWithMetrics {
            reader: BufReader::new(io::stdin()),
            metrics: InputMetrics::for_input_type("stdin"),
        }
        .into_stream()
        .await
    }
}
