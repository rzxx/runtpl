mod builtin_fns;
mod cli;
mod context;
mod engine;
mod error;

use clap::Parser;
use cli::{Cli, Commands};
use context::Context;
use engine::VarUsage;
use error::AppError;
use serde_json::{Map, Value};
use std::fs;
use std::io::Write;

fn main() -> Result<(), AppError> {
    let cli = Cli::parse();
    // Обернем логику в Result, чтобы красиво обработать ошибки
    let result = match cli.command {
        Commands::Run {
            template_name,
            args,
            interactive,
            no_copy,
        } => run_command(template_name, args, interactive, no_copy),
        Commands::Template { command } => {
            println!(
                "Template command selected: {:?} (Not implemented yet)",
                command
            );
            Ok(())
        }
    };

    // Специальная обработка ошибки InteractiveAbort
    if let Err(AppError::InteractiveAbort(msg)) = result {
        println!("{}", msg);
        return Ok(());
    }

    result
}

// Выносим логику команды в отдельную функцию
fn run_command(
    template_name: String,
    args: Vec<String>,
    interactive: bool,
    no_copy: bool,
) -> Result<(), AppError> {
    let template_content = fs::read_to_string(&template_name)?;

    let context = if interactive {
        if !args.is_empty() {
            return Err(AppError::InvalidArgument(
                "Cannot use data arguments with --interactive mode.".to_string(),
            ));
        }
        run_interactive_mode(&template_content)?
    } else {
        Context::from_args(&args)?
    };

    match engine::render(&template_content, &context) {
        Ok(result) => {
            // Печатаем результат в stdout в любом случае
            print!("{}", result);

            // Если флаг --no-copy НЕ был передан
            if !no_copy {
                // Пытаемся получить доступ к буферу обмена
                match arboard::Clipboard::new() {
                    Ok(mut clipboard) => {
                        // Пытаемся записать текст
                        if let Err(e) = clipboard.set_text(result) {
                            // Если не получилось, выводим предупреждение в stderr,
                            // но не прерываем программу с ошибкой.
                            // Это важно, т.к. копирование - второстепенная операция.
                            eprintln!("\n\nWarning: Could not copy to clipboard: {}", e);
                        } else {
                            // Сообщаем пользователю об успехе
                            eprintln!("\n\n(Result copied to clipboard)");
                        }
                    }
                    Err(e) => {
                        // То же самое, если буфер обмена недоступен (например, в CI/CD)
                        eprintln!("\n\nWarning: Could not access clipboard: {}", e);
                    }
                }
            }
        }
        Err(e) => eprintln!("Error rendering template: {}", e),
    }

    Ok(())
}

/// НОВАЯ РЕКУРСИВНАЯ ФУНКЦИЯ для построения JSON-значения
fn build_json_value(usage: &VarUsage) -> Value {
    match usage {
        VarUsage::Simple => Value::String("".into()),
        VarUsage::CollectionOfSimple => Value::Array(vec![]),
        VarUsage::CollectionOfObjects(structure) => {
            // Создаем один объект-пример на основе структуры
            let mut object_scaffold = Map::new();
            for (key, inner_usage) in structure {
                object_scaffold.insert(key.clone(), build_json_value(inner_usage));
            }
            // Помещаем этот объект в массив
            Value::Array(vec![Value::Object(object_scaffold)])
        }
    }
}

fn run_interactive_mode(template_content: &str) -> Result<Context, AppError> {
    println!("Interactive mode activated. Analyzing template...");

    let variables = engine::extract_variables(template_content);

    if variables.is_empty() {
        println!("No variables found in the template. Nothing to fill.");
        return Ok(Context::default());
    }

    // Создаем JSON, используя нашу новую рекурсивную функцию
    let mut data_map = Map::new();
    println!("Please fill in the following variables in the editor:");
    for (var, usage) in &variables {
        println!("- {}", var); // Упростили вывод, т.к. структура видна в JSON
        data_map.insert(var.clone(), build_json_value(usage));
    }

    // ... остальная часть функции (создание файла, вызов редактора, проверка на изменения)
    // остается БЕЗ ИЗМЕНЕНИЙ ...

    let mut scaffold_map = Map::new();
    scaffold_map.insert(
        "__comment".to_string(),
        Value::String(
            "Please fill in the values. An example structure is provided for lists of objects."
                .to_string(),
        ),
    );
    scaffold_map.append(&mut data_map);

    let initial_json = serde_json::to_string_pretty(&scaffold_map)
        .map_err(|e| AppError::JsonParse(format!("Failed to create JSON scaffold: {}", e)))?;

    let mut file = tempfile::Builder::new()
        .prefix("template-vars-")
        .suffix(".json")
        .tempfile()?;
    file.write_all(initial_json.as_bytes())?;
    let path = file.path().to_path_buf();

    println!("\nOpening editor: {}", path.display());
    edit::edit_file(&path).map_err(|e| AppError::Editor(e.to_string()))?;

    let user_data = fs::read_to_string(&path)?;

    if initial_json.replace("\r\n", "\n") == user_data.replace("\r\n", "\n") {
        return Err(AppError::InteractiveAbort(
            "No changes detected. Aborting.".to_string(),
        ));
    }

    println!("Editor closed. Reading data...");
    Context::from_interactive_json(&user_data)
}
