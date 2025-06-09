use crate::error::AppError;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Read};

#[derive(Debug, Default, Clone)]
pub struct Context(pub HashMap<String, Value>);

fn normalize_string(s: &str) -> String {
    s.strip_prefix('\u{feff}')
        .unwrap_or(s)
        .replace("\r\n", "\n")
}

impl Context {
    pub fn from_args(args: &[String]) -> Result<Self, AppError> {
        let mut context = Context::default();
        let mut stdin_used = false;

        for arg in args {
            if let Some(key) = arg.strip_suffix("@-") {
                if stdin_used { /* ... */ }
                let mut buffer = String::new();
                io::stdin().read_to_string(&mut buffer)?;
                let normalized = normalize_string(&buffer);
                let value =
                    serde_json::from_str(&normalized).unwrap_or_else(|_| Value::String(normalized));
                context.0.insert(key.to_string(), value);
                stdin_used = true;
            } else if let Some((key, path)) = arg.split_once("@=") {
                let content = fs::read_to_string(path)?;
                let normalized = normalize_string(&content);
                let value =
                    serde_json::from_str(&normalized).unwrap_or_else(|_| Value::String(normalized));
                context.0.insert(key.to_string(), value);
            } else if let Some((key, value_str)) = arg.split_once('=') {
                let normalized = normalize_string(value_str);

                if normalized.contains(',') {
                    let items: Vec<Value> = normalized
                        .split(',')
                        .map(|s| Value::String(s.trim().to_string()))
                        .collect();
                    context.0.insert(key.to_string(), Value::Array(items));
                } else {
                    context.0.insert(key.to_string(), Value::String(normalized));
                }
            } else {
                return Err(AppError::InvalidArgument(format!(
                    "Argument '{}' is not in a valid format (key=value, key@=filepath, or key@-)",
                    arg
                )));
            }
        }

        Ok(context)
    }

    pub fn from_interactive_json(json_str: &str) -> Result<Self, AppError> {
        let value: Value = serde_json::from_str(json_str)?;
        match value {
            Value::Object(map) => {
                let hash_map = map.into_iter().collect();
                Ok(Context(hash_map))
            }
            _ => Err(AppError::JsonParse(
                "Root of the data file must be a JSON object.".to_string(),
            )),
        }
    }
}
