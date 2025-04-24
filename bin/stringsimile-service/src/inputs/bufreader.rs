use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio_stream::{StreamExt, wrappers::LinesStream};
use tracing::{error, warn};

use super::InputStreamBuilder;

impl<R: AsyncRead + Send + 'static> InputStreamBuilder for BufReader<R> {
    async fn into_stream(
        self,
    ) -> crate::Result<
        std::pin::Pin<Box<dyn futures::Stream<Item = (String, Option<serde_json::Value>)> + Send>>,
    > {
        Ok(Box::pin(LinesStream::new(self.lines()).filter_map(
            |line| match line {
                Ok(line) => match serde_json::from_str(&line) {
                    Ok(parsed) => Some((line, Some(parsed))),
                    Err(error) => {
                        warn!(message = "Parsing input line failed.", error = %error);
                        Some((line, None))
                    }
                },
                Err(error) => {
                    error!(message = "Reading failed", error = %error);
                    None
                }
            },
        )))
    }
}
