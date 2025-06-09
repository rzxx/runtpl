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
/// Возвращает массив объектов, где каждый объект {name, path, absolute_path, content}.
/// 'path' - относительный путь (к текущей рабочей директории), 'absolute_path' - канонический абсолютный путь.
pub fn files(args: &Map<String, Value>) -> Result<Value, Value> {
    let source_paths: Vec<String> = match args.get("source") {
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
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
        None => true,
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

    let mut result_files = Vec::new();

    for path in source_paths {
        let mut walker_builder = WalkDir::new(&path);
        if !recursive {
            walker_builder = walker_builder.max_depth(1);
        }
        let walker = walker_builder.into_iter();

        for entry_result in walker {
            let entry = match entry_result {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("Warning: Skipping path due to error: {}", e);
                    continue;
                }
            };

            if entry.file_type().is_dir() {
                continue;
            }

            let file_path = entry.path();

            let relative_path_str = file_path.to_string_lossy();
            let file_name_str = file_path.file_name().unwrap_or_default().to_string_lossy();

            if exclude_names.iter().any(|name| *name == file_name_str) {
                continue;
            }
            if exclude_paths.iter().any(|p| relative_path_str.contains(p)) {
                continue;
            }

            let absolute_path = match fs::canonicalize(file_path) {
                Ok(path) => path,
                Err(e) => {
                    eprintln!(
                        "Warning: Skipping file '{}' because its absolute path could not be determined: {}",
                        file_path.display(),
                        e
                    );
                    continue;
                }
            };
            let absolute_path_str = absolute_path.to_string_lossy();

            match fs::read_to_string(file_path) {
                Ok(content) => {
                    let mut file_obj = Map::new();
                    file_obj.insert("name".to_string(), Value::String(file_name_str.to_string()));

                    file_obj.insert(
                        "path".to_string(),
                        Value::String(relative_path_str.to_string()),
                    );

                    file_obj.insert(
                        "absolute_path".to_string(),
                        Value::String(absolute_path_str.to_string()),
                    );
                    file_obj.insert("content".to_string(), Value::String(content));
                    result_files.push(Value::Object(file_obj));
                }
                Err(e) => {
                    eprintln!("Warning: Could not read file {}: {}", relative_path_str, e);
                }
            }
        }
    }

    Ok(Value::Array(result_files))
}
