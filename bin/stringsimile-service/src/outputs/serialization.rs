use tracing::warn;

use crate::message::StringsimileMessage;

use super::metrics::OutputMetrics;

pub async fn json_serialize_value(message: StringsimileMessage, metrics: &OutputMetrics) -> String {
    if let Some(value) = message.parsed_value() {
        match serde_json::to_string(&value) {
            Ok(serialized) => serialized,
            Err(err) => {
                metrics.serialization_errors.increment(1);
                warn!(message = "Serializing message for output failed. Writing unmodified input.", error = %err);
                message.into_original_input()
            }
        }
    } else {
        message.into_original_input()
    }
}
