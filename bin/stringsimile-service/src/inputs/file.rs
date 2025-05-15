use std::path::PathBuf;

use tokio::{fs::File, io::BufReader};
use tracing::error;

use crate::message::StringsimileMessage;

use super::{InputStreamBuilder, bufreader::BufReaderWithMetrics, metrics::InputMetrics};

pub struct FileStream(pub PathBuf);

impl InputStreamBuilder for FileStream {
    async fn into_stream(
        self,
    ) -> crate::Result<std::pin::Pin<Box<dyn futures::Stream<Item = StringsimileMessage> + Send>>>
    {
        let file = match File::open(self.0).await {
            Ok(file) => file,
            Err(error) => {
                error!(message = "Opening input file failed!", error = %error);
                return Err(Box::new(error));
            }
        };
        BufReaderWithMetrics {
            reader: BufReader::new(file),
            metrics: InputMetrics::for_input_type("file"),
        }
        .into_stream()
        .await
    }
}
