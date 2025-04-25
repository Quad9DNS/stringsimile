use std::{path::PathBuf, pin::Pin};

use file::FileStream;
use futures::Stream;
use serde_json::Value;
use stdout::StdoutOutput;

mod bufwriter;
mod file;
mod stdout;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Output {
    Stdout,
    File(PathBuf),
}

impl OutputStreamBuilder for Output {
    async fn consume_stream(
        self,
        stream: Pin<Box<dyn Stream<Item = (String, Option<Value>)> + Send>>,
    ) -> crate::Result<()> {
        match self {
            Output::Stdout => StdoutOutput.consume_stream(stream).await,
            Output::File(path_buf) => FileStream(path_buf).consume_stream(stream).await,
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
        }
    }
}

pub trait OutputStreamBuilder {
    async fn consume_stream(
        self,
        stream: Pin<Box<dyn Stream<Item = (String, Option<Value>)> + Send>>,
    ) -> crate::Result<()>;
}

pub trait OutputBuilder: OutputStreamBuilder {
    #[allow(unused)]
    fn name(&self) -> String;
}
