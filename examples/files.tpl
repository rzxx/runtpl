Список файлов в директориях
{{foreach path in sourcesList}}
 - {{path}}
{{endfor}}

{{foreach file in files(source: sourcesList, recursive: true, exclude_paths: ["target", ".git"])}}

--- Файл: {{file.path}} ---
{{file.content}}
--- Конец файла: {{file.name}} ---

{{endfor}}