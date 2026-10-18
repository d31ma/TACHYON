use serde_json::{Value, json};

pub(crate) struct CsharpTaskAnswer {
    reader: &'static str,
    item: String,
    shape: String,
    suffix: &'static str,
    models: Value,
}

impl CsharpTaskAnswer {
    const RECORD: &'static str = "ReadResult";
    const VALUE: &'static str = "ReadResultValue";
    const LIST: &'static str = "ReadResultList";
    const SUFFIXES: [&'static str; 2] = ["", ".ToArray()"];

    pub(crate) fn record(name: &str, models: Value) -> Self {
        Self {
            reader: Self::RECORD,
            item: format!("Models.{name}"),
            shape: name.to_owned(),
            suffix: Self::SUFFIXES[0],
            models,
        }
    }

    pub(crate) fn value(kind: &str, shape: &str) -> Self {
        Self {
            reader: Self::VALUE,
            item: kind.to_owned(),
            shape: shape.to_owned(),
            suffix: Self::SUFFIXES[0],
            models: json!({}),
        }
    }

    pub(crate) fn listed(self, array: bool) -> Self {
        Self {
            reader: Self::LIST,
            shape: format!("[]{}", self.shape),
            suffix: Self::SUFFIXES[usize::from(array)],
            ..self
        }
    }

    pub(crate) fn metadata(&self) -> Value {
        json!({
            "reader": format!("{}<{}>", self.reader, self.item),
            "shape": self.shape,
            "suffix": self.suffix,
            "models": self.models,
        })
    }
}
