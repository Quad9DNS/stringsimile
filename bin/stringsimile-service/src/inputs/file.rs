use std::path::PathBuf;

use tokio::{fs::File, io::BufReader};
use tracing::error;

use super::InputStreamBuilder;

pub struct FileStream(pub PathBuf);

impl InputStreamBuilder for FileStream {
    async fn into_stream(
        self,
    ) -> crate::Result<
        std::pin::Pin<Box<dyn futures::Stream<Item = (String, Option<serde_json::Value>)> + Send>>,
    > {
        let file = match File::open(self.0).await {
            Ok(file) => file,
            Err(error) => {
                error!(message = "Opening input file failed!", error = %error);
                return Err(Box::new(error));
            }
        };
        BufReader::new(file).into_stream().await
    }
}
