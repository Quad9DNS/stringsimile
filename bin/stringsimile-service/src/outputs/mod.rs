use std::{path::PathBuf, pin::Pin};

use file::FileStream;
use futures::Stream;
use stdout::StdoutOutput;

mod bufwriter;
mod file;
#[cfg(feature = "outputs-kafka")]
mod kafka;
#[cfg(feature = "outputs-kafka")]
pub use kafka::KafkaOutputConfig;

use crate::message::StringsimileMessage;
mod metrics;
mod serialization;
mod stdout;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Output {
    Stdout,
    File(PathBuf),
    #[cfg(feature = "outputs-kafka")]
    Kafka(KafkaOutputConfig),
}

impl OutputStreamBuilder for Output {
    async fn consume_stream(
        self,
        stream: Pin<Box<dyn Stream<Item = StringsimileMessage> + Send>>,
    ) -> crate::Result<()> {
        match self {
            Output::Stdout => StdoutOutput.consume_stream(stream).await,
            Output::File(path_buf) => FileStream(path_buf).consume_stream(stream).await,
            #[cfg(feature = "outputs-kafka")]
            Output::Kafka(kafka_otuput_config) => {
                kafka::KafkaOutputStream::new(kafka_otuput_config)
                    .consume_stream(stream)
                    .await
            }
        }
    }
}

impl OutputBuilder for Output {
    fn name(&self) -> String {
        match self {
            Output::Stdout => "stdout".to_string(),
            Output::File(path_buf) => {
                let file_name = path_buf.to_string_lossy();
                format!("file{file_name}")
            }
            #[cfg(feature = "outputs-kafka")]
            Output::Kafka(kafka_output_config) => {
                let server = kafka_output_config.server();
                format!("kafka({server})")
            }
        }
    }
}

pub(crate) trait OutputStreamBuilder {
    async fn consume_stream(
        self,
        stream: Pin<Box<dyn Stream<Item = StringsimileMessage> + Send>>,
    ) -> crate::Result<()>;
}

pub(crate) trait OutputBuilder: OutputStreamBuilder {
    #[allow(unused)]
    fn name(&self) -> String;
}
