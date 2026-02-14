use anyhow::{bail, Result};
use dialoguer::{Confirm, Input, Select};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RelationType {
    BelongsTo,
    HasMany,
    ManyToMany,
}

#[derive(Debug, Clone, Serialize)]
pub struct RelationDefinition {
    pub name: String,
    pub relation_type: RelationType,
    pub target_entity: String,
    pub fk_column: Option<String>,
    pub optional: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct EntityDefinition {
    pub name: String,
    pub fields: Vec<FieldDefinition>,
    pub relations: Vec<RelationDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ValidationRule {
    Min(u64),
    Max(u64),
    Email,
    Url,
    Regex(String),
    Required,
    Unique,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FieldVisibility {
    Public,             // Everyone can see
    Authenticated,      // Only logged-in users
    AdminOnly,          // Only admin role
    Roles(Vec<String>), // Specific roles
}

impl Default for FieldVisibility {
    fn default() -> Self {
        FieldVisibility::Public
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct FieldDefinition {
    pub name: String,
    pub field_type: FieldType,
    pub optional: bool,
    pub default: Option<String>,
    pub relation: Option<String>,
    #[serde(default)]
    pub validations: Vec<ValidationRule>,
    #[serde(default)]
    pub searchable: bool,
    #[serde(default)]
    pub visibility: FieldVisibility,
}

#[derive(Debug, Clone, Serialize)]
pub enum FieldType {
    String,
    Text,
    Bool,
    Int32,
    Int64,
    Float64,
    Decimal,
    Uuid,
    DateTime,
    Date,
    Json,
    Enum(Vec<String>),
    File,
    Image,
}

impl FieldType {
    pub fn to_rust(&self) -> &str {
        match self {
            FieldType::String => "String",
            FieldType::Text => "String",
            FieldType::Bool => "bool",
            FieldType::Int32 => "i32",
            FieldType::Int64 => "i64",
            FieldType::Float64 => "f64",
            FieldType::Decimal => "Decimal",
            FieldType::Uuid => "Uuid",
            FieldType::DateTime => "DateTimeWithTimeZone",
            FieldType::Date => "Date",
            FieldType::Json => "Json",
            FieldType::Enum(_) => "String",
            FieldType::File => "String",
            FieldType::Image => "String",
        }
    }

    pub fn to_typescript(&self) -> &str {
        match self {
            FieldType::String
            | FieldType::Text
            | FieldType::Uuid
            | FieldType::Enum(_)
            | FieldType::File
            | FieldType::Image => "string",
            FieldType::Bool => "boolean",
            FieldType::Int32 | FieldType::Int64 | FieldType::Float64 | FieldType::Decimal => {
                "number"
            }
            FieldType::DateTime | FieldType::Date => "string",
            FieldType::Json => "unknown",
        }
    }

    pub fn to_postgres(&self) -> &str {
        match self {
            FieldType::String => "VARCHAR(255)",
            FieldType::Text => "TEXT",
            FieldType::Bool => "BOOLEAN",
            FieldType::Int32 => "INTEGER",
            FieldType::Int64 => "BIGINT",
            FieldType::Float64 => "DOUBLE PRECISION",
            FieldType::Decimal => "DECIMAL",
            FieldType::Uuid => "UUID",
            FieldType::DateTime => "TIMESTAMPTZ",
            FieldType::Date => "DATE",
            FieldType::Json => "JSONB",
            FieldType::Enum(_) => "VARCHAR(255)",
            FieldType::File => "VARCHAR(512)",
            FieldType::Image => "VARCHAR(512)",
        }
    }

    pub fn to_sea_orm_column(&self) -> &str {
        match self {
            FieldType::String => "ColumnType::String(StringLen::N(255))",
            FieldType::Text => "ColumnType::Text",
            FieldType::Bool => "ColumnType::Boolean",
            FieldType::Int32 => "ColumnType::Integer",
            FieldType::Int64 => "ColumnType::BigInteger",
            FieldType::Float64 => "ColumnType::Double",
            FieldType::Decimal => "ColumnType::Decimal(None)",
            FieldType::Uuid => "ColumnType::Uuid",
            FieldType::DateTime => "ColumnType::TimestampWithTimeZone",
            FieldType::Date => "ColumnType::Date",
            FieldType::Json => "ColumnType::JsonBinary",
            FieldType::Enum(_) => "ColumnType::String(StringLen::N(255))",
            FieldType::File => "ColumnType::String(StringLen::N(512))",
            FieldType::Image => "ColumnType::String(StringLen::N(512))",
        }
    }

    /// Returns the SeaORM migration builder method chain for this type.
    pub fn to_sea_orm_migration(&self) -> &str {
        match self {
            FieldType::String => "string_len(255)",
            FieldType::Text => "text()",
            FieldType::Bool => "boolean()",
            FieldType::Int32 => "integer()",
            FieldType::Int64 => "big_integer()",
            FieldType::Float64 => "double()",
            FieldType::Decimal => "decimal()",
            FieldType::Uuid => "uuid()",
            FieldType::DateTime => "timestamp_with_time_zone()",
            FieldType::Date => "date()",
            FieldType::Json => "json_binary()",
            FieldType::Enum(_) => "string_len(255)",
            FieldType::File => "string_len(512)",
            FieldType::Image => "string_len(512)",
        }
    }

    pub fn to_shadcn(&self) -> &str {
        match self {
            FieldType::String | FieldType::Uuid => "Input",
            FieldType::Text => "Textarea",
            FieldType::Bool => "Switch",
            FieldType::Int32 | FieldType::Int64 | FieldType::Float64 | FieldType::Decimal => {
                "Input"
            }
            FieldType::DateTime | FieldType::Date => "Input",
            FieldType::Json => "Textarea",
            FieldType::Enum(_) => "Select",
            FieldType::File => "FileInput",
            FieldType::Image => "ImageInput",
        }
    }

    pub fn input_type(&self) -> &str {
        match self {
            FieldType::Int32 | FieldType::Int64 | FieldType::Float64 | FieldType::Decimal => {
                "number"
            }
            FieldType::Date => "date",
            FieldType::DateTime => "datetime-local",
            FieldType::File | FieldType::Image => "file",
            _ => "text",
        }
    }
}

fn parse_field_type(s: &str) -> Result<FieldType> {
    let lower = s.to_lowercase();
    if lower.starts_with("enum(") && lower.ends_with(')') {
        let inner = &s[5..s.len() - 1];
        let variants: Vec<String> = inner
            .split(',')
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .collect();
        if variants.is_empty() {
            bail!("Enum type requires at least one variant: enum(a,b,c)");
        }
        return Ok(FieldType::Enum(variants));
    }
    match lower.as_str() {
        "string" | "str" => Ok(FieldType::String),
        "text" => Ok(FieldType::Text),
        "bool" | "boolean" => Ok(FieldType::Bool),
        "i32" | "int" | "int32" | "integer" => Ok(FieldType::Int32),
        "i64" | "int64" | "bigint" => Ok(FieldType::Int64),
        "f64" | "float" | "float64" | "double" => Ok(FieldType::Float64),
        "decimal" | "money" => Ok(FieldType::Decimal),
        "uuid" => Ok(FieldType::Uuid),
        "datetime" | "timestamp" => Ok(FieldType::DateTime),
        "date" => Ok(FieldType::Date),
        "json" | "jsonb" => Ok(FieldType::Json),
        "file" => Ok(FieldType::File),
        "image" => Ok(FieldType::Image),
        _ => bail!("Unknown field type: '{}'", s),
    }
}

/// Parse validation rules from a bracket-enclosed string like `[min=3,max=100,email]`.
/// Visibility annotations (`admin_only`, `authenticated`, `roles=hr,admin`) are
/// skipped here and handled by `parse_visibility()` instead.
fn parse_validations(s: &str) -> Vec<ValidationRule> {
    let mut rules = Vec::new();

    for part in s.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        if let Some((key, value)) = part.split_once('=') {
            match key.trim() {
                "min" => {
                    if let Ok(n) = value.trim().parse::<u64>() {
                        rules.push(ValidationRule::Min(n));
                    }
                }
                "max" => {
                    if let Ok(n) = value.trim().parse::<u64>() {
                        rules.push(ValidationRule::Max(n));
                    }
                }
                "regex" => {
                    rules.push(ValidationRule::Regex(value.trim().to_string()));
                }
                "roles" => {} // Handled by parse_visibility
                _ => {}
            }
        } else {
            match part {
                "email" => rules.push(ValidationRule::Email),
                "url" => rules.push(ValidationRule::Url),
                "required" => rules.push(ValidationRule::Required),
                "unique" => rules.push(ValidationRule::Unique),
                "searchable" => {}       // Handled separately
                "admin_only" => {}       // Handled by parse_visibility
                "authenticated" => {}    // Handled by parse_visibility
                _ => {}
            }
        }
    }

    rules
}

/// Parse field visibility from bracket annotations.
///
/// Recognized annotations:
/// - `admin_only` → `FieldVisibility::AdminOnly`
/// - `authenticated` → `FieldVisibility::Authenticated`
/// - `roles=hr,admin` → `FieldVisibility::Roles(vec!["hr", "admin"])`
/// - (none of the above) → `FieldVisibility::Public`
fn parse_visibility(s: &str) -> FieldVisibility {
    for part in s.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        if part == "admin_only" {
            return FieldVisibility::AdminOnly;
        }
        if part == "authenticated" {
            return FieldVisibility::Authenticated;
        }

        if let Some((key, value)) = part.split_once('=') {
            if key.trim() == "roles" {
                let roles: Vec<String> = value
                    .split(';')
                    .map(|r| r.trim().to_string())
                    .filter(|r| !r.is_empty())
                    .collect();
                if !roles.is_empty() {
                    return FieldVisibility::Roles(roles);
                }
            }
        }
    }

    FieldVisibility::Public
}

/// Parse entity from CLI args.
/// Format: name:type or name:type->Relation or name:type?  (? = optional)
/// Validation: name:type[min=3,max=100]
/// Searchable: name:type[searchable]
/// Relation formats: name:has_many->Entity, name:m2m->Entity
pub fn parse_entity(name: &str, field_strs: &[String]) -> Result<EntityDefinition> {
    let mut fields = Vec::new();
    let mut relations = Vec::new();

    for field_str in field_strs {
        let (field_str, optional) = if field_str.ends_with('?') {
            (&field_str[..field_str.len() - 1], true)
        } else {
            (field_str.as_str(), false)
        };

        let parts: Vec<&str> = field_str.splitn(2, ':').collect();
        if parts.len() != 2 {
            bail!(
                "Invalid field format '{}'. Expected name:type",
                field_str
            );
        }

        let field_name = parts[0].to_string();

        // Extract annotations from brackets: type[annotations]
        let (type_and_relation_str, annotations) = if let Some(bracket_start) = parts[1].find('[') {
            if let Some(bracket_end) = parts[1].find(']') {
                let annotations_str = &parts[1][bracket_start + 1..bracket_end];
                let type_str = &parts[1][..bracket_start];
                // Append anything after ] (like ->Entity)
                let after_bracket = &parts[1][bracket_end + 1..];
                let full_type = format!("{}{}", type_str, after_bracket);
                (full_type, annotations_str.to_string())
            } else {
                (parts[1].to_string(), String::new())
            }
        } else {
            (parts[1].to_string(), String::new())
        };

        let type_and_relation: Vec<&str> = type_and_relation_str.splitn(2, "->").collect();
        let type_str = type_and_relation[0].to_lowercase();

        // Check for relation-only types (no column generated)
        match type_str.as_str() {
            "has_many" => {
                let target = type_and_relation
                    .get(1)
                    .ok_or_else(|| anyhow::anyhow!("has_many requires target entity: {}:has_many->Entity", field_name))?;
                relations.push(RelationDefinition {
                    name: field_name,
                    relation_type: RelationType::HasMany,
                    target_entity: target.to_string(),
                    fk_column: None,
                    optional: false,
                });
                continue;
            }
            "m2m" => {
                let target = type_and_relation
                    .get(1)
                    .ok_or_else(|| anyhow::anyhow!("m2m requires target entity: {}:m2m->Entity", field_name))?;
                relations.push(RelationDefinition {
                    name: field_name,
                    relation_type: RelationType::ManyToMany,
                    target_entity: target.to_string(),
                    fk_column: None,
                    optional: false,
                });
                continue;
            }
            _ => {}
        }

        let field_type = parse_field_type(type_and_relation[0])?;
        let relation = type_and_relation.get(1).map(|s| s.to_string());

        // Parse validations, searchable, and visibility from annotations
        let validations = parse_validations(&annotations);
        let searchable = annotations.contains("searchable");
        let visibility = parse_visibility(&annotations);

        // If field has a belongs_to relation, also add it to relations vec
        if let Some(ref target) = relation {
            relations.push(RelationDefinition {
                name: field_name.clone(),
                relation_type: RelationType::BelongsTo,
                target_entity: target.clone(),
                fk_column: Some(field_name.clone()),
                optional,
            });
        }

        fields.push(FieldDefinition {
            name: field_name,
            field_type,
            optional,
            default: None,
            relation,
            validations,
            searchable,
            visibility,
        });
    }

    Ok(EntityDefinition {
        name: name.to_string(),
        fields,
        relations,
    })
}

const FIELD_TYPE_OPTIONS: &[(&str, &str)] = &[
    ("string", "String (VARCHAR 255)"),
    ("text", "Text (unlimited)"),
    ("bool", "Boolean"),
    ("int", "Integer (i32)"),
    ("bigint", "Big Integer (i64)"),
    ("float", "Float (f64)"),
    ("decimal", "Decimal"),
    ("uuid", "UUID"),
    ("datetime", "DateTime (with timezone)"),
    ("date", "Date"),
    ("json", "JSON"),
    ("file", "File (upload)"),
    ("image", "Image (upload with validation)"),
    ("enum", "Enum (custom variants)"),
];

const RELATION_TYPE_OPTIONS: &[(&str, &str)] = &[
    ("belongs_to", "Belongs To (FK on this entity)"),
    ("has_many", "Has Many (reverse side)"),
    ("m2m", "Many to Many (junction table)"),
];

/// Interactively prompt the user to define entity fields and relations.
pub fn prompt_entity_fields(entity_name: &str) -> Result<(Vec<FieldDefinition>, Vec<RelationDefinition>)> {
    println!(
        "Define fields for '{}' (press Enter with empty name to finish):",
        entity_name
    );

    let type_labels: Vec<&str> = FIELD_TYPE_OPTIONS.iter().map(|(_, label)| *label).collect();
    let mut fields = Vec::new();
    let mut relations = Vec::new();

    loop {
        let field_name: String = Input::new()
            .with_prompt("Field name")
            .allow_empty(true)
            .interact_text()?;

        if field_name.is_empty() {
            break;
        }

        let type_idx = Select::new()
            .with_prompt("Field type")
            .items(&type_labels)
            .default(0)
            .interact()?;

        let optional = Confirm::new()
            .with_prompt("Optional (nullable)?")
            .default(false)
            .interact()?;

        let relation: String = Input::new()
            .with_prompt("Foreign key (entity name, or empty to skip)")
            .allow_empty(true)
            .interact_text()?;

        let (type_key, _) = FIELD_TYPE_OPTIONS[type_idx];
        let field_type = if type_key == "enum" {
            let variants_input: String = Input::new()
                .with_prompt("Enter enum variants (comma-separated)")
                .interact_text()?;
            let variants: Vec<String> = variants_input
                .split(',')
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
                .collect();
            if variants.is_empty() {
                println!("No variants provided, defaulting to String type.");
                FieldType::String
            } else {
                FieldType::Enum(variants)
            }
        } else {
            parse_field_type(type_key)?
        };
        let relation = if relation.is_empty() {
            None
        } else {
            Some(relation)
        };

        if let Some(ref target) = relation {
            relations.push(RelationDefinition {
                name: field_name.clone(),
                relation_type: RelationType::BelongsTo,
                target_entity: target.clone(),
                fk_column: Some(field_name.clone()),
                optional,
            });
        }

        fields.push(FieldDefinition {
            name: field_name,
            field_type,
            optional,
            default: None,
            relation,
            validations: Vec::new(),
            searchable: false,
            visibility: FieldVisibility::default(),
        });

        println!();
    }

    // Prompt for additional relations (has_many, m2m)
    let add_relations = Confirm::new()
        .with_prompt("Add relations (has_many, m2m)?")
        .default(false)
        .interact()?;

    if add_relations {
        let rel_labels: Vec<&str> = RELATION_TYPE_OPTIONS.iter().map(|(_, label)| *label).collect();

        loop {
            let rel_name: String = Input::new()
                .with_prompt("Relation name (empty to finish)")
                .allow_empty(true)
                .interact_text()?;

            if rel_name.is_empty() {
                break;
            }

            let rel_idx = Select::new()
                .with_prompt("Relation type")
                .items(&rel_labels)
                .default(0)
                .interact()?;

            let target: String = Input::new()
                .with_prompt("Target entity (PascalCase)")
                .interact_text()?;

            let (rel_key, _) = RELATION_TYPE_OPTIONS[rel_idx];
            let relation_type = match rel_key {
                "belongs_to" => RelationType::BelongsTo,
                "has_many" => RelationType::HasMany,
                "m2m" => RelationType::ManyToMany,
                _ => unreachable!(),
            };

            relations.push(RelationDefinition {
                name: rel_name,
                relation_type,
                target_entity: target,
                fk_column: None,
                optional: false,
            });

            println!();
        }
    }

    Ok((fields, relations))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_field_type ──────────────────────────────────────────────

    #[test]
    fn parse_string_aliases() {
        assert!(matches!(parse_field_type("string").unwrap(), FieldType::String));
        assert!(matches!(parse_field_type("str").unwrap(), FieldType::String));
        assert!(matches!(parse_field_type("STRING").unwrap(), FieldType::String));
    }

    #[test]
    fn parse_text() {
        assert!(matches!(parse_field_type("text").unwrap(), FieldType::Text));
    }

    #[test]
    fn parse_bool_aliases() {
        assert!(matches!(parse_field_type("bool").unwrap(), FieldType::Bool));
        assert!(matches!(parse_field_type("boolean").unwrap(), FieldType::Bool));
    }

    #[test]
    fn parse_int32_aliases() {
        assert!(matches!(parse_field_type("i32").unwrap(), FieldType::Int32));
        assert!(matches!(parse_field_type("int").unwrap(), FieldType::Int32));
        assert!(matches!(parse_field_type("int32").unwrap(), FieldType::Int32));
        assert!(matches!(parse_field_type("integer").unwrap(), FieldType::Int32));
    }

    #[test]
    fn parse_int64_aliases() {
        assert!(matches!(parse_field_type("i64").unwrap(), FieldType::Int64));
        assert!(matches!(parse_field_type("int64").unwrap(), FieldType::Int64));
        assert!(matches!(parse_field_type("bigint").unwrap(), FieldType::Int64));
    }

    #[test]
    fn parse_float64_aliases() {
        assert!(matches!(parse_field_type("f64").unwrap(), FieldType::Float64));
        assert!(matches!(parse_field_type("float").unwrap(), FieldType::Float64));
        assert!(matches!(parse_field_type("float64").unwrap(), FieldType::Float64));
        assert!(matches!(parse_field_type("double").unwrap(), FieldType::Float64));
    }

    #[test]
    fn parse_decimal_aliases() {
        assert!(matches!(parse_field_type("decimal").unwrap(), FieldType::Decimal));
        assert!(matches!(parse_field_type("money").unwrap(), FieldType::Decimal));
    }

    #[test]
    fn parse_uuid() {
        assert!(matches!(parse_field_type("uuid").unwrap(), FieldType::Uuid));
    }

    #[test]
    fn parse_datetime_aliases() {
        assert!(matches!(parse_field_type("datetime").unwrap(), FieldType::DateTime));
        assert!(matches!(parse_field_type("timestamp").unwrap(), FieldType::DateTime));
    }

    #[test]
    fn parse_date() {
        assert!(matches!(parse_field_type("date").unwrap(), FieldType::Date));
    }

    #[test]
    fn parse_json_aliases() {
        assert!(matches!(parse_field_type("json").unwrap(), FieldType::Json));
        assert!(matches!(parse_field_type("jsonb").unwrap(), FieldType::Json));
    }

    #[test]
    fn parse_file() {
        assert!(matches!(parse_field_type("file").unwrap(), FieldType::File));
    }

    #[test]
    fn parse_image() {
        assert!(matches!(parse_field_type("image").unwrap(), FieldType::Image));
    }

    #[test]
    fn parse_enum_type() {
        let ft = parse_field_type("enum(draft,published,archived)").unwrap();
        assert!(matches!(ft, FieldType::Enum(ref v) if v.len() == 3));
    }

    #[test]
    fn parse_enum_type_with_spaces() {
        let ft = parse_field_type("enum(a, b, c)").unwrap();
        assert!(matches!(ft, FieldType::Enum(ref v) if v.len() == 3 && v[0] == "a"));
    }

    #[test]
    fn parse_enum_empty_errors() {
        assert!(parse_field_type("enum()").is_err());
    }

    #[test]
    fn parse_enum_case_insensitive() {
        let ft = parse_field_type("Enum(Draft,Published)").unwrap();
        assert!(matches!(ft, FieldType::Enum(ref v) if v.len() == 2 && v[0] == "Draft"));
    }

    #[test]
    fn parse_unknown_type_errors() {
        assert!(parse_field_type("foobar").is_err());
    }

    // ── FieldType::to_rust ────────────────────────────────────────────

    #[test]
    fn to_rust_mappings() {
        assert_eq!(FieldType::String.to_rust(), "String");
        assert_eq!(FieldType::Text.to_rust(), "String");
        assert_eq!(FieldType::Bool.to_rust(), "bool");
        assert_eq!(FieldType::Int32.to_rust(), "i32");
        assert_eq!(FieldType::Int64.to_rust(), "i64");
        assert_eq!(FieldType::Float64.to_rust(), "f64");
        assert_eq!(FieldType::Decimal.to_rust(), "Decimal");
        assert_eq!(FieldType::Uuid.to_rust(), "Uuid");
        assert_eq!(FieldType::DateTime.to_rust(), "DateTimeWithTimeZone");
        assert_eq!(FieldType::Date.to_rust(), "Date");
        assert_eq!(FieldType::Json.to_rust(), "Json");
        assert_eq!(FieldType::Enum(vec!["A".into()]).to_rust(), "String");
        assert_eq!(FieldType::File.to_rust(), "String");
        assert_eq!(FieldType::Image.to_rust(), "String");
    }

    // ── FieldType::to_typescript ──────────────────────────────────────

    #[test]
    fn to_typescript_mappings() {
        assert_eq!(FieldType::String.to_typescript(), "string");
        assert_eq!(FieldType::Text.to_typescript(), "string");
        assert_eq!(FieldType::Bool.to_typescript(), "boolean");
        assert_eq!(FieldType::Int32.to_typescript(), "number");
        assert_eq!(FieldType::Int64.to_typescript(), "number");
        assert_eq!(FieldType::Float64.to_typescript(), "number");
        assert_eq!(FieldType::Decimal.to_typescript(), "number");
        assert_eq!(FieldType::Uuid.to_typescript(), "string");
        assert_eq!(FieldType::DateTime.to_typescript(), "string");
        assert_eq!(FieldType::Date.to_typescript(), "string");
        assert_eq!(FieldType::Json.to_typescript(), "unknown");
        assert_eq!(FieldType::Enum(vec![]).to_typescript(), "string");
        assert_eq!(FieldType::File.to_typescript(), "string");
        assert_eq!(FieldType::Image.to_typescript(), "string");
    }

    // ── FieldType::to_postgres ────────────────────────────────────────

    #[test]
    fn to_postgres_mappings() {
        assert_eq!(FieldType::String.to_postgres(), "VARCHAR(255)");
        assert_eq!(FieldType::Text.to_postgres(), "TEXT");
        assert_eq!(FieldType::Bool.to_postgres(), "BOOLEAN");
        assert_eq!(FieldType::Int32.to_postgres(), "INTEGER");
        assert_eq!(FieldType::Int64.to_postgres(), "BIGINT");
        assert_eq!(FieldType::Float64.to_postgres(), "DOUBLE PRECISION");
        assert_eq!(FieldType::Decimal.to_postgres(), "DECIMAL");
        assert_eq!(FieldType::Uuid.to_postgres(), "UUID");
        assert_eq!(FieldType::DateTime.to_postgres(), "TIMESTAMPTZ");
        assert_eq!(FieldType::Date.to_postgres(), "DATE");
        assert_eq!(FieldType::Json.to_postgres(), "JSONB");
        assert_eq!(FieldType::File.to_postgres(), "VARCHAR(512)");
        assert_eq!(FieldType::Image.to_postgres(), "VARCHAR(512)");
    }

    // ── FieldType::to_sea_orm_column ──────────────────────────────────

    #[test]
    fn to_sea_orm_column_mappings() {
        assert_eq!(FieldType::String.to_sea_orm_column(), "ColumnType::String(StringLen::N(255))");
        assert_eq!(FieldType::Text.to_sea_orm_column(), "ColumnType::Text");
        assert_eq!(FieldType::Bool.to_sea_orm_column(), "ColumnType::Boolean");
        assert_eq!(FieldType::Int32.to_sea_orm_column(), "ColumnType::Integer");
        assert_eq!(FieldType::Int64.to_sea_orm_column(), "ColumnType::BigInteger");
        assert_eq!(FieldType::Float64.to_sea_orm_column(), "ColumnType::Double");
        assert_eq!(FieldType::Decimal.to_sea_orm_column(), "ColumnType::Decimal(None)");
        assert_eq!(FieldType::Uuid.to_sea_orm_column(), "ColumnType::Uuid");
        assert_eq!(FieldType::DateTime.to_sea_orm_column(), "ColumnType::TimestampWithTimeZone");
        assert_eq!(FieldType::Date.to_sea_orm_column(), "ColumnType::Date");
        assert_eq!(FieldType::Json.to_sea_orm_column(), "ColumnType::JsonBinary");
        assert_eq!(FieldType::File.to_sea_orm_column(), "ColumnType::String(StringLen::N(512))");
        assert_eq!(FieldType::Image.to_sea_orm_column(), "ColumnType::String(StringLen::N(512))");
    }

    // ── FieldType::to_sea_orm_migration ───────────────────────────────

    #[test]
    fn to_sea_orm_migration_mappings() {
        assert_eq!(FieldType::String.to_sea_orm_migration(), "string_len(255)");
        assert_eq!(FieldType::Text.to_sea_orm_migration(), "text()");
        assert_eq!(FieldType::Bool.to_sea_orm_migration(), "boolean()");
        assert_eq!(FieldType::Int32.to_sea_orm_migration(), "integer()");
        assert_eq!(FieldType::Int64.to_sea_orm_migration(), "big_integer()");
        assert_eq!(FieldType::Float64.to_sea_orm_migration(), "double()");
        assert_eq!(FieldType::Decimal.to_sea_orm_migration(), "decimal()");
        assert_eq!(FieldType::Uuid.to_sea_orm_migration(), "uuid()");
        assert_eq!(FieldType::DateTime.to_sea_orm_migration(), "timestamp_with_time_zone()");
        assert_eq!(FieldType::Date.to_sea_orm_migration(), "date()");
        assert_eq!(FieldType::Json.to_sea_orm_migration(), "json_binary()");
        assert_eq!(FieldType::File.to_sea_orm_migration(), "string_len(512)");
        assert_eq!(FieldType::Image.to_sea_orm_migration(), "string_len(512)");
    }

    // ── FieldType::to_shadcn ──────────────────────────────────────────

    #[test]
    fn to_shadcn_mappings() {
        assert_eq!(FieldType::String.to_shadcn(), "Input");
        assert_eq!(FieldType::Text.to_shadcn(), "Textarea");
        assert_eq!(FieldType::Bool.to_shadcn(), "Switch");
        assert_eq!(FieldType::Int32.to_shadcn(), "Input");
        assert_eq!(FieldType::Uuid.to_shadcn(), "Input");
        assert_eq!(FieldType::DateTime.to_shadcn(), "Input");
        assert_eq!(FieldType::Json.to_shadcn(), "Textarea");
        assert_eq!(FieldType::Enum(vec!["A".into()]).to_shadcn(), "Select");
        assert_eq!(FieldType::File.to_shadcn(), "FileInput");
        assert_eq!(FieldType::Image.to_shadcn(), "ImageInput");
    }

    // ── FieldType::input_type ─────────────────────────────────────────

    #[test]
    fn input_type_mappings() {
        assert_eq!(FieldType::String.input_type(), "text");
        assert_eq!(FieldType::Int32.input_type(), "number");
        assert_eq!(FieldType::Float64.input_type(), "number");
        assert_eq!(FieldType::Date.input_type(), "date");
        assert_eq!(FieldType::DateTime.input_type(), "datetime-local");
        assert_eq!(FieldType::File.input_type(), "file");
        assert_eq!(FieldType::Image.input_type(), "file");
        assert_eq!(FieldType::Bool.input_type(), "text");
    }

    // ── parse_entity: basic field ─────────────────────────────────────

    #[test]
    fn parse_entity_basic_field() {
        let entity = parse_entity("Post", &["title:string".to_string()]).unwrap();
        assert_eq!(entity.name, "Post");
        assert_eq!(entity.fields.len(), 1);
        assert_eq!(entity.fields[0].name, "title");
        assert!(matches!(entity.fields[0].field_type, FieldType::String));
        assert!(!entity.fields[0].optional);
        assert!(entity.fields[0].relation.is_none());
    }

    // ── parse_entity: optional field ──────────────────────────────────

    #[test]
    fn parse_entity_optional_field() {
        let entity = parse_entity("Post", &["bio:text?".to_string()]).unwrap();
        assert_eq!(entity.fields.len(), 1);
        assert!(entity.fields[0].optional);
        assert!(matches!(entity.fields[0].field_type, FieldType::Text));
    }

    // ── parse_entity: belongs_to relation ─────────────────────────────

    #[test]
    fn parse_entity_belongs_to_relation() {
        let entity = parse_entity("Post", &["author_id:uuid->User".to_string()]).unwrap();

        // Field should exist with relation set
        assert_eq!(entity.fields.len(), 1);
        assert_eq!(entity.fields[0].name, "author_id");
        assert!(matches!(entity.fields[0].field_type, FieldType::Uuid));
        assert_eq!(entity.fields[0].relation.as_deref(), Some("User"));

        // BelongsTo relation should be in relations vec
        assert_eq!(entity.relations.len(), 1);
        assert_eq!(entity.relations[0].target_entity, "User");
        assert!(matches!(entity.relations[0].relation_type, RelationType::BelongsTo));
        assert_eq!(entity.relations[0].fk_column.as_deref(), Some("author_id"));
    }

    // ── parse_entity: has_many relation ───────────────────────────────

    #[test]
    fn parse_entity_has_many_relation() {
        let entity = parse_entity("User", &["posts:has_many->Post".to_string()]).unwrap();

        // has_many does not create a field, only a relation
        assert_eq!(entity.fields.len(), 0);
        assert_eq!(entity.relations.len(), 1);
        assert!(matches!(entity.relations[0].relation_type, RelationType::HasMany));
        assert_eq!(entity.relations[0].target_entity, "Post");
    }

    // ── parse_entity: m2m relation ────────────────────────────────────

    #[test]
    fn parse_entity_m2m_relation() {
        let entity = parse_entity("Post", &["tags:m2m->Tag".to_string()]).unwrap();

        // m2m does not create a field, only a relation
        assert_eq!(entity.fields.len(), 0);
        assert_eq!(entity.relations.len(), 1);
        assert!(matches!(entity.relations[0].relation_type, RelationType::ManyToMany));
        assert_eq!(entity.relations[0].target_entity, "Tag");
    }

    // ── parse_entity: validation annotations ──────────────────────────

    #[test]
    fn parse_entity_validations() {
        let entity = parse_entity("Post", &["title:string[min=3,max=100]".to_string()]).unwrap();
        assert_eq!(entity.fields.len(), 1);
        let validations = &entity.fields[0].validations;
        assert_eq!(validations.len(), 2);
        assert!(validations.contains(&ValidationRule::Min(3)));
        assert!(validations.contains(&ValidationRule::Max(100)));
    }

    // ── parse_entity: searchable annotation ───────────────────────────

    #[test]
    fn parse_entity_searchable() {
        let entity = parse_entity("Post", &["title:string[searchable]".to_string()]).unwrap();
        assert_eq!(entity.fields.len(), 1);
        assert!(entity.fields[0].searchable);
    }

    // ── parse_entity: multiple fields ─────────────────────────────────

    #[test]
    fn parse_entity_multiple_fields() {
        let entity = parse_entity(
            "Product",
            &[
                "title:string".to_string(),
                "price:decimal".to_string(),
                "description:text?".to_string(),
                "category_id:uuid->Category".to_string(),
                "tags:m2m->Tag".to_string(),
            ],
        )
        .unwrap();

        assert_eq!(entity.name, "Product");
        // title, price, description, category_id are fields; tags (m2m) creates no field
        assert_eq!(entity.fields.len(), 4);
        // BelongsTo(Category) + ManyToMany(Tag)
        assert_eq!(entity.relations.len(), 2);
    }

    // ── parse_entity: enum field via CLI ────────────────────────────────

    #[test]
    fn parse_entity_enum_field() {
        let entity = parse_entity(
            "Post",
            &["status:enum(draft,published,archived)".to_string()],
        )
        .unwrap();
        assert_eq!(entity.fields.len(), 1);
        assert_eq!(entity.fields[0].name, "status");
        assert!(matches!(
            entity.fields[0].field_type,
            FieldType::Enum(ref v) if v == &["draft", "published", "archived"]
        ));
    }

    // ── parse_entity: invalid format ──────────────────────────────────

    #[test]
    fn parse_entity_invalid_format() {
        assert!(parse_entity("Post", &["invalid_no_colon".to_string()]).is_err());
    }

    // ── parse_entity: unknown field type ──────────────────────────────

    #[test]
    fn parse_entity_unknown_field_type() {
        assert!(parse_entity("Post", &["name:foobar".to_string()]).is_err());
    }

    // ── parse_validations: mixed rules ────────────────────────────────

    #[test]
    fn parse_validations_mixed() {
        let rules = parse_validations("min=5,max=200,email,unique");
        assert_eq!(rules.len(), 4);
        assert!(rules.contains(&ValidationRule::Min(5)));
        assert!(rules.contains(&ValidationRule::Max(200)));
        assert!(rules.contains(&ValidationRule::Email));
        assert!(rules.contains(&ValidationRule::Unique));
    }

    #[test]
    fn parse_validations_regex() {
        let rules = parse_validations("regex=^[a-z]+$");
        assert_eq!(rules.len(), 1);
        assert!(matches!(&rules[0], ValidationRule::Regex(r) if r == "^[a-z]+$"));
    }

    #[test]
    fn parse_validations_empty() {
        let rules = parse_validations("");
        assert!(rules.is_empty());
    }

    // ── parse_entity: optional belongs_to ─────────────────────────────

    #[test]
    fn parse_entity_optional_belongs_to() {
        let entity = parse_entity("Post", &["category_id:uuid->Category?".to_string()]).unwrap();
        assert_eq!(entity.fields.len(), 1);
        assert!(entity.fields[0].optional);
        assert_eq!(entity.relations.len(), 1);
        assert!(entity.relations[0].optional);
    }

    // ── parse_entity: validation + relation combined ──────────────────

    #[test]
    fn parse_entity_validation_with_relation() {
        let entity = parse_entity("Post", &["author_id:uuid[required]->User".to_string()]).unwrap();
        assert_eq!(entity.fields.len(), 1);
        assert_eq!(entity.fields[0].relation.as_deref(), Some("User"));
        assert!(entity.fields[0].validations.contains(&ValidationRule::Required));
    }
}
