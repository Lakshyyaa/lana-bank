use anyhow::anyhow;
use colored::*;
use serde_json::Value;
use similar::{ChangeTag, TextDiff};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
};

use super::SchemaInfo;

#[derive(Clone)]
pub struct SchemaChangeInfo {
    pub schema_info: SchemaInfo,
    pub previous_schema: Option<Value>,
    pub current_schema: Value,
    pub has_changed: bool,
}

pub fn detect_schema_changes(
    schemas: &[SchemaInfo],
    schemas_out_dir: &str,
) -> anyhow::Result<Vec<SchemaChangeInfo>> {
    let schemas_dir = Path::new(schemas_out_dir);
    let mut schema_changes = Vec::new();

    for schema_info in schemas {
        let filepath = schemas_dir.join(schema_info.filename);
        let new_schema = (schema_info.generate_schema)();

        let (previous_schema, has_changed) = if filepath.exists() {
            let existing_content = fs::read_to_string(&filepath)?;
            let existing_schema: serde_json::Value = serde_json::from_str(&existing_content)?;

            let changed = existing_schema != new_schema;
            (Some(existing_schema), changed)
        } else {
            (None, true) // New schema is always considered a change
        };

        schema_changes.push(SchemaChangeInfo {
            schema_info: schema_info.clone(),
            previous_schema,
            current_schema: new_schema,
            has_changed,
        });
    }

    Ok(schema_changes)
}

pub fn process_schemas_with_changes(
    schema_changes: &[SchemaChangeInfo],
    schemas_out_dir: &str,
) -> anyhow::Result<Vec<SchemaChangeInfo>> {
    let schemas_dir = Path::new(schemas_out_dir);
    if !schemas_dir.exists() {
        fs::create_dir_all(schemas_dir)?;
    }

    let mut has_breaking_changes = false;
    let mut processed_changes = Vec::new();

    for change in schema_changes {
        let filepath = schemas_dir.join(change.schema_info.filename);
        let new_schema_pretty = serde_json::to_string_pretty(&change.current_schema)?;

        if change.has_changed {
            if let Some(ref previous_schema) = change.previous_schema {
                println!(
                    "{} {}",
                    "Schema changed:".yellow().bold(),
                    change.schema_info.name
                );

                // Show diff
                let existing_content = serde_json::to_string_pretty(previous_schema)?;
                show_diff(&existing_content, &new_schema_pretty);

                // Check for breaking changes
                if is_breaking_change(previous_schema, &change.current_schema)? {
                    println!(
                        "{} Breaking change detected in {}",
                        "❌".red(),
                        change.schema_info.name.red().bold()
                    );
                    has_breaking_changes = true;
                } else {
                    println!(
                        "{} Non-breaking change in {}",
                        "✅".green(),
                        change.schema_info.name.green()
                    );
                }
            } else {
                println!(
                    "{} Creating new schema: {}",
                    "📝".blue(),
                    change.schema_info.name
                );
            }
        } else {
            println!("{} {} (no changes)", "✅".green(), change.schema_info.name);
        }

        processed_changes.push(change.clone());

        // Write the new schema
        fs::write(&filepath, format!("{new_schema_pretty}\n"))?;
    }

    if has_breaking_changes {
        println!("\n{} Breaking changes detected!", "❌".red().bold());
        std::process::exit(1);
    } else {
        println!(
            "\n{} All schemas updated successfully!",
            "✅".green().bold()
        );
    }

    Ok(processed_changes)
}

pub fn show_diff(old_content: &str, new_content: &str) {
    let diff = TextDiff::from_lines(old_content, new_content);

    for change in diff.iter_all_changes() {
        let sign = match change.tag() {
            ChangeTag::Delete => "-".red(),
            ChangeTag::Insert => "+".green(),
            ChangeTag::Equal => " ".normal(),
        };

        let line = change.value().trim_end_matches('\n');
        match change.tag() {
            ChangeTag::Delete => println!("{} {}", sign, line.red()),
            ChangeTag::Insert => println!("{} {}", sign, line.green()),
            ChangeTag::Equal => {
                // Skip showing unchanged lines for brevity unless they're context lines
                // Only show context lines near changes
                continue;
            }
        }
    }
    println!();
}

