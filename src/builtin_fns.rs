use serde_json::{Map, Value};
use std::fs;
use walkdir::WalkDir;

/// Обрабатывает ошибку, возвращая её в виде `Err(Value::String(...))`
macro_rules! func_err {
    ($($arg:tt)*) => {
        return Err(Value::String(format!($($arg)*)))
    };
}

/// Встроенная функция `files(source, recursive, exclude_names, exclude_paths)`
/// Возвращает массив объектов, где каждый объект {name, path, content}.
pub fn files(args: &Map<String, Value>) -> Result<Value, Value> {
    // 1. Извлекаем и валидируем аргументы
    let source_paths: Vec<String> = match args.get("source") {
        // Случай 1: Пришел массив строк ["./src", "./tests"]
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        // Случай 2: Пришла строка "./src, ./tests"
        Some(Value::String(s)) => s
            .split(',')
            .map(|p| p.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        _ => func_err!(
            "'files' function requires a 'source' argument. It must be a comma-separated string (e.g., \"./src\") or an array of strings (e.g., [\"./src\", \"./tests\"])"
        ),
    };

    let recursive = match args.get("recursive") {
        Some(Value::Bool(b)) => *b,
        None => true, // По умолчанию рекурсивный обход включен
        _ => func_err!("'recursive' argument must be a boolean (true or false)"),
    };

    let exclude_names: Vec<String> = match args.get("exclude_names") {
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        None => vec![],
        _ => func_err!("'exclude_names' argument must be an array of strings"),
    };

    let exclude_paths: Vec<String> = match args.get("exclude_paths") {
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        None => vec![],
        _ => func_err!("'exclude_paths' argument must be an array of strings"),
    };

    // 2. Основная логика: обход директорий
    let mut result_files = Vec::new();

    for path in source_paths {
        let walker = WalkDir::new(&path).into_iter();

        for entry_result in walker {
            let entry = match entry_result {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("Warning: Skipping path due to error: {}", e);
                    continue;
                }
            };

            // Пропускаем директории
            if entry.file_type().is_dir() {
                // Если не рекурсивно, запрещаем входить в поддиректории
                if !recursive && entry.depth() > 0 {
                    // WalkDir iterator will skip the contents of this directory
                }
                continue;
            }

            let file_path = entry.path();
            let file_path_str = file_path.to_string_lossy();
            let file_name_str = file_path.file_name().unwrap_or_default().to_string_lossy();

            // 3. Применяем фильтры
            if exclude_names.iter().any(|name| *name == file_name_str) {
                continue;
            }
            if exclude_paths.iter().any(|p| file_path_str.contains(p)) {
                continue;
            }

            // 4. Читаем файл и создаем объект
            match fs::read_to_string(file_path) {
                Ok(content) => {
                    let mut file_obj = Map::new();
                    file_obj.insert("name".to_string(), Value::String(file_name_str.to_string()));
                    file_obj.insert("path".to_string(), Value::String(file_path_str.to_string()));
                    file_obj.insert("content".to_string(), Value::String(content));
                    result_files.push(Value::Object(file_obj));
                }
                Err(e) => {
                    eprintln!("Warning: Could not read file {}: {}", file_path_str, e);
                }
            }
        }
    }

    Ok(Value::Array(result_files))
}
