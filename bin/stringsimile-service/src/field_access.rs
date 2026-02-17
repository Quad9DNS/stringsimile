use serde::{Deserialize, Serialize};
use serde_json::Value;
use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum FieldAccessError {
    #[snafu(display("Input data was not valid JSON!"))]
    InvalidJson,

    #[snafu(display("Input data was not a JSON object! Found: {value}"))]
    NotAJsonObject { value: Value },

    #[snafu(display("Field was expected to be a JSON object, but found: {value}"))]
    FieldNotAJsonObject { value: Value },

    #[snafu(display("Field was expected to be a JSON array, but found: {value}"))]
    FieldNotAJsonArray { value: Value },

    #[snafu(display("Specified key field ({field_name}) not found in input object."))]
    FieldNotFound { field_name: String },

    #[snafu(display("Specified key field was not a string. Found: {field_value}"))]
    FieldNotString { field_value: Value },
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum FieldAccessorConfigError {
    #[snafu(display("Found empty field name in field name at {pos}"))]
    EmptyFieldName { pos: usize },

    #[snafu(display(
        "Invalid indexing sequence in field accessor found in ({start}, {end}): {sequence}"
    ))]
    InvalidIndexingSequence {
        start: usize,
        end: usize,
        sequence: String,
    },

    #[snafu(display(
        "Invalid indexing sequence in field accessor found at {pos}. Missing field access (.) after indexing."
    ))]
    MissingFieldAccessAfterIndexing { pos: usize },

    #[snafu(display(
        "Invalid indexing sequence in field accessor found at {pos}. Missing closing bracket."
    ))]
    MissingEndOfIndexingSequence { pos: usize },

    #[snafu(display(
        "Found unexpected indexing sequence. Accessors can't start with an indexing sequence."
    ))]
    UnexpectedIndexingSequence,
}

#[derive(Debug, Clone)]
pub struct FieldAccessor {
    /// The original input field value given in config
    input_field: String,
    /// Steps to parse input object to reach the field
    parse_steps: Vec<ParseStep>,
}

#[derive(Debug, Clone)]
pub enum ParseStep {
    FieldAccess { field_name: String },
    Indexing { index: usize },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FieldAccessorConfig(pub String);

impl FieldAccessorConfig {
    pub fn build(&self) -> crate::Result<FieldAccessor> {
        let input_field = &self.0;
        let mut parse_steps = Vec::new();

        let mut chunk_start = 0;
        let mut expect_empty = false;
        while let Some(chunk_end) = input_field[chunk_start..].find(['.', '[']) {
            let part = &input_field[chunk_start..(chunk_start + chunk_end)].to_string();
            if expect_empty && !part.is_empty() {
                return Err(FieldAccessorConfigError::MissingFieldAccessAfterIndexing {
                    pos: chunk_start,
                }
                .into());
            } else if !expect_empty && chunk_start != 0 && part.is_empty() {
                return Err(FieldAccessorConfigError::EmptyFieldName { pos: chunk_start }.into());
            }
            expect_empty = false;
            chunk_start += chunk_end + 1;
            if !part.is_empty() {
                parse_steps.push(ParseStep::FieldAccess {
                    field_name: part.clone(),
                });
            }

            if let Some('[') = input_field[(chunk_start - 1)..].chars().next() {
                let Some(indexing_end) = input_field[chunk_start..].find(']') else {
                    return Err(FieldAccessorConfigError::MissingEndOfIndexingSequence {
                        pos: chunk_start - 1,
                    }
                    .into());
                };
                let Ok(index) =
                    input_field[(chunk_start)..(chunk_start + indexing_end)].parse::<usize>()
                else {
                    return Err(FieldAccessorConfigError::InvalidIndexingSequence {
                        start: chunk_start,
                        end: chunk_start + indexing_end,
                        sequence: input_field[(chunk_start)..(chunk_start + indexing_end)]
                            .to_string(),
                    }
                    .into());
                };
                // Can't start with indexing step
                if parse_steps.is_empty() {
                    return Err(FieldAccessorConfigError::UnexpectedIndexingSequence.into());
                }
                parse_steps.push(ParseStep::Indexing { index });
                chunk_start += indexing_end + 1;
                expect_empty = true;
            }
        }

        match (expect_empty, chunk_start == input_field.len()) {
            (true, true) => (),
            (true, false) => {
                return Err(FieldAccessorConfigError::MissingFieldAccessAfterIndexing {
                    pos: chunk_start,
                }
                .into());
            }
            (false, true) => {
                return Err(FieldAccessorConfigError::EmptyFieldName { pos: chunk_start }.into());
            }
            (false, false) => {
                parse_steps.push(ParseStep::FieldAccess {
                    field_name: input_field[chunk_start..].to_string(),
                });
            }
        }

        Ok(FieldAccessor {
            input_field: input_field.to_string(),
            parse_steps,
        })
    }
}

impl FieldAccessor {
    pub fn access_field<'a>(&self, object: &'a Value) -> crate::Result<&'a str> {
        if !object.is_object() {
            return Err(FieldAccessError::NotAJsonObject {
                value: object.clone(),
            }
            .into());
        };

