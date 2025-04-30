use serde_json::Value;
use tracing::warn;

use super::metrics::OutputMetrics;

pub async fn json_serialize_value(
    original_input: String,
    value: &Value,
    metrics: &OutputMetrics,
) -> String {
    match serde_json::to_string(&value) {
        Ok(serialized) => serialized,
        Err(err) => {
            metrics.serialization_errors.increment(1);
            warn!(message = "Serializing message for output failed. Writing unmodified input.", error = %err);
            original_input
        }
    }
}
