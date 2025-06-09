use crate::builtin_fns;
use crate::context::Context;
use itertools::Itertools;
use lazy_static::lazy_static;
use regex::{Captures, Regex};
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};

// Тип для указателя на встроенную функцию
type BuiltInFns = fn(&Map<String, Value>) -> Result<Value, Value>;

// --- Регулярные выражения (без изменений) ---
lazy_static! {
    static ref RE_VAR: Regex = Regex::new(r"\{\{\s*([a-zA-Z0-9_.]+)\s*\}\}").unwrap();
    // Обновляем RE_FOREACH, чтобы он мог распознавать `function(...)`
    static ref RE_FOREACH: Regex = Regex::new(
        r"(?m)(^\s*)\{\{foreach\s+([a-zA-Z0-9_]+)\s+in\s+([a-zA-Z0-9_.]+)(?:\(([^)]*)\))?\s*\}\}\s*?\r?\n?"
    ).unwrap();
    static ref RE_ENDFOR: Regex = Regex::new(r"(?m)(^\s*)\{\{endfor\}\}\s*?\r?\n?").unwrap();
}

lazy_static! {
    static ref BUILTIN_FNS: HashMap<&'static str, BuiltInFns> = {
        let mut m = HashMap::new();
        m.insert("files", builtin_fns::files as BuiltInFns);
        m
    };
    static ref RESERVED_WORDS: HashSet<&'static str> = {
        let mut s = HashSet::new();
        s.insert("endfor");
        s.insert("in");
        // 'foreach' не нужно, т.к. RE_VAR его не поймает
        s
    };
}

// --- Хелперы (resolve_path и value_to_string без изменений) ---
fn resolve_path<'a>(context: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = context;
    for key in path.split('.') {
        current = current.get(key)?;
    }
    Some(current)
}

fn value_to_string(value: &Value) -> String {
    if let Some(s) = value.as_str() {
        s.to_string()
    } else {
        value.to_string()
    }
}

// Пытается распознать значение: сначала как JSON, потом как переменную контекста
fn resolve_arg_value(val_str: &str, context: &Value) -> Result<Value, String> {
    let trimmed = val_str.trim();
    // 1. Попробовать как литерал JSON (e.g., "string", true, 123, ["a", "b"])
    if let Ok(json_val) = serde_json::from_str(trimmed) {
        return Ok(json_val);
    }
    // 2. Если не получилось, попробовать как переменную из контекста
    if let Some(context_val) = resolve_path(context, trimmed) {
        return Ok(context_val.clone());
    }
    // 3. Ошибка
    Err(format!(
        "Argument value '{}' is not a valid JSON literal nor a known variable.",
        trimmed
    ))
}

