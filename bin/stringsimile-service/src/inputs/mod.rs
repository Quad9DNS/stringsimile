use std::{path::PathBuf, pin::Pin};

use file::FileStream;
use futures::Stream;
use serde_json::Value;
use stdin::StdinStream;

mod bufreader;
mod file;
mod stdin;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Input {
    Stdin,
    File(PathBuf),
}

impl InputStreamBuilder for Input {
    async fn into_stream(
        self,
    ) -> crate::Result<Pin<Box<dyn Stream<Item = (String, Option<Value>)> + Send>>> {
        match self {
            Input::Stdin => StdinStream.into_stream().await,
            Input::File(path_buf) => FileStream(path_buf).into_stream().await,
        }
    }
}

impl InputBuilder for Input {
    fn name(&self) -> String {
        match self {
            Input::Stdin => "stdin".to_string(),
            Input::File(path_buf) => {
                let file_name = path_buf.to_string_lossy();
                format!("file{file_name}")
            }
        }
    }
}

pub trait InputStreamBuilder {
    async fn into_stream(
        self,
    ) -> crate::Result<Pin<Box<dyn Stream<Item = (String, Option<Value>)> + Send>>>;
}

pub trait InputBuilder: InputStreamBuilder {
    fn name(&self) -> String;
}
