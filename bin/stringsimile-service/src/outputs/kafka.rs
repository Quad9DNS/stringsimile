use std::hash::Hash;
use std::{collections::HashMap, time::Duration};

use futures::Stream;
use rdkafka::{
    ClientConfig,
    producer::{FutureProducer, FutureRecord},
};
use serde::{Deserialize, Serialize};
use tokio_stream::StreamExt;
use tracing::trace;

use super::OutputStreamBuilder;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KafkaOutputConfig {
    host: String,
    #[serde(default = "default_kafka_output_port")]
    port: usize,
    topic: String,
    identifier: String,
    #[serde(default)]
    librdkafka_options: HashMap<String, String>,
}

impl Hash for KafkaOutputConfig {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.host.hash(state);
        self.port.hash(state);
        self.topic.hash(state);
        self.identifier.hash(state);
    }
}

impl KafkaOutputConfig {
    pub fn server(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

const fn default_kafka_output_port() -> usize {
    9092
}

pub struct KafkaOutputStream {
    config: KafkaOutputConfig,
}

impl KafkaOutputStream {
    pub fn new(config: KafkaOutputConfig) -> Self {
        Self { config }
    }
}

impl OutputStreamBuilder for KafkaOutputStream {
    async fn consume_stream(
        self,
        mut stream: std::pin::Pin<
            Box<dyn Stream<Item = (String, Option<serde_json::Value>)> + Send>,
        >,
    ) -> crate::Result<()> {
        let mut config = ClientConfig::new();
        for (key, value) in &self.config.librdkafka_options {
            config.set(key, value);
        }
        config.set("bootstrap.servers", self.config.server());
        config.set("client.id", self.config.identifier);

        let producer: FutureProducer = config.create()?;

        while let Some((original_input, object)) = stream.next().await {
            if let Some(value) = object {
                let serialized = &serde_json::to_vec(&value).expect("Serialization failed");
                let send_status = producer
                    .send(
                        FutureRecord::<(), _>::to(&self.config.topic).payload(serialized),
                        Duration::from_secs(0),
                    )
                    .await
                    .expect("Kafka send failed");
                trace!("Kafka send status: {:?}", send_status);
            } else {
                let send_status = producer
                    .send(
                        FutureRecord::<(), _>::to(&self.config.topic)
                            .payload(original_input.as_bytes()),
                        Duration::from_secs(0),
                    )
                    .await
                    .expect("Kafka send failed");
                trace!("Kafka send status: {:?}", send_status);
            }
        }

        Ok(())
    }
}
