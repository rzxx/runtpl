# runtpl (Run Template)

A powerful command-line interface (CLI) tool for automating the creation of AI prompts, code snippets, configuration files, and more, using a flexible templating engine. Define your templates once, then dynamically generate content by injecting data from various sources.

`runtpl` streamlines repetitive text generation tasks, making it ideal for developers, AI engineers, and anyone who frequently generates structured text.

## Features

* **Flexible Templating:** Supports variables (`{{ var }}`) and powerful `foreach` loops for iterating over collections, including nested structures.
* **Built-in Functions:** Includes functions like `files()` to automatically read and embed file contents into your templates.
* **Multiple Data Input Methods:**
  * **CLI Arguments:** Pass key-value pairs directly (e.g., `key=value`, `key=item1,item2`).
  * **File Input:** Load data from local files (e.g., `key@=path/to/data.json`).
  * **Stdin Input:** Pipe data from standard input (e.g., `cat data.json | runtpl run my_template data@-`).
* **Interactive Mode:** Automatically extracts variables from your template and opens an editor to prompt for values in a structured JSON format.
* **Clipboard Integration:** Automatically copies the rendered output to your system clipboard (configurable).
* **Template Management:** Commands to list, create, edit, and remove templates stored globally.
* **Cross-Platform:** Built with Rust for speed and reliability.

## Installation

### Prerequisites

