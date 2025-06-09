use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(author, version, about = "A powerful CLI templating tool")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Renders a template with provided data
    #[command(name = "run")]
    Run {
        /// The name of the template file to use
        template_name: String,

        /// Data arguments in `key=value`, `key@=filepath`, or `key@-` format
        #[arg()]
        args: Vec<String>,

        /// Enter interactive mode to fill variables
        #[arg(short, long)]
        interactive: bool,

        /// Do not copy the output to the clipboard
        #[arg(short = 'n', long = "no-copy")]
        no_copy: bool,
    },
    /// Manage templates
    Template {
        #[command(subcommand)]
        command: TemplateCommands,
    },
}

#[derive(Subcommand, Debug)]
pub enum TemplateCommands {
    /// List available templates
    List,
    /// Create a new template file
    New { name: String },
    /// Edit an existing template
    Edit { name: String },
}
