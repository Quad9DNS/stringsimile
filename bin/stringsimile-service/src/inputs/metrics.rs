use metrics::{Counter, Unit, counter, describe_counter};

#[derive(Clone)]
pub struct InputMetrics {
    pub objects: Counter,
    pub bytes: Counter,
    pub read_errors: Counter,
    pub parse_errors: Counter,
}

impl InputMetrics {
    pub fn for_input_type(input_type: &str) -> Self {
        describe_counter!(
            "input_objects_read",
            Unit::Count,
            "Number of objects read by this input"
        );
        describe_counter!(
            "input_bytes_read",
            Unit::Bytes,
            "Number of bytes read by this input"
        );
        describe_counter!(
            "input_errors",
            Unit::Count,
            "Number of errors encountered by this input"
        );
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
                "input_errors",
                "input_type" => input_type.to_string(),
                "error_type" => "read"
            ),
            parse_errors: counter!(
                "input_errors",
                "input_type" => input_type.to_string(),
                "error_type" => "parse"
            ),
        }
    }
}
