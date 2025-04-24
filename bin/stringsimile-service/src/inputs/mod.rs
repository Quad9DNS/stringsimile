use std::{path::PathBuf, pin::Pin};

use async_stream::stream;
use futures::Stream;
use serde_json::Value;
use tokio::{
    fs::File,
    io::{self, AsyncBufReadExt, BufReader},
};
use tracing::{error, info, warn};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Input {
    Stdin,
    File(PathBuf),
}

impl Input {
    pub fn name(&self) -> String {
        match self {
            Input::Stdin => "stdin".to_string(),
            Input::File(path_buf) => {
                let file_name = path_buf.to_string_lossy();
                format!("file{file_name}")
            }
        }
    }

    pub fn into_stream(self) -> Pin<Box<dyn Stream<Item = (String, Value)> + Send>> {
        match self {
            Input::Stdin => Box::pin(stream! {
                let stdin = io::stdin();
                let reader = BufReader::new(stdin);
                let mut lines = reader.lines();

                loop {
                    tokio::select! {
                        line = lines.next_line() => match line {
                            Ok(Some(line)) => {
                                let Ok(parsed) = serde_json::from_str(&line) else {
                                    warn!("Parsing input line failed.");
                                    break;
                                };
                                yield (line, parsed);
                            },
                            Ok(None) => {
                                info!("EOF reached.");
                                break;
                            },
                            Err(error) => {
                                error!(message = "Reading failed", error = %error);
                            },
                        }
                    }
                }
            }),
            Input::File(path_buf) => Box::pin(stream! {
                let file = match File::open(path_buf).await {
                    Ok(file) => file,
                    Err(error) => {
                        error!(message = "Opening input file failed!", error = %error);
                        return;
                    }
                };
                let reader = BufReader::new(file);
                let mut lines = reader.lines();

                loop {
                    tokio::select! {
                        line = lines.next_line() => match line {
                            Ok(Some(line)) => {
                                let Ok(parsed) = serde_json::from_str(&line) else {
                                    warn!("Parsing input line failed.");
                                    break;
                                };
                                yield parsed;
                            },
                            Ok(None) => {
                                info!("EOF reached.");
                                break;
                            },
                            Err(error) => {
                                error!(message = "Reading failed", error = %error);
                            },
                        }
                    }
                }
            }),
        }
    }
}