* [Rust programming language](https://www.rust-lang.org/tools/install) (Rustup is recommended).

### Install from Cargo

```bash
cargo install runtpl
```

This will compile `runtpl` and place the executable in your Cargo bin directory (usually `~/.cargo/bin`), which should be in your system's PATH.

## Usage

### `runtpl run <template_name> [args...]`

Renders a template with provided data.

* `<template_name>`: The name of a template stored globally (e.g., `my_prompt`) or a path to a local template file (e.g., `./templates/local_template.tpl`).
* `[args...]`: Data arguments to pass to the template.

#### Data Argument Formats

1. **`key=value`**: Simple key-value pair. If `value` contains commas, it will be parsed as a comma-separated array of strings.
    * Example: `name=Alice`, `tags=rust,cli,tool`
2. **`key@=filepath`**: Reads the content of `filepath` and assigns it to `key`. The content will be parsed as JSON if valid, otherwise as a plain string.
    * Example: `code@=src/main.rs`, `config@=config.json`
3. **`key@-`**: Reads the content from standard input (`stdin`) and assigns it to `key`. The content will be parsed as JSON if valid, otherwise as a plain string. Only one `key@-` argument is allowed per run.
    * Example: `cat my_data.json | runtpl run my_template data@-`

#### Options

* `-i`, `--interactive`: Enter interactive mode. `runtpl` will analyze the template, create a JSON scaffold of expected variables, open your default editor for you to fill them, and then render the template with the provided data. Cannot be used with `[args...]`.
* `-n`, `--no-copy`: Do not copy the rendered output to the system clipboard. By default, output is copied.

#### Examples

```bash
# Basic usage with direct arguments
runtpl run my_prompt name=John description="a powerful CLI tool"

# Passing data from a JSON file
runtpl run generate_config settings@=app_settings.json

# Passing data via stdin
echo '{"message": "Hello from stdin!"}' | runtpl run simple_template data@-

# Using interactive mode to fill variables
runtpl run complex_ai_prompt --interactive

# Render without copying to clipboard
runtpl run my_template var=value --no-copy
```

### `runtpl template <command>`

Manages your globally stored templates.

* Templates are stored in your configuration directory:
  * **Linux:** `~/.config/runtpl/templates/`
  * **macOS:** `~/Library/Application Support/runtpl/templates/`
  * **Windows:** `%APPDATA%\runtpl\templates\`

#### Commands

* **`list`**: Lists all available templates in the global template directory.

    ```bash
    runtpl template list
    ```

* **`new <name>`**: Creates a new empty template file with the given name and opens it in your default editor. If the file is left empty, it will be discarded.

    ```bash
    runtpl template new my_ai_prompt
    ```

* **`edit <name>`**: Opens an existing template file in your default editor.

    ```bash
    runtpl template edit my_ai_prompt
    ```

* **`remove <name>`**: Deletes an existing template file after a confirmation prompt.

    ```bash
    runtpl template remove old_template
    ```

## Template Syntax

`runtpl` uses a simple, yet powerful, templating syntax inspired by popular templating engines.

### Variables

Variables are enclosed in double curly braces: `{{ variable_name }}`.
You can access nested properties using dot notation: `{{ object.property }}`.

```tpl
Hello, {{ name }}!
Your description: {{ project.description }}
```

### Loops (`foreach`)

The `foreach` block allows you to iterate over arrays or collections.

Syntax: `{{foreach item_variable in collection_source}} ... {{endfor}}`

* `item_variable`: The name of the variable that will hold the current item during iteration.
* `collection_source`: The name of the array variable or a built-in function call that returns a collection.

#### Iterating over simple lists

If `my_list` is `["apple", "banana", "cherry"]`:

```tpl
My favorite fruits:
{{foreach fruit in my_list}}
- {{ fruit }}
{{endfor}}
```

#### Iterating over objects in a list

If `teams` is `[{"name": "Alpha", "members": ["Alice", "Bob"]}, {"name": "Beta", "members": ["Charlie"]}]`:

```tpl
Project Report for {{ project_name }}:
{{foreach team in teams}}
Team: {{ team.name }}
  Members:
    {{foreach member in team.members}}
    - {{ member }}
    {{endfor}}
{{endfor}}
```

#### Iterating over built-in function results

The `files` built-in function (see below) returns a list of file objects.

```tpl
Files in the source directory:
{{foreach file in files(source: ["./src", "./docs"], recursive: true, exclude_paths: ["target", ".git"])}}

--- Path: {{file.path}} ---
Name: {{file.name}}
Content:
{{file.content}}
--- End File: {{file.name}} ---

{{endfor}}
```

## Built-in Functions

`runtpl` provides built-in functions that can be used as `collection_source` in `foreach` loops.

### `files(source, recursive, exclude_names, exclude_paths)`

Scans specified directories and returns an array of objects, where each object represents a file.

* **`source`** (required):
  * A string with comma-separated paths (e.g., `"./src,./tests"`).
  * An array of strings (e.g., `["./src", "./tests"]`).
* **`recursive`** (optional, boolean): If `true` (default), scans subdirectories. If `false`, only scans the top-level files in `source` directories.
* **`exclude_names`** (optional, array of strings): A list of file names to exclude (e.g., `["main.rs", "README.md"]`).
* **`exclude_paths`** (optional, array of strings): A list of path substrings to exclude. If a file's relative path contains any of these substrings, it will be excluded (e.g., `["target", ".git"]`).

Each file object returned by `files()` has the following properties:

* **`name`**: The file name (e.g., `main.rs`).
* **`path`**: The file's path relative to the current working directory (e.g., `src/main.rs`).
* **`absolute_path`**: The file's canonical absolute path (e.g., `/home/user/project/src/main.rs`).
* **`content`**: The full content of the file as a string.

#### Example Usage (within a template)

```tpl
// Get all .rs files in src, excluding target and .git directories

{{foreach item in files(source: "./src", recursive: true, exclude_paths: ["target", ".git"])}}
File: {{item.name}} ({{item.path}})
Content:

{{item.content}}

{{endfor}}
```

## Template Examples

For more detailed template examples, please refer to the `examples/` directory in the repository.

* `examples/files.tpl`: Demonstrates using the `files` built-in function.
* `examples/nested_loop.tpl`: Shows how to iterate over nested data structures.

## Contributing

Contributions are welcome! If you find a bug or have a feature request, please open an issue on the GitHub repository.

## License

This project is licensed under the MIT License. See the `LICENSE` file for details.
