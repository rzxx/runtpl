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
        } => run_command(template_name, args, interactive),
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
        Ok(result) => print!("{}", result),
        Err(e) => eprintln!("Error rendering template: {}", e),
    }

    Ok(())
}

/// ОБНОВЛЕННАЯ ФУНКЦИЯ: Обрабатывает логику интерактивного режима.
fn run_interactive_mode(template_content: &str) -> Result<Context, AppError> {
    println!("Interactive mode activated. Analyzing template...");

    // 1. Извлекаем переменные и их тип использования
    let variables = engine::extract_variables(template_content);

    if variables.is_empty() {
        println!("No variables found in the template. Nothing to fill.");
        return Ok(Context::default());
    }

    // 2. Создаем "умный" JSON-объект с разными значениями по умолчанию
    let mut data_map = Map::new();
    println!("Please fill in the following variables in the editor:");
    for (var, usage) in &variables {
        println!(
            "- {} (as a {})",
            var,
            if *usage == VarUsage::Collection {
                "list"
            } else {
                "value"
            }
        );
        let default_value = match usage {
            VarUsage::Collection => Value::Array(vec![]),
            VarUsage::Simple => Value::String("".to_string()),
        };
        data_map.insert(var.clone(), default_value);
    }

    // Добавляем полезный комментарий в начало JSON
    let mut scaffold_map = Map::new();
    scaffold_map.insert(
        "__comment".to_string(),
        Value::String(
            "Please fill in the values. Use [] for lists/arrays and \"...\" for strings."
                .to_string(),
        ),
    );
    scaffold_map.append(&mut data_map);

    let initial_json = serde_json::to_string_pretty(&scaffold_map)
        .map_err(|e| AppError::JsonParse(format!("Failed to create JSON scaffold: {}", e)))?;

    // 3. Создаем временный файл и записываем в него JSON
    let mut file = tempfile::Builder::new()
        .prefix("template-vars-")
        .suffix(".json")
        .tempfile()?;

    file.write_all(initial_json.as_bytes())?;
    let path = file.path().to_path_buf();

    // 4. Открываем файл в редакторе
    println!("\nOpening editor: {}", path.display());
    edit::edit_file(&path).map_err(|e| AppError::Editor(e.to_string()))?;

    // 5. Читаем данные, введенные пользователем
    let user_data = fs::read_to_string(&path)?;

    // 6. ПРОВЕРКА НА ИЗМЕНЕНИЯ!
    // Мы нормализуем окончания строк, чтобы избежать ложных срабатываний (CRLF vs LF)
    if initial_json.replace("\r\n", "\n") == user_data.replace("\r\n", "\n") {
        return Err(AppError::InteractiveAbort(
            "No changes detected. Aborting.".to_string(),
        ));
    }

    println!("Editor closed. Reading data...");

    // 7. Парсим JSON и создаем контекст
    Context::from_interactive_json(&user_data)
}
