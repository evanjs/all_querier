use nu_protocol::{LabeledError, Span, Value};

pub fn serde_json_to_nu_value(
    value: serde_json::Value,
    span: Span,
) -> anyhow::Result<Value> {
    match value {
        serde_json::Value::Null => Ok(Value::nothing(span)),
        serde_json::Value::Bool(value) => Ok(Value::bool(value, span)),
        serde_json::Value::Number(value) => {
            if let Some(value) = value.as_i64() {
                Ok(Value::int(value, span))
            } else if let Some(value) = value.as_u64() {
                let value = i64::try_from(value)?;
                Ok(Value::int(value, span))
            } else if let Some(value) = value.as_f64() {
                Ok(Value::float(value, span))
            } else {
                Ok(Value::nothing(span))
            }
        }
        serde_json::Value::String(value) => Ok(Value::string(value, span)),
        serde_json::Value::Array(values) => {
            let values = values
                .into_iter()
                .map(|value| serde_json_to_nu_value(value, span))
                .collect::<anyhow::Result<Vec<_>>>()?;

            Ok(Value::list(values, span))
        }
        serde_json::Value::Object(object) => {
            let record = object
                .into_iter()
                .map(|(key, value)| {
                    serde_json_to_nu_value(value, span)
                        .map(|value| (key, value))
                })
                .collect::<anyhow::Result<Vec<_>>>()?;

            Ok(Value::record(record.into_iter().collect(), span))
        }
    }
}

pub fn labeled_error(
    span: Span,
    message: impl Into<String>,
    error: impl std::fmt::Display,
) -> LabeledError {
    LabeledError::new(message.into())
        .with_label(error.to_string(), span)
}