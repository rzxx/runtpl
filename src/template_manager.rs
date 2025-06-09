use crate::error::AppError;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

const TEMPLATE_EXTENSION: &str = "tpl";

/// Returns the path to the central template storage directory.
/// Creates the directory if it doesn't exist.
fn get_template_dir() -> Result<PathBuf, AppError> {
    let config_dir = dirs::config_dir().ok_or_else(|| {
        AppError::Editor("Could not find a valid configuration directory.".to_string())
    })?;
    let app_dir = config_dir.join("runtpl");
    let templates_dir = app_dir.join("templates");

    if !templates_dir.exists() {
        fs::create_dir_all(&templates_dir)?;
    }

    Ok(templates_dir)
}

/// Constructs the full path for a named template in the central store.
fn get_template_path(name: &str) -> Result<PathBuf, AppError> {
    let dir = get_template_dir()?;
    Ok(dir.join(format!("{}.{}", name, TEMPLATE_EXTENSION)))
}

/// Resolves a template name to a file path.
/// 1. Checks for a local file with the given name.
/// 2. Checks for a global template in the central store.
pub fn resolve_template_path(name: &str) -> Result<PathBuf, AppError> {
    let local_path = Path::new(name);
    if local_path.exists() {
        return Ok(local_path.to_path_buf());
    }

    let global_path = get_template_path(name)?;
    if global_path.exists() {
        return Ok(global_path);
    }

    Err(AppError::InvalidArgument(format!(
        "Template '{}' not found locally or in the global template directory ({}).",
        name,
        get_template_dir()?.display()
    )))
}

/// Handles the `template list` command.
pub fn list_templates() -> Result<(), AppError> {
    let dir = get_template_dir()?;
    println!("Available templates in {}:", dir.display());

    let entries: Vec<_> = fs::read_dir(dir)?.collect();

    if entries.is_empty() {
        println!("  (No templates found. Use 'runtpl template new <name>' to create one.)");
        return Ok(());
    }

    for entry in entries {
        let path = entry?.path();
        if path.is_file() {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                println!("- {}", stem);
            }
        }
    }
    Ok(())
}

/// Handles the `template new <name>` command.
pub fn new_template(name: &str) -> Result<(), AppError> {
    let path = get_template_path(name)?;
    if path.exists() {
        return Err(AppError::InvalidArgument(format!(
            "Template '{}' already exists. Use 'runtpl template edit {}' to edit it.",
            name, name
        )));
    }

    fs::File::create(&path)?;

    println!("Opening editor for new template: {}", path.display());
    edit::edit_file(&path).map_err(|e| AppError::Editor(e.to_string()))?;

    if fs::metadata(&path)?.len() == 0 {
        fs::remove_file(&path)?;
        println!("Empty template discarded. Creation cancelled.");
    } else {
        println!("Template '{}' created successfully.", name);
    }

    Ok(())
}

/// Handles the `template edit <name>` command.
pub fn edit_template(name: &str) -> Result<(), AppError> {
    let path = get_template_path(name)?;
    if !path.exists() {
        return Err(AppError::InvalidArgument(format!(
            "Template '{}' not found. Use 'runtpl template new {}' to create it.",
            name, name
        )));
    }

    println!("Opening editor for template: {}", path.display());
    edit::edit_file(&path).map_err(|e| AppError::Editor(e.to_string()))?;
    println!("Template '{}' saved.", name);
    Ok(())
}

/// Handles the `template remove <name>` command.
pub fn remove_template(name: &str) -> Result<(), AppError> {
    let path = get_template_path(name)?;
    if !path.exists() {
        return Err(AppError::InvalidArgument(format!(
            "Template '{}' not found. Use 'runtpl template list' to see available templates.",
            name
        )));
    }

    print!(
        "Are you sure you want to delete the template '{}' from {}? [y/N]: ",
        name,
        path.display()
    );
    io::stdout().flush()?;

    let mut confirmation = String::new();
    io::stdin().read_line(&mut confirmation)?;

    if confirmation.trim().to_lowercase() != "y" {
        println!("Removal cancelled.");
        return Ok(());
    }

    fs::remove_file(&path)?;

    println!("Template '{}' removed successfully.", name);
    Ok(())
}
