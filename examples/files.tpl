List of files in directories
{{foreach path in sourcesList}}
 - {{path}}
{{endfor}}

{{foreach file in files(source: sourcesList, recursive: true, exclude_paths: ["target", ".git"])}}

--- File: {{file.path}} ---
{{file.content}}
--- End of file: {{file.name}} ---

{{endfor}}