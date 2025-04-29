use std::collections::HashMap;
use std::hash::Hash;

use rdkafka::{
    ClientConfig, Message,
    consumer::{CommitMode, Consumer, StreamConsumer},
};
use serde::{Deserialize, Serialize};
use tracing::warn;

use super::InputStreamBuilder;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KafkaInputConfig {
    host: String,
    #[serde(default = "default_kafka_input_port")]
    port: usize,
    topic: String,
    identifier: String,
    pointer: usize,
    #[serde(default)]
    librdkafka_options: HashMap<String, String>,
}

impl Hash for KafkaInputConfig {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.host.hash(state);
        self.port.hash(state);
        self.topic.hash(state);
        self.identifier.hash(state);
        self.pointer.hash(state);
    }
}

impl KafkaInputConfig {
    pub fn server(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

const fn default_kafka_input_port() -> usize {
    9092
}

pub struct KafkaInputStream {
    config: KafkaInputConfig,
}

impl KafkaInputStream {
    pub fn new(config: KafkaInputConfig) -> Self {
        Self { config }
    }
}

impl InputStreamBuilder for KafkaInputStream {
    async fn into_stream(
        self,
    ) -> crate::Result<
        std::pin::Pin<Box<dyn futures::Stream<Item = (String, Option<serde_json::Value>)> + Send>>,
    > {
        let mut config = ClientConfig::new();
        for (key, value) in &self.config.librdkafka_options {
            config.set(key, value);
        }
        config.set("bootstrap.servers", self.config.server());

        let consumer: StreamConsumer = config.create()?;
        consumer.subscribe(&[&self.config.topic])?;

        consumer.into_stream().await
    }
}

impl InputStreamBuilder for StreamConsumer {
    async fn into_stream(
        self,
    ) -> crate::Result<
        std::pin::Pin<Box<dyn futures::Stream<Item = (String, Option<serde_json::Value>)> + Send>>,
    > {
        Ok(Box::pin(async_stream::stream! {
            loop {
                match self.recv().await {
                    Err(e) => warn!("Kafka error: {}", e),
                    Ok(m) => {
                        match m.payload_view::<str>() {
                            None => {
                                warn!("Error while reading kafka message");
                            },
                            Some(Ok(s)) => match serde_json::from_str(s) {
                                Ok(parsed) => yield (s.to_string(), Some(parsed)),
                                Err(error) => {
                                    warn!(message = "Parsing input message failed.", error = %error);
                                    yield (s.to_string(), None);
                                }
                            },
                            Some(Err(e)) => {
                                warn!("Error while deserializing message payload: {:?}", e);
                            }
                        };
                        self.commit_message(&m, CommitMode::Async).unwrap();
                    }
                };
            }
        }))
    }
}
