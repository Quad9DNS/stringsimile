use std::collections::HashMap;
use std::hash::Hash;

use rdkafka::{
    ClientConfig, Message,
    consumer::{CommitMode, Consumer, StreamConsumer},
};
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::message::StringsimileMessage;

use super::{InputStreamBuilder, metrics::InputMetrics};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KafkaInputConfig {
    host: String,
    #[serde(default = "default_kafka_input_port")]
    port: usize,
    topics: Vec<String>,
    identifier: String,
    pointer: usize,
    #[serde(default)]
    librdkafka_options: HashMap<String, String>,
}

impl Hash for KafkaInputConfig {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.host.hash(state);
        self.port.hash(state);
        self.topics.hash(state);
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
    ) -> crate::Result<std::pin::Pin<Box<dyn futures::Stream<Item = StringsimileMessage> + Send>>>
    {
        let mut config = ClientConfig::new();
        for (key, value) in &self.config.librdkafka_options {
            config.set(key, value);
        }
        config.set("bootstrap.servers", self.config.server());
        config.set("group.id", self.config.identifier);

        let consumer: StreamConsumer = config.create()?;
        let topics: Vec<&str> = self.config.topics.iter().map(|t| t.as_str()).collect();
        consumer.subscribe(&topics)?;

        consumer.into_stream().await
    }
}

impl InputStreamBuilder for StreamConsumer {
    async fn into_stream(
        self,
    ) -> crate::Result<std::pin::Pin<Box<dyn futures::Stream<Item = StringsimileMessage> + Send>>>
    {
        let metrics = InputMetrics::for_input_type("kafka");
        Ok(Box::pin(async_stream::stream! {
            loop {
                match self.recv().await {
                    Err(e) => {
                        metrics.read_errors.increment(1);
                        warn!("Kafka error: {}", e)
                    },
                    Ok(m) => {
                        match m.payload_view::<str>() {
                            None => {
                                metrics.read_errors.increment(1);
                                warn!("Error while reading kafka message");
                            },
                            Some(Ok(s)) => match serde_json::from_str(s) {
                                Ok(parsed) => {
                                    metrics.objects.increment(1);
                                    metrics.bytes.increment(s.len() as u64);
                                    yield StringsimileMessage::new_parsed(s.to_string(), parsed)
                                },
                                Err(error) => {
                                    metrics.parse_errors.increment(1);
                                    warn!(message = "Parsing input message failed.", error = %error);
                                    yield StringsimileMessage::new_unparsed(s.to_string())
                                }
                            },
                            Some(Err(e)) => {
                                metrics.parse_errors.increment(1);
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
