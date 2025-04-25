use tokio::io::{AsyncWrite, AsyncWriteExt, BufWriter};
use tokio_stream::StreamExt;

use super::OutputStreamBuilder;

impl<W: AsyncWrite> OutputStreamBuilder for BufWriter<W> {
    async fn consume_stream(
        self,
        mut stream: std::pin::Pin<
            Box<dyn futures::Stream<Item = (String, Option<serde_json::Value>)> + Send>,
        >,
    ) -> crate::Result<()> {
        let mut this = Box::pin(self);
        while let Some((original_input, object)) = stream.next().await {
            if let Some(value) = object {
                this.write_all(&serde_json::to_vec(&value).expect("Serialization failed"))
                    .await
                    .expect("Write failed");
            } else {
                this.write_all(original_input.as_bytes())
                    .await
                    .expect("Write failed");
            }
            this.write_all(b"\n").await.expect("Write failed");
            this.flush().await.expect("Flush failed");
        }

        Ok(())
    }
}
