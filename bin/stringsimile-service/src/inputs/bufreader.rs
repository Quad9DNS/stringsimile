use futures::StreamExt as _;
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, BufReader},
    sync::broadcast::Receiver,
};
use tokio_stream::{
    StreamExt as _,
    wrappers::{BroadcastStream, LinesStream},
};
use tracing::{error, warn};

use crate::message::StringsimileMessage;

use super::{InputStreamBuilder, metrics::InputMetrics};

pub struct BufReaderWithMetrics<R> {
    pub reader: BufReader<R>,
    pub metrics: InputMetrics,
}

impl<R: AsyncRead + Send + 'static> InputStreamBuilder for BufReaderWithMetrics<R> {
    async fn into_stream(
        self,
        shutdown: Receiver<()>,
    ) -> crate::Result<std::pin::Pin<Box<dyn futures::Stream<Item = StringsimileMessage> + Send>>>
    {
        let shutdown = BroadcastStream::new(shutdown).into_future();
        Ok(Box::pin(
            LinesStream::new(self.reader.lines())
                .take_until(shutdown)
                .map_while(move |line| match line {
                    Ok(line) => match serde_json::from_str(&line) {
                        Ok(parsed) => {
                            self.metrics.objects.increment(1);
                            self.metrics.bytes.increment(line.len() as u64);
                            Some(StringsimileMessage::new_parsed(line, parsed))
                        }
                        Err(error) => {
                            self.metrics.parse_errors.increment(1);
                            warn!(message = "Parsing input line failed.", error = %error);
                            Some(StringsimileMessage::new_unparsed(line))
                        }
                    },
                    Err(error) => {
                        self.metrics.read_errors.increment(1);
                        error!(message = "Reading failed", error = %error);
                        None
                    }
                }),
        ))
    }
}

#[cfg(test)]
mod tests {
    use tokio::sync::broadcast::Sender;

    use super::*;

    async fn build_stream<R: AsyncRead + Send + 'static>(
        reader: BufReader<R>,
    ) -> crate::Result<(
        std::pin::Pin<Box<dyn futures::Stream<Item = StringsimileMessage> + Send>>,
        Sender<()>,
    )> {
        let (tx, rx) = tokio::sync::broadcast::channel(1);
        let reader_with_metrics = BufReaderWithMetrics {
            reader,
            metrics: InputMetrics::for_input_type("test"),
        };

        Ok((reader_with_metrics.into_stream(rx).await?, tx))
    }

    #[tokio::test]
    async fn bad_input() {
        let input = "".as_bytes();
        let reader = BufReader::new(input);

        let result = build_stream(reader).await;
        assert!(result.is_ok());
        let (mut stream, _shutdown) = result.unwrap();

        assert!(tokio_stream::StreamExt::next(&mut stream).await.is_none());
    }

    #[tokio::test]
    async fn invalid_json() {
        let input = "test".as_bytes();
        let reader = BufReader::new(input);

        let result = build_stream(reader).await;
        assert!(result.is_ok());
        let (mut stream, _shutdown) = result.unwrap();

        let first_item = tokio_stream::StreamExt::next(&mut stream)
            .await
            .expect("Read failed");
        // Preserve original text
        assert_eq!(first_item.original_input(), "test");
        // Invalid JSON so no parsed value
        assert!(first_item.parsed_value().is_none());
    }

    #[tokio::test]
    async fn valid_json() {
        let original_input = r#"{"input": "test", "metadata": {}}"#;
        let input = original_input.as_bytes();
        let reader = BufReader::new(input);

        let result = build_stream(reader).await;
        assert!(result.is_ok());
        let (mut stream, _shutdown) = result.unwrap();

        let first_item = tokio_stream::StreamExt::next(&mut stream)
            .await
            .expect("Read failed");
        // Preserve original text
        assert_eq!(first_item.original_input(), original_input);

        let parsed_value = first_item.parsed_value().expect("Parse failed");
        let object = parsed_value.as_object().expect("JSON not an object");
        assert_eq!(
            object
                .get("input")
                .expect("Missing input field")
                .as_str()
                .expect("Expected input to be a string"),
            "test"
        );
        assert!(
            object
                .get("metadata")
                .expect("Missing metadata field")
                .as_object()
                .expect("Expected metadata to be an object")
                .is_empty(),
        );
    }

    #[tokio::test]
    async fn shutdown() {
        let original_input = r#"{"input": "test", "metadata": {}}"#;
        let input = original_input.as_bytes();
        let reader = BufReader::new(input);

        let result = build_stream(reader).await;
        assert!(result.is_ok());
        let (mut stream, shutdown) = result.unwrap();

        shutdown.send(()).expect("Shutdown failed");
        assert!(tokio_stream::StreamExt::next(&mut stream).await.is_none());
    }
}