// Парсит строку вида `key1: value1, key2: value2` в Map
fn parse_function_args(args_str: &str, context: &Value) -> Result<Map<String, Value>, String> {
    let mut args_map = Map::new();
    if args_str.trim().is_empty() {
        return Ok(args_map);
    }

    // Split only on commas that are not inside brackets or quotes
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut bracket_level = 0;
    let mut in_quotes = false;
    for c in args_str.chars() {
        match c {
            '"' => {
                in_quotes = !in_quotes;
                current.push(c);
            }
            '[' | '{' if !in_quotes => {
                bracket_level += 1;
                current.push(c);
            }
            ']' | '}' if !in_quotes => {
                bracket_level -= 1;
                current.push(c);
            }
            ',' if !in_quotes && bracket_level == 0 => {
                parts.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(c),
        }
    }
    if !current.trim().is_empty() {
        parts.push(current.trim().to_string());
    }

    for part in parts {
        let mut kv = part.splitn(2, ':');
        let key = kv
            .next()
            .ok_or_else(|| format!("Invalid argument part: '{}'", part))?
            .trim();
        let val_str = kv
            .next()
            .ok_or_else(|| format!("Argument '{}' is missing a value", key))?
            .trim();

        // Always try to parse as JSON first
        let value = resolve_arg_value(val_str, context)?;
        args_map.insert(key.to_string(), value);
    }
    Ok(args_map)
}

// --- render_variables (теперь принимает &Value, а не &Context) ---
fn render_variables(template: &str, context: &Value) -> String {
    RE_VAR
        .replace_all(template, |caps: &Captures| {
            let path = &caps[1];
            resolve_path(context, path)
                .map(value_to_string)
                .unwrap_or_default()
        })
        .into_owned()
}

/// НОВЫЙ, БОЛЕЕ МОЩНЫЙ ENUM ДЛЯ ОПИСАНИЯ ПЕРЕМЕННЫХ
#[derive(Debug, Clone, PartialEq)]
pub enum VarUsage {
    /// Простая переменная: {{ var }} -> "..."
    Simple,
    /// Массив простых значений: {{ foreach item in my_list }} {{ item }} {{ endfor }} -> [...]
    CollectionOfSimple,
    /// Массив объектов: {{ foreach item in my_list }} {{ item.name }} {{ endfor }}
    /// Хранит структуру объекта.
    CollectionOfObjects(HashMap<String, VarUsage>),
}

fn analyze_object_structure(
    loop_body: &str,
    item_var: &str,
    all_loop_vars: &HashSet<String>, // Все переменные цикла из всего шаблона
) -> HashMap<String, VarUsage> {
    let mut structure = HashMap::new();

    // 1. Ищем вложенные циклы, источником которых является свойство нашего объекта.
    // Пример: {{ foreach member in team.members }}
    for caps in RE_FOREACH.captures_iter(loop_body) {
        let source_path = &caps[3];
        // Проверяем, что источник - это свойство нашего item_var (e.g., "team.members")
        if let Some(prop_name) = source_path.strip_prefix(&format!("{}.", item_var)) {
            let inner_loop_item_var = &caps[2];
            let inner_loop_body = find_loop_body(loop_body, &caps[0]);

            // Рекурсивно анализируем структуру вложенного объекта
            let sub_structure =
                analyze_object_structure(&inner_loop_body, inner_loop_item_var, all_loop_vars);

            let usage = if sub_structure.is_empty() {
                VarUsage::CollectionOfSimple
            } else {
                VarUsage::CollectionOfObjects(sub_structure)
            };
            structure.insert(prop_name.to_string(), usage);
        }
    }

    // 2. Ищем простые свойства объекта.
    // Пример: {{ team.name }}
    for caps in RE_VAR.captures_iter(loop_body) {
        let path = &caps[1];
        if let Some(prop_name) = path.strip_prefix(&format!("{}.", item_var)) {
            // Убедимся, что это не свойство какой-то другой, более вложенной переменной.
            // Например, в {{ team.manager.name }} свойство 'manager' относится к 'team',
            // а 'name' - к 'manager', а не к 'team'. Мы берем только первый сегмент.
            if let Some(first_prop) = prop_name.split('.').next() {
                structure
                    .entry(first_prop.to_string())
                    .or_insert(VarUsage::Simple);
            }
        }
    }

    structure
}

/// Хелпер для поиска тела цикла по его открывающему тегу.
/// Нужен, чтобы не дублировать код поиска `endfor`.
fn find_loop_body(template_chunk: &str, start_tag: &str) -> String {
    if let Some(start_match) = RE_FOREACH.find(template_chunk) {
        if start_match.as_str() != start_tag {
            // Это не тот цикл, который мы ищем
            return "".to_string();
        }

        let search_start_pos = start_match.end();
        let mut nesting_level = 0;

        for (offset, tag_type) in RE_FOREACH
            .find_iter(&template_chunk[search_start_pos..])
            .map(|m| (m.start(), "start"))
            .chain(
                RE_ENDFOR
                    .find_iter(&template_chunk[search_start_pos..])
                    .map(|m| (m.start(), "end")),
            )
            .sorted_by_key(|(offset, _)| *offset)
        {
            if tag_type == "start" {
                nesting_level += 1;
            } else if nesting_level == 0 {
                let end_pos = search_start_pos + offset;
                return template_chunk[search_start_pos..end_pos].to_string();
            } else {
                nesting_level -= 1;
            }
        }
    }
    "".to_string()
}

/// Точка входа, переписанная для использования нового подхода.
pub fn extract_variables(template: &str) -> HashMap<String, VarUsage> {
    let mut variables = HashMap::new();

    // ШАГ 1: Найти ВСЕ переменные, объявленные в циклах, чтобы игнорировать их глобально.
    let all_loop_vars: HashSet<String> = RE_FOREACH
        .captures_iter(template)
        .map(|caps| caps[2].to_string())
        .collect();

    // ШАГ 2: Обработать циклы верхнего уровня.
    for caps in RE_FOREACH.captures_iter(template) {
        let source_path = &caps[3];
        if let Some(base_var) = source_path.split('.').next() {
            // Пропускаем, если источник - это свойство другой переменной цикла
            if all_loop_vars.contains(base_var) {
                continue;
            }

            // Пропускаем встроенные функции
            let is_function_call = caps.get(4).is_some();
            if is_function_call || BUILTIN_FNS.contains_key(source_path) {
                continue;
            }

            let item_var = &caps[2];
            let loop_body = find_loop_body(template, caps.get(0).unwrap().as_str());

            // Используем новую функцию для анализа структуры
            let structure = analyze_object_structure(&loop_body, item_var, &all_loop_vars);

            let usage = if structure.is_empty() {
                VarUsage::CollectionOfSimple
            } else {
                VarUsage::CollectionOfObjects(structure)
            };
            variables.insert(base_var.to_string(), usage);
        }
    }

    // ШАГ 3: Найти простые переменные верхнего уровня.
    for caps in RE_VAR.captures_iter(template) {
        if let Some(base_var) = caps[1].split('.').next() {
            if !RESERVED_WORDS.contains(base_var) && !all_loop_vars.contains(base_var) {
                variables
                    .entry(base_var.to_string())
                    .or_insert(VarUsage::Simple);
            }
        }
    }

    variables
}

// --- Главная функция render (новая рекурсивная реализация) ---
pub fn render(template: &str, context: &Context) -> Result<String, String> {
    let context_value = Value::Object(context.0.clone().into_iter().collect());
    render_recursive(template, &context_value)
}

// Рекурсивный движок
fn render_recursive(template: &str, context: &Value) -> Result<String, String> {
    // 1. Ищем самый первый (внешний) блок foreach
    if let Some(start_match) = RE_FOREACH.find(template) {
        // Умный поиск `endfor` (ваш код - он идеален)
        let search_start_pos = start_match.end();
        let mut nesting_level = 0;
        let mut end_match_pos = None;

        // Используем itertools.sorted_by_key для более чистого кода
        for (offset, tag_type) in RE_FOREACH
            .find_iter(&template[search_start_pos..])
            .map(|m| (m.start(), "start"))
            .chain(
                RE_ENDFOR
                    .find_iter(&template[search_start_pos..])
                    .map(|m| (m.start(), "end")),
            )
            .sorted_by_key(|(offset, _)| *offset)
        {
            if tag_type == "start" {
                nesting_level += 1;
            } else if nesting_level == 0 {
                end_match_pos = Some(search_start_pos + offset);
                break;
            } else {
                nesting_level -= 1;
            }
        }

        if let Some(end_pos) = end_match_pos {
            let end_match = RE_ENDFOR.find_at(template, end_pos).unwrap();

            // 2. Разделяем шаблон на три части
            let before_loop = &template[..start_match.start()];
            let loop_body_template = &template[start_match.end()..end_match.start()];
            let after_loop = &template[end_match.end()..];

            // 3. Рендерим каждую часть
            let rendered_before = render_recursive(before_loop, context)?;

            let caps = RE_FOREACH.captures(start_match.as_str()).unwrap();
            let item_name = &caps[2];
            let source_name = &caps[3];
            let args_str_opt = caps.get(4).map(|m| m.as_str());

            let collection_val = if let Some(args_str) = args_str_opt {
                // Это вызов функции
                let func = BUILTIN_FNS
                    .get(source_name)
                    .ok_or_else(|| format!("Unknown function '{}'", source_name))?;

                let args_map = parse_function_args(args_str, context)?;

                func(&args_map).map_err(|e| {
                    format!(
                        "Error in function '{}': {}",
                        source_name,
                        value_to_string(&e)
                    )
                })?
            } else {
                // Это обычная переменная
                resolve_path(context, source_name)
                    .cloned()
                    .unwrap_or(Value::Array(vec![])) // Если переменной нет, считаем ее пустым массивом
            };

            let mut rendered_loop_body = String::new();
            let items_to_iterate = match collection_val {
                // Если это уже массив, используем его как есть.
                Value::Array(arr) => arr,
                // Если это ЛЮБОЕ другое значение (String, Number, Bool, Object),
                // создаем массив из этого одного элемента.
                single_val => vec![single_val],
            };

            for item in items_to_iterate {
                if let Some(mut new_context_obj) = context.as_object().cloned() {
                    new_context_obj.insert(item_name.to_string(), item.clone());
                    let new_context_val = Value::Object(new_context_obj);
                    rendered_loop_body
                        .push_str(&render_recursive(loop_body_template, &new_context_val)?);
                }
            }

            let rendered_after = render_recursive(after_loop, context)?;

            return Ok(format!(
                "{}{}{}",
                rendered_before, rendered_loop_body, rendered_after
            ));
        }
    }

    // 5. Базовый случай рекурсии: в шаблоне больше нет `foreach`.
    //    Осталось только заменить переменные.
    Ok(render_variables(template, context))
}