#[derive(Clone)]
struct SchemaContext<'a> {
    definitions: HashMap<String, &'a Value>,
    _root: &'a Value,
}

impl<'a> SchemaContext<'a> {
    fn new(schema: &'a Value) -> anyhow::Result<Self> {
        let mut definitions = HashMap::new();

        // Extract definitions from root level
        if let Some(Value::Object(defs)) = schema.get("definitions") {
            for (name, def) in defs {
                definitions.insert(format!("#/definitions/{name}"), def);
            }
        }

        // Also check for $defs (draft 2019-09+)
        if let Some(Value::Object(defs)) = schema.get("$defs") {
            for (name, def) in defs {
                definitions.insert(format!("#/$defs/{name}"), def);
            }
        }

        Ok(SchemaContext {
            definitions,
            _root: schema,
        })
    }

    fn resolve_ref(&self, reference: &str) -> anyhow::Result<&'a Value> {
        self.definitions
            .get(reference)
            .copied()
            .ok_or_else(|| anyhow!("Reference {} not found", reference))
    }
}

pub fn is_breaking_change(old_schema: &Value, new_schema: &Value) -> anyhow::Result<bool> {
    let old_ctx = SchemaContext::new(old_schema)?;
    let new_ctx = SchemaContext::new(new_schema)?;

    is_breaking_change_with_context(&old_ctx, old_schema, &new_ctx, new_schema)
}

fn is_breaking_change_with_context(
    old_ctx: &SchemaContext,
    old_schema: &Value,
    new_ctx: &SchemaContext,
    new_schema: &Value,
) -> anyhow::Result<bool> {
    // Handle $ref
    if let Some(Value::String(old_ref)) = old_schema.get("$ref") {
        let old_resolved = old_ctx.resolve_ref(old_ref)?;
        if let Some(Value::String(new_ref)) = new_schema.get("$ref") {
            let new_resolved = new_ctx.resolve_ref(new_ref)?;
            return is_breaking_change_with_context(old_ctx, old_resolved, new_ctx, new_resolved);
        } else {
            // Old had ref, new doesn't - need to compare resolved old with new
            return is_breaking_change_with_context(old_ctx, old_resolved, new_ctx, new_schema);
        }
    } else if let Some(Value::String(new_ref)) = new_schema.get("$ref") {
        // Old didn't have ref, new does - compare old with resolved new
        let new_resolved = new_ctx.resolve_ref(new_ref)?;
        return is_breaking_change_with_context(old_ctx, old_schema, new_ctx, new_resolved);
    }

    // Handle oneOf, anyOf, allOf
    if old_schema.get("oneOf").is_some() || new_schema.get("oneOf").is_some() {
        return is_breaking_oneof_change(old_ctx, old_schema, new_ctx, new_schema);
    }

    if old_schema.get("anyOf").is_some() || new_schema.get("anyOf").is_some() {
        return is_breaking_anyof_change(old_ctx, old_schema, new_ctx, new_schema);
    }

    if old_schema.get("allOf").is_some() || new_schema.get("allOf").is_some() {
        return is_breaking_allof_change(old_ctx, old_schema, new_ctx, new_schema);
    }

    // Check basic schema properties
    let old_obj = old_schema.as_object();
    let new_obj = new_schema.as_object();

    if old_obj.is_none() || new_obj.is_none() {
        // If either is not an object, do simple equality check
        return Ok(old_schema != new_schema);
    }

    let old_obj = old_obj.unwrap();
    let new_obj = new_obj.unwrap();

    // Check type changes
    if let (Some(old_type), Some(new_type)) = (old_obj.get("type"), new_obj.get("type")) {
        if old_type != new_type {
            return Ok(true);
        }
    }

    // Check all the basic constraints
    if is_breaking_required_change(old_obj, new_obj)? {
        return Ok(true);
    }

    if is_breaking_properties_change_with_context(old_ctx, old_obj, new_ctx, new_obj)? {
        return Ok(true);
    }

    if is_breaking_enum_change(old_obj, new_obj)? {
        return Ok(true);
    }

    if is_breaking_additional_properties_change(old_obj, new_obj)? {
        return Ok(true);
    }

    if is_breaking_numeric_constraints_change(old_obj, new_obj)? {
        return Ok(true);
    }

    if is_breaking_string_constraints_change(old_obj, new_obj)? {
        return Ok(true);
    }

    if is_breaking_array_constraints_change(old_obj, new_obj)? {
        return Ok(true);
    }

    Ok(false)
}

