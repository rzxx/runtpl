mod builtin_fns;
mod cli;
mod context;
mod engine;
mod error;
mod template_manager;

use clap::Parser;
use cli::{Cli, Commands, TemplateCommands};
use context::Context;
use engine::VarUsage;
use error::AppError;
use serde_json::{Map, Value};
use std::fs;
use std::io::Write;

fn main() -> Result<(), ()> {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Run {
            template_name,
            args,
            interactive,
            no_copy,
        } => run_command(template_name, args, interactive, no_copy),
        Commands::Template { command } => match command {
            TemplateCommands::List => template_manager::list_templates(),
            TemplateCommands::New { name } => template_manager::new_template(&name),
            TemplateCommands::Edit { name } => template_manager::edit_template(&name),
            TemplateCommands::Remove { name } => template_manager::remove_template(&name),
        },
    };

    if let Err(AppError::InteractiveAbort(msg)) = result {
        println!("{}", msg);
        return Ok(());
    }

    if let Err(e) = result {
        eprintln!("\x1b[31;1mError:\x1b[0m {}", e);
        std::process::exit(1);
    }

    Ok(())
}

fn run_command(
    template_name: String,
    args: Vec<String>,
    interactive: bool,
    no_copy: bool,
) -> Result<(), AppError> {
    let template_path = template_manager::resolve_template_path(&template_name)?;
    let template_content = fs::read_to_string(&template_path)?;

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
            print!("{}", result);

            if !no_copy {
                match arboard::Clipboard::new() {
                    Ok(mut clipboard) => {
                        if let Err(e) = clipboard.set_text(result) {
                            eprintln!("\n\nWarning: Could not copy to clipboard: {}", e);
                        } else {
                            eprintln!("\n\n(Result copied to clipboard)");
                        }
                    }
                    Err(e) => {
                        eprintln!("\n\nWarning: Could not access clipboard: {}", e);
                    }
                }
            }
        }
        Err(e) => eprintln!("Error rendering template: {}", e),
    }

    Ok(())
}

fn build_json_value(usage: &VarUsage) -> Value {
    match usage {
        VarUsage::Simple => Value::String("".into()),
        VarUsage::CollectionOfSimple => Value::Array(vec![]),
        VarUsage::CollectionOfObjects(structure) => {
            let mut object_scaffold = Map::new();
            for (key, inner_usage) in structure {
                object_scaffold.insert(key.clone(), build_json_value(inner_usage));
            }

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

    let mut data_map = Map::new();
    println!("Please fill in the following variables in the editor:");
    for (var, usage) in &variables {
        println!("- {}", var);
        data_map.insert(var.clone(), build_json_value(usage));
    }

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
