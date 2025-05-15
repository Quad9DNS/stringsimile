use serde_json::Value;

#[derive(Clone)]
pub struct StringsimileMessage {
    original_input: String,
    parsed: Option<Value>,
}

#[allow(unused)]
impl StringsimileMessage {
    pub fn new_parsed(original_input: String, value: Value) -> Self {
        Self {
            original_input,
            parsed: Some(value),
        }
    }

    pub fn new_unparsed(original_input: String) -> Self {
        Self {
            original_input,
            parsed: None,
        }
    }

    pub fn from_parts(original_input: String, parsed: Option<Value>) -> StringsimileMessage {
        Self {
            original_input,
            parsed,
        }
    }

    pub fn original_input(&self) -> &str {
        &self.original_input
    }

    pub fn into_original_input(self) -> String {
        self.original_input
    }

    pub fn parsed_value(&self) -> Option<&Value> {
        self.parsed.as_ref()
    }

    pub fn into_parsed_value(self) -> Option<Value> {
        self.parsed
    }

    pub fn into_parts(self) -> (String, Option<Value>) {
        (self.original_input, self.parsed)
    }
}
