use metrics::{Counter, Unit, counter, describe_counter};

#[derive(Clone)]
pub struct OutputMetrics {
    pub objects: Counter,
    pub bytes: Counter,
    pub write_errors: Counter,
    pub serialization_errors: Counter,
}

impl OutputMetrics {
    pub fn for_output_type(output_type: &str) -> Self {
        describe_counter!(
            "output_objects_written",
            Unit::Count,
            "Number of objects written to the output"
        );
        describe_counter!(
            "output_bytes_written",
            Unit::Bytes,
            "Number of bytes written to the output"
        );
        describe_counter!(
            "output_errors",
            Unit::Count,
            "Number of errors encountered by this output"
        );
        Self {
            objects: counter!(
                "output_objects_written",
                "output_type" => output_type.to_string()
            ),
            bytes: counter!(
                "output_bytes_written",
                "output_type" => output_type.to_string()
            ),
            write_errors: counter!(
                "output_errors",
                "output_type" => output_type.to_string(),
                "error_type" => "write"
            ),
            serialization_errors: counter!(
                "output_errors",
                "output_type" => output_type.to_string(),
                "error_type" => "serialization"
            ),
        }
    }
}
