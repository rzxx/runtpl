use std::io;

#[derive(Debug)]
pub enum AppError {
    InvalidArgument(String),
    Io(io::Error),
    // Здесь будут другие типы ошибок, например, для парсинга шаблонов
}

impl From<io::Error> for AppError {
    fn from(err: io::Error) -> Self {
        AppError::Io(err)
    }
}

// Реализации для красивого отображения ошибок
impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AppError::InvalidArgument(msg) => write!(f, "Invalid argument: {}", msg),
            AppError::Io(err) => write!(f, "IO Error: {}", err),
        }
    }
}

impl std::error::Error for AppError {}
