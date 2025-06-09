mod builtin_fns;
mod cli;
mod context;
mod engine;
mod error;

use clap::Parser;
use cli::{Cli, Commands};
use context::Context;
use error::AppError;
use std::fs;

fn main() -> Result<(), AppError> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Run {
            template_name,
            args,
            interactive,
        } => {
            if interactive {
                println!("Interactive mode is selected. (Not implemented yet)");
                // Здесь будет логика для интерактивного режима
            } else {
                let context = Context::from_args(&args)?;
                let template_content = fs::read_to_string(&template_name)?;
                let output = engine::render(&template_content, &context);

                match output {
                    Ok(result) => print!("{}", result),
                    Err(e) => eprintln!("Error rendering template: {}", e),
                }
            }
        }
        Commands::Template { command } => {
            println!(
                "Template command selected: {:?} (Not implemented yet)",
                command
            );
        }
    }

    Ok(())
}
