use std::{path::PathBuf, pin::Pin};

use file::FileStream;
use futures::Stream;
use stdin::StdinStream;

mod bufreader;
mod file;
#[cfg(feature = "inputs-kafka")]
mod kafka;
mod pipe;
#[cfg(feature = "inputs-kafka")]
pub use kafka::KafkaInputConfig;

use crate::{inputs::pipe::PipeStream, message::StringsimileMessage};
mod metrics;
mod stdin;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Input {
    Stdin,
    File(PathBuf),
    Pipe(PathBuf),
    #[cfg(feature = "inputs-kafka")]
    Kafka(KafkaInputConfig),
}

impl InputStreamBuilder for Input {
    async fn into_stream(
        self,
    ) -> crate::Result<Pin<Box<dyn Stream<Item = StringsimileMessage> + Send>>> {
        match self {
            Input::Stdin => StdinStream.into_stream().await,
            Input::File(path_buf) => FileStream(path_buf).into_stream().await,
            Input::Pipe(path_buf) => PipeStream(path_buf).into_stream().await,
            #[cfg(feature = "inputs-kafka")]
            Input::Kafka(kafka_input_config) => {
                kafka::KafkaInputStream::new(kafka_input_config)
                    .into_stream()
                    .await
            }
        }
    }
}

impl InputBuilder for Input {
    fn name(&self) -> String {
        match self {
            Input::Stdin => "stdin".to_string(),
            Input::File(path_buf) => {
                let file_name = path_buf.to_string_lossy();
                format!("file({file_name})")
            }
            Input::Pipe(path_buf) => {
                let pipe_name = path_buf.to_string_lossy();
                format!("pipe({pipe_name})")
            }
            #[cfg(feature = "inputs-kafka")]
            Input::Kafka(kafka_input_config) => {
                let server = kafka_input_config.server();
                format!("kafka({server})")
            }
        }
    }
}

pub(crate) trait InputStreamBuilder {
    async fn into_stream(
        self,
    ) -> crate::Result<Pin<Box<dyn Stream<Item = StringsimileMessage> + Send>>>;
}

pub(crate) trait InputBuilder: InputStreamBuilder {
    fn name(&self) -> String;
}
