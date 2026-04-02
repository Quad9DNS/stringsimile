use std::hash::Hash;
use std::{collections::HashMap, time::Duration};

use futures::StreamExt;
use rdkafka::ClientContext;
use rdkafka::consumer::{BaseConsumer, ConsumerContext, Rebalance};
use rdkafka::{
    ClientConfig, Message,
    consumer::{Consumer, StreamConsumer},
};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast::Receiver;
use tokio_stream::wrappers::BroadcastStream;
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
    pointer: Option<KafkaPointer>,
    #[serde(default)]
    librdkafka_options: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
enum KafkaPointer {
    Offset(u32),
    String(String),
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

pub struct KafkaInputContext {
    config: KafkaInputConfig,
}

impl ClientContext for KafkaInputContext {}

impl ConsumerContext for KafkaInputContext {
    fn post_rebalance(
        &self,
        base_consumer: &BaseConsumer<KafkaInputContext>,
        rebalance: &Rebalance<'_>,
    ) {
        if let Rebalance::Assign(offsets) = rebalance
            && let Some(pointer_config) = &self.config.pointer
        {
            let mut offsets = (*offsets).clone();
            match pointer_config {
                KafkaPointer::Offset(offset) => {
                    offsets.set_all_offsets(rdkafka::Offset::OffsetTail(*offset as i64))
                }
                KafkaPointer::String(string) if string == "now" || string == "end" => {
                    offsets.set_all_offsets(rdkafka::Offset::End)
                }
                KafkaPointer::String(string) if string == "begin" || string == "start" => {
                    offsets.set_all_offsets(rdkafka::Offset::Beginning)
                }
                KafkaPointer::String(special) => {
                    warn!("Unsupported kafka pointer value: {}", special);
                    Ok(())
                }
            }
            .unwrap_or_else(|err| {
                warn!("Failed to apply kafka input offsets: {}", err);
            });
            if let Err(err) =
                base_consumer.seek_partitions(offsets.clone(), Duration::from_secs(30))
            {
                warn!("Failed to seek kafka input partitions: {}", err);
            }
        }
    }
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
        shutdown: Receiver<()>,
    ) -> crate::Result<std::pin::Pin<Box<dyn futures::Stream<Item = StringsimileMessage> + Send>>>
    {
        let mut config = ClientConfig::new();

        config
            .set("enable.auto.commit", "true")
            .set("auto.commit.interval.ms", "5000")
            .set("enable.auto.offset.store", "true")
            .set("client.id", "stringsimile");

        for (key, value) in &self.config.librdkafka_options {
            config.set(key, value);
        }

        config
            .set("bootstrap.servers", self.config.server())
            .set("group.id", self.config.identifier.clone());

        let consumer: StreamConsumer<KafkaInputContext> =
            config.create_with_context(KafkaInputContext {
                config: self.config.clone(),
            })?;
        let topics: Vec<&str> = self.config.topics.iter().map(|t| t.as_str()).collect();
        consumer.subscribe(&topics)?;

        consumer.into_stream(shutdown).await
    }
}

impl InputStreamBuilder for StreamConsumer<KafkaInputContext> {
    async fn into_stream(
        self,
        shutdown: Receiver<()>,
    ) -> crate::Result<std::pin::Pin<Box<dyn futures::Stream<Item = StringsimileMessage> + Send>>>
    {
        let metrics = InputMetrics::for_input_type("kafka");
        let shutdown = BroadcastStream::new(shutdown).into_future();
        Ok(Box::pin(async_stream::stream! {
            let mut stream = self.stream().take_until(shutdown);
            loop {
                match stream.next().await {
                    None => {
                        // This must be a shutdown, just exit
                        break;
                    },
                    Some(Err(e)) => {
                        metrics.read_errors.increment(1);
                        warn!("Kafka error: {}", e)
                    },
                    Some(Ok(m)) => {
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
                    }
                };
            }
        }))
    }
}
