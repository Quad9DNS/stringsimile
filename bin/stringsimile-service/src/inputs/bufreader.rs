use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio_stream::{StreamExt, wrappers::LinesStream};
use tracing::{error, warn};

use super::{InputStreamBuilder, metrics::InputMetrics};

pub struct BufReaderWithMetrics<R> {
    pub reader: BufReader<R>,
    pub metrics: InputMetrics,
}

impl<R: AsyncRead + Send + 'static> InputStreamBuilder for BufReaderWithMetrics<R> {
    async fn into_stream(
        self,
    ) -> crate::Result<
        std::pin::Pin<Box<dyn futures::Stream<Item = (String, Option<serde_json::Value>)> + Send>>,
    > {
        Ok(Box::pin(LinesStream::new(self.reader.lines()).filter_map(
            move |line| match line {
                Ok(line) => match serde_json::from_str(&line) {
                    Ok(parsed) => {
                        self.metrics.objects.increment(1);
                        self.metrics.bytes.increment(line.len() as u64);
                        Some((line, Some(parsed)))
                    }
                    Err(error) => {
                        self.metrics.parse_errors.increment(1);
                        warn!(message = "Parsing input line failed.", error = %error);
                        Some((line, None))
                    }
                },
                Err(error) => {
                    self.metrics.read_errors.increment(1);
                    error!(message = "Reading failed", error = %error);
                    None
                }
            },
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn bad_input() {
        let input = "".as_bytes();
        let reader = BufReader::new(input);
        let reader_with_metrics = BufReaderWithMetrics {
            reader,
            metrics: InputMetrics::for_input_type("test"),
        };

        let result = reader_with_metrics.into_stream().await;
        assert!(result.is_ok());
        let mut stream = result.unwrap();

        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn invalid_json() {
        let input = "test".as_bytes();
        let reader = BufReader::new(input);
        let reader_with_metrics = BufReaderWithMetrics {
            reader,
            metrics: InputMetrics::for_input_type("test"),
        };

        let result = reader_with_metrics.into_stream().await;
        assert!(result.is_ok());
        let mut stream = result.unwrap();

        let first_item = stream.next().await.expect("Read failed");
        // Preserve original text
        assert_eq!(first_item.0, "test");
        // Invalid JSON so no parsed value
        assert!(first_item.1.is_none());
    }

    #[tokio::test]
    async fn valid_json() {
        let original_input = r#"{"input": "test", "metadata": {}}"#;
        let input = original_input.as_bytes();
        let reader = BufReader::new(input);
        let reader_with_metrics = BufReaderWithMetrics {
            reader,
            metrics: InputMetrics::for_input_type("test"),
        };

        let result = reader_with_metrics.into_stream().await;
        assert!(result.is_ok());
        let mut stream = result.unwrap();

        let first_item = stream.next().await.expect("Read failed");
        // Preserve original text
        assert_eq!(first_item.0, original_input);

        let parsed_value = first_item.1.expect("Parse failed");
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
}
