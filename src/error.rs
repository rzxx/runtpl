use serde_json;
use std::io;

#[derive(Debug)]
pub enum AppError {
    InvalidArgument(String),
    Io(io::Error),
    Editor(String),
    JsonParse(String),
    // НОВЫЙ ВАРИАНТ
    // Используется для штатного прерывания, когда пользователь не внес изменений
    InteractiveAbort(String),
}

impl From<io::Error> for AppError {
    fn from(err: io::Error) -> Self {
        AppError::Io(err)
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::JsonParse(err.to_string())
    }
}

// Реализации для красивого отображения ошибок
impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AppError::InvalidArgument(msg) => write!(f, "Invalid argument: {}", msg),
            AppError::Io(err) => write!(f, "IO Error: {}", err),
            AppError::Editor(msg) => write!(f, "Editor Error: {}", msg),
            AppError::JsonParse(msg) => write!(f, "JSON Parse Error: {}", msg),
            // Для штатного прерывания мы не будем выводить префикс "Error:"
            AppError::InteractiveAbort(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for AppError {}
