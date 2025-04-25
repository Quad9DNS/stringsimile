use std::path::PathBuf;

use futures::Stream;
use tokio::{fs::File, io::BufWriter};
use tracing::error;

use super::OutputStreamBuilder;

pub struct FileStream(pub PathBuf);

impl OutputStreamBuilder for FileStream {
    async fn consume_stream(
        self,
        stream: std::pin::Pin<Box<dyn Stream<Item = (String, Option<serde_json::Value>)> + Send>>,
    ) -> crate::Result<()> {
        let file = match File::create(self.0).await {
            Ok(file) => file,
            Err(error) => {
                error!(message = "Opening output file failed!", error = %error);
                return Err(Box::new(error));
            }
        };
        BufWriter::new(file).consume_stream(stream).await
    }
}
