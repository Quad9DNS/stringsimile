use metrics::{Counter, counter};

#[derive(Clone)]
pub struct InputMetrics {
    pub objects: Counter,
    pub bytes: Counter,
    pub read_errors: Counter,
    pub parse_errors: Counter,
}

impl InputMetrics {
    pub fn for_input_type(input_type: &str) -> Self {
        Self {
            objects: counter!(
                "input_objects_read",
                "input_type" => input_type.to_string()
            ),
            bytes: counter!(
                "input_bytes_read",
                "input_type" => input_type.to_string()
            ),
            read_errors: counter!(
                "input_read_errors",
                "input_type" => input_type.to_string()
            ),
            parse_errors: counter!(
                "input_parse_errors",
                "input_type" => input_type.to_string()
            ),
        }
    }
}