        let mut current_value = object;
        for step in &self.parse_steps {
            match step {
                ParseStep::FieldAccess { field_name } => {
                    current_value = current_value
                        .as_object()
                        .ok_or(FieldAccessError::FieldNotAJsonObject {
                            value: current_value.clone(),
                        })?
                        .get(field_name)
                        .ok_or(FieldAccessError::FieldNotFound {
                            field_name: self.input_field.clone(),
                        })?;
                }
                ParseStep::Indexing { index } => {
                    current_value = current_value
                        .as_array()
                        .ok_or(FieldAccessError::FieldNotAJsonArray {
                            value: current_value.clone(),
                        })?
                        .get(*index)
                        .ok_or(FieldAccessError::FieldNotFound {
                            field_name: self.input_field.clone(),
                        })?;
                }
            }
        }

        let Value::String(name) = current_value else {
            return Err(FieldAccessError::FieldNotString {
                field_value: current_value.clone(),
            }
            .into());
        };
        Ok(name)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn default_field_accessor() {
        let built = FieldAccessorConfig(".domain_name".to_string())
            .build()
            .expect("Failed building default field accessor");
        let object = json!({
            "domain_name": "value"
        });

        assert_eq!(
            "value",
            built
                .access_field(&object)
                .expect("Failed accessing default field")
        );
    }

    #[test]
    fn default_field_accessor_without_dot() {
        let built = FieldAccessorConfig("domain_name".to_string())
            .build()
            .expect("Failed building field accessor without dot prefix");
        let object = json!({
            "domain_name": "value"
        });

        assert_eq!(
            "value",
            built
                .access_field(&object)
                .expect("Failed accessing field without dot prefix")
        );
    }

    #[test]
    fn indexing_accessor() {
        let built = FieldAccessorConfig("domain_names[0]".to_string())
            .build()
            .expect("Failed building field accessor with indexing");
        let object = json!({
            "domain_names": ["value"]
        });

        assert_eq!(
            "value",
            built
                .access_field(&object)
                .expect("Failed accessing field with indexing")
        );
    }

    #[test]
    fn nested_case() {
        let built = FieldAccessorConfig(".values.domain_name".to_string())
            .build()
            .expect("Failed building nested field accessor");
        let object = json!({
            "values": {
                "domain_name": "value",
                "other": "test"
            },
            "metadata": {}
        });

        assert_eq!(
            "value",
            built
                .access_field(&object)
                .expect("Failed accessing nested field")
        );
    }

    #[test]
    fn complex_case() {
        let built = FieldAccessorConfig(".values.array[0].domain_name".to_string())
            .build()
            .expect("Failed building nested field accessor");
        let object = json!({
            "values": {
                "array": [
                    {
                        "domain_name": "value",
                    },
                    {
                        "domain_name": "different_value",
                    },
                ],
                "other": "test"
            },
            "metadata": {}
        });

        assert_eq!(
            "value",
            built
                .access_field(&object)
                .expect("Failed accessing nested field")
        );
    }

    #[test]
    fn invalid_dots_error() {
        FieldAccessorConfig("...".to_string())
            .build()
            .expect_err("Expected error for just dots config");
    }

    #[test]
    fn invalid_indexing_error() {
        FieldAccessorConfig(".values[3.x".to_string())
            .build()
            .expect_err("Expected error for invalid indexing");
    }

    #[test]
    fn missing_dot_after_indexing_error() {
        FieldAccessorConfig(".values[3]domain_name".to_string())
            .build()
            .expect_err("Expected error for invalid indexing");
    }

    #[test]
    fn indexing_right_after_dot_error() {
        FieldAccessorConfig(".values.[3].domain_name".to_string())
            .build()
            .expect_err("Expected error for invalid indexing");
    }

    #[test]
    fn start_with_indexing_error() {
        FieldAccessorConfig("[3].values.domain_name".to_string())
            .build()
            .expect_err("Expected error for invalid indexing");
    }
}