fn is_breaking_oneof_change(
    old_ctx: &SchemaContext,
    old_schema: &Value,
    new_ctx: &SchemaContext,
    new_schema: &Value,
) -> anyhow::Result<bool> {
    match (old_schema.get("oneOf"), new_schema.get("oneOf")) {
        (Some(Value::Array(old_options)), Some(Value::Array(new_options))) => {
            // For oneOf, removing any option is breaking
            // We need to check if all old options are still valid in the new schema
            for old_option in old_options {
                let mut found_compatible = false;
                for new_option in new_options {
                    // Check if old option is compatible with at least one new option
                    if !is_breaking_change_with_context(old_ctx, old_option, new_ctx, new_option)? {
                        found_compatible = true;
                        break;
                    }
                }
                if !found_compatible {
                    return Ok(true); // Old option has no compatible new option
                }
            }
            Ok(false)
        }
        (Some(_), None) => Ok(true), // Removed oneOf constraint
        (None, Some(_)) => Ok(true), // Added oneOf constraint
        _ => Ok(false),
    }
}

fn is_breaking_anyof_change(
    old_ctx: &SchemaContext,
    old_schema: &Value,
    new_ctx: &SchemaContext,
    new_schema: &Value,
) -> anyhow::Result<bool> {
    match (old_schema.get("anyOf"), new_schema.get("anyOf")) {
        (Some(Value::Array(old_options)), Some(Value::Array(new_options))) => {
            // For anyOf, we need at least one compatible path for each old valid data
            // Similar logic to oneOf
            for old_option in old_options {
                let mut found_compatible = false;
                for new_option in new_options {
                    if !is_breaking_change_with_context(old_ctx, old_option, new_ctx, new_option)? {
                        found_compatible = true;
                        break;
                    }
                }
                if !found_compatible {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        (Some(_), None) => Ok(true),
        (None, Some(_)) => Ok(true),
        _ => Ok(false),
    }
}

fn is_breaking_allof_change(
    old_ctx: &SchemaContext,
    old_schema: &Value,
    new_ctx: &SchemaContext,
    new_schema: &Value,
) -> anyhow::Result<bool> {
    match (old_schema.get("allOf"), new_schema.get("allOf")) {
        (Some(Value::Array(old_all)), Some(Value::Array(new_all))) => {
            // For allOf, adding new constraints is breaking
            if new_all.len() > old_all.len() {
                return Ok(true);
            }

            // Check if any existing constraint became more restrictive
            for (old_constraint, new_constraint) in old_all.iter().zip(new_all.iter()) {
                if is_breaking_change_with_context(
                    old_ctx,
                    old_constraint,
                    new_ctx,
                    new_constraint,
                )? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        (Some(_), None) => Ok(false), // Removing allOf constraints is not breaking
        (None, Some(_)) => Ok(true),  // Adding allOf constraints is breaking
        _ => Ok(false),
    }
}

fn is_breaking_properties_change_with_context(
    old_ctx: &SchemaContext,
    old_obj: &serde_json::Map<String, Value>,
    new_ctx: &SchemaContext,
    new_obj: &serde_json::Map<String, Value>,
) -> anyhow::Result<bool> {
    if let (Some(Value::Object(old_props)), Some(Value::Object(new_props))) =
        (old_obj.get("properties"), new_obj.get("properties"))
    {
        // Check for removed properties
        for (prop_name, _) in old_props {
            if !new_props.contains_key(prop_name) {
                return Ok(true);
            }
        }

        // Check for breaking changes in existing properties
        for (prop_name, old_prop_schema) in old_props {
            if let Some(new_prop_schema) = new_props.get(prop_name) {
                if is_breaking_change_with_context(
                    old_ctx,
                    old_prop_schema,
                    new_ctx,
                    new_prop_schema,
                )? {
                    return Ok(true);
                }
            }
        }
    }

    Ok(false)
}

// Keep all the helper functions from the original implementation
fn is_breaking_required_change(
    old_obj: &serde_json::Map<String, Value>,
    new_obj: &serde_json::Map<String, Value>,
) -> anyhow::Result<bool> {
    let old_required = get_string_array(old_obj.get("required"))?;
    let new_required = get_string_array(new_obj.get("required"))?;

    for field in &new_required {
        if !old_required.contains(field) {
            return Ok(true);
        }
    }

    Ok(false)
}

fn is_breaking_enum_change(
    old_obj: &serde_json::Map<String, Value>,
    new_obj: &serde_json::Map<String, Value>,
) -> anyhow::Result<bool> {
    if let (Some(Value::Array(old_enum)), Some(Value::Array(new_enum))) =
        (old_obj.get("enum"), new_obj.get("enum"))
    {
        let old_set: HashSet<&Value> = old_enum.iter().collect();
        let new_set: HashSet<&Value> = new_enum.iter().collect();

        for value in &old_set {
            if !new_set.contains(value) {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

fn is_breaking_additional_properties_change(
    old_obj: &serde_json::Map<String, Value>,
    new_obj: &serde_json::Map<String, Value>,
) -> anyhow::Result<bool> {
    match (
        old_obj.get("additionalProperties"),
        new_obj.get("additionalProperties"),
    ) {
        (Some(Value::Bool(true)), Some(Value::Bool(false))) => Ok(true),
        (None, Some(Value::Bool(false))) => Ok(true),
        _ => Ok(false),
    }
}

fn is_breaking_numeric_constraints_change(
    old_obj: &serde_json::Map<String, Value>,
    new_obj: &serde_json::Map<String, Value>,
) -> anyhow::Result<bool> {
    if let (Some(old_min), Some(new_min)) = (
        get_number(old_obj.get("minimum")),
        get_number(new_obj.get("minimum")),
    ) {
        if new_min > old_min {
            return Ok(true);
        }
    }

    if let (Some(old_max), Some(new_max)) = (
        get_number(old_obj.get("maximum")),
        get_number(new_obj.get("maximum")),
    ) {
        if new_max < old_max {
            return Ok(true);
        }
    }

    if let (Some(old_min), Some(new_min)) = (
        get_number(old_obj.get("exclusiveMinimum")),
        get_number(new_obj.get("exclusiveMinimum")),
    ) {
        if new_min > old_min {
            return Ok(true);
        }
    }

    if let (Some(old_max), Some(new_max)) = (
        get_number(old_obj.get("exclusiveMaximum")),
        get_number(new_obj.get("exclusiveMaximum")),
    ) {
        if new_max < old_max {
            return Ok(true);
        }
    }

    Ok(false)
}

fn is_breaking_string_constraints_change(
    old_obj: &serde_json::Map<String, Value>,
    new_obj: &serde_json::Map<String, Value>,
) -> anyhow::Result<bool> {
    if let (Some(old_min), Some(new_min)) = (
        get_u64(old_obj.get("minLength")),
        get_u64(new_obj.get("minLength")),
    ) {
        if new_min > old_min {
            return Ok(true);
        }
    }

    if let (Some(old_max), Some(new_max)) = (
        get_u64(old_obj.get("maxLength")),
        get_u64(new_obj.get("maxLength")),
    ) {
        if new_max < old_max {
            return Ok(true);
        }
    }

    match (old_obj.get("pattern"), new_obj.get("pattern")) {
        (None, Some(_)) => return Ok(true),
        (Some(old_pattern), Some(new_pattern)) if old_pattern != new_pattern => {
            return Ok(true);
        }
        _ => {}
    }

    Ok(false)
}

fn is_breaking_array_constraints_change(
    old_obj: &serde_json::Map<String, Value>,
    new_obj: &serde_json::Map<String, Value>,
) -> anyhow::Result<bool> {
    if let (Some(old_min), Some(new_min)) = (
        get_u64(old_obj.get("minItems")),
        get_u64(new_obj.get("minItems")),
    ) {
        if new_min > old_min {
            return Ok(true);
        }
    }

    if let (Some(old_max), Some(new_max)) = (
        get_u64(old_obj.get("maxItems")),
        get_u64(new_obj.get("maxItems")),
    ) {
        if new_max < old_max {
            return Ok(true);
        }
    }

    match (old_obj.get("uniqueItems"), new_obj.get("uniqueItems")) {
        (Some(Value::Bool(false)), Some(Value::Bool(true))) => return Ok(true),
        (None, Some(Value::Bool(true))) => return Ok(true),
        _ => {}
    }

    Ok(false)
}

fn get_string_array(value: Option<&Value>) -> anyhow::Result<Vec<String>> {
    match value {
        Some(Value::Array(arr)) => arr
            .iter()
            .map(|v| {
                v.as_str()
                    .map(String::from)
                    .ok_or_else(|| anyhow!("Invalid string in array"))
            })
            .collect(),
        None => Ok(Vec::new()),
        _ => Err(anyhow!("Expected array")),
    }
}

fn get_number(value: Option<&Value>) -> Option<f64> {
    value?.as_f64()
}

fn get_u64(value: Option<&Value>) -> Option<u64> {
    value?.as_u64()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_nested_required_field_change() -> anyhow::Result<()> {
        let old_schema = json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "definitions": {
                "AuditInfo": {
                    "properties": {
                        "audit_entry_id": {
                            "type": "integer"
                        },
                        "sub": {
                            "type": "string"
                        }
                    },
                    "required": ["audit_entry_id"],
                    "type": "object"
                }
            },
            "properties": {
                "audit_info": {
                    "$ref": "#/definitions/AuditInfo"
                }
            }
        });

        let new_schema = json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "definitions": {
                "AuditInfo": {
                    "properties": {
                        "audit_entry_id": {
                            "type": "integer"
                        },
                        "sub": {
                            "type": "string"
                        }
                    },
                    "required": ["audit_entry_id", "sub"], // Added required field
                    "type": "object"
                }
            },
            "properties": {
                "audit_info": {
                    "$ref": "#/definitions/AuditInfo"
                }
            }
        });

        assert!(is_breaking_change(&old_schema, &new_schema)?);
        Ok(())
    }

    #[test]
    fn test_oneof_enum_change() -> anyhow::Result<()> {
        let old_schema = json!({
            "oneOf": [
                {
                    "properties": {
                        "type": {
                            "enum": ["initialized"],
                            "type": "string"
                        }
                    },
                    "required": ["type"],
                    "type": "object"
                },
                {
                    "properties": {
                        "type": {
                            "enum": ["updated"],
                            "type": "string"
                        }
                    },
                    "required": ["type"],
                    "type": "object"
                }
            ]
        });

        let new_schema = json!({
            "oneOf": [
                {
                    "properties": {
                        "type": {
                            "enum": ["initialized"],
                            "type": "string"
                        }
                    },
                    "required": ["type"],
                    "type": "object"
                }
                // Removed the "updated" option
            ]
        });

        assert!(is_breaking_change(&old_schema, &new_schema)?);
        Ok(())
    }

    #[test]
    fn test_complex_user_event_schema() -> anyhow::Result<()> {
        let schema = json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "definitions": {
                "AuditEntryId": {
                    "format": "int64",
                    "type": "integer"
                },
                "AuditInfo": {
                    "properties": {
                        "audit_entry_id": {
                            "$ref": "#/definitions/AuditEntryId"
                        },
                        "sub": {
                            "type": "string"
                        }
                    },
                    "required": [
                        "audit_entry_id",
                        "sub"
                    ],
                    "type": "object"
                }
            },
            "oneOf": [
                {
                    "properties": {
                        "audit_info": {
                            "$ref": "#/definitions/AuditInfo"
                        },
                        "type": {
                            "enum": ["initialized"],
                            "type": "string"
                        }
                    },
                    "required": ["audit_info", "type"],
                    "type": "object"
                }
            ],
            "title": "UserEvent"
        });

        // Schema compared with itself should not have breaking changes
        assert!(!is_breaking_change(&schema, &schema)?);
        Ok(())
    }
}
