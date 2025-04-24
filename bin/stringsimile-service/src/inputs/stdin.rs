use tokio::io::{self, BufReader};

use super::InputStreamBuilder;

pub struct StdinStream;

impl InputStreamBuilder for StdinStream {
    async fn into_stream(
        self,
    ) -> crate::Result<
        std::pin::Pin<Box<dyn futures::Stream<Item = (String, Option<serde_json::Value>)> + Send>>,
    > {
        let stdin = io::stdin();
        BufReader::new(stdin).into_stream().await
    }
}
