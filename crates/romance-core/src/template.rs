use anyhow::{Context as _, Result};
use heck::{ToLowerCamelCase, ToPascalCase, ToSnakeCase, ToTitleCase};
use romance_templates::Templates;
use std::collections::HashMap;
use tera::{Context, Tera, Value};

pub struct TemplateEngine {
    tera: Tera,
}

impl TemplateEngine {
    pub fn new() -> Result<Self> {
        let mut tera = Tera::default();

        // Load only .tera templates into Tera (static files are accessed via get_raw())
        for file in Templates::iter() {
            let path = file.as_ref();
            if !path.ends_with(".tera") {
                continue;
            }
            if let Some(content) = Templates::get(path) {
                let content_str = std::str::from_utf8(content.data.as_ref())?;
                tera.add_raw_template(path, content_str)?;
            }
        }

        // Register custom filters
        tera.register_filter("snake_case", snake_case_filter);
        tera.register_filter("pascal_case", pascal_case_filter);
        tera.register_filter("camel_case", camel_case_filter);
        tera.register_filter("plural", plural_filter);
        tera.register_filter("title_case", title_case_filter);
        tera.register_filter("rust_ident", rust_ident_filter);

        Ok(TemplateEngine { tera })
    }

    pub fn render(&self, template_name: &str, context: &Context) -> Result<String> {
        let result = self
            .tera
            .render(template_name, context)
            .with_context(|| format!("Failed to render template '{}'", template_name))?;
        Ok(result)
    }

    /// Read an embedded file as raw string without Tera rendering.
    pub fn get_raw(&self, path: &str) -> Result<String> {
        let content = Templates::get(path)
            .with_context(|| format!("Embedded file '{}' not found", path))?;
        let s = std::str::from_utf8(content.data.as_ref())
            .with_context(|| format!("Invalid UTF-8 in '{}'", path))?;
        Ok(s.to_string())
    }
}

fn snake_case_filter(
    value: &Value,
    _args: &HashMap<String, Value>,
) -> tera::Result<Value> {
    match value.as_str() {
        Some(s) => Ok(Value::String(s.to_snake_case())),
        None => Err(tera::Error::msg("snake_case filter expects a string")),
    }
}

fn pascal_case_filter(
    value: &Value,
    _args: &HashMap<String, Value>,
) -> tera::Result<Value> {
    match value.as_str() {
        Some(s) => Ok(Value::String(s.to_pascal_case())),
        None => Err(tera::Error::msg("pascal_case filter expects a string")),
    }
}

fn camel_case_filter(
    value: &Value,
    _args: &HashMap<String, Value>,
) -> tera::Result<Value> {
    match value.as_str() {
        Some(s) => Ok(Value::String(s.to_lower_camel_case())),
        None => Err(tera::Error::msg("camel_case filter expects a string")),
    }
}

fn plural_filter(
    value: &Value,
    _args: &HashMap<String, Value>,
) -> tera::Result<Value> {
    match value.as_str() {
        Some(s) => Ok(Value::String(crate::utils::pluralize(s))),
        None => Err(tera::Error::msg("plural filter expects a string")),
    }
}

fn title_case_filter(
    value: &Value,
    _args: &HashMap<String, Value>,
) -> tera::Result<Value> {
    match value.as_str() {
        Some(s) => Ok(Value::String(s.to_title_case())),
        None => Err(tera::Error::msg("title_case filter expects a string")),
    }
}

fn rust_ident_filter(
    value: &Value,
    _args: &HashMap<String, Value>,
) -> tera::Result<Value> {
    match value.as_str() {
        Some(s) => Ok(Value::String(crate::utils::rust_ident(s))),
        None => Err(tera::Error::msg("rust_ident filter expects a string")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Filter unit tests (direct function calls) ─────────────────────

    fn val(s: &str) -> Value {
        Value::String(s.to_string())
    }
    fn empty_args() -> HashMap<String, Value> {
        HashMap::new()
    }

    #[test]
    fn snake_case_filter_works() {
        let result = snake_case_filter(&val("ProductCategory"), &empty_args()).unwrap();
        assert_eq!(result.as_str().unwrap(), "product_category");
    }

    #[test]
    fn snake_case_filter_single_word() {
        let result = snake_case_filter(&val("Post"), &empty_args()).unwrap();
        assert_eq!(result.as_str().unwrap(), "post");
    }

    #[test]
    fn pascal_case_filter_works() {
        let result = pascal_case_filter(&val("product_category"), &empty_args()).unwrap();
        assert_eq!(result.as_str().unwrap(), "ProductCategory");
    }

    #[test]
    fn pascal_case_filter_from_snake() {
        let result = pascal_case_filter(&val("blog_post"), &empty_args()).unwrap();
        assert_eq!(result.as_str().unwrap(), "BlogPost");
    }

    #[test]
    fn camel_case_filter_works() {
        let result = camel_case_filter(&val("ProductCategory"), &empty_args()).unwrap();
        assert_eq!(result.as_str().unwrap(), "productCategory");
    }

    #[test]
    fn camel_case_filter_from_snake() {
        let result = camel_case_filter(&val("blog_post"), &empty_args()).unwrap();
        assert_eq!(result.as_str().unwrap(), "blogPost");
    }

    #[test]
    fn plural_filter_regular() {
        let result = plural_filter(&val("post"), &empty_args()).unwrap();
        assert_eq!(result.as_str().unwrap(), "posts");
    }

    #[test]
    fn plural_filter_category() {
        let result = plural_filter(&val("Category"), &empty_args()).unwrap();
        assert_eq!(result.as_str().unwrap(), "Categories");
    }

    #[test]
    fn plural_filter_box() {
        let result = plural_filter(&val("box"), &empty_args()).unwrap();
        assert_eq!(result.as_str().unwrap(), "boxes");
    }

    #[test]
    fn title_case_filter_works() {
        let result = title_case_filter(&val("product_category"), &empty_args()).unwrap();
        assert_eq!(result.as_str().unwrap(), "Product Category");
    }

    #[test]
    fn rust_ident_filter_works() {
        let result = rust_ident_filter(&val("type"), &empty_args()).unwrap();
        assert_eq!(result.as_str().unwrap(), "r#type");
    }

    #[test]
    fn rust_ident_filter_non_reserved() {
        let result = rust_ident_filter(&val("title"), &empty_args()).unwrap();
        assert_eq!(result.as_str().unwrap(), "title");
    }

    // ── Filter error on non-string ────────────────────────────────────

    #[test]
    fn snake_case_filter_rejects_non_string() {
        let num = Value::Number(serde_json::Number::from(42));
        assert!(snake_case_filter(&num, &empty_args()).is_err());
    }

    #[test]
    fn pascal_case_filter_rejects_non_string() {
        let num = Value::Number(serde_json::Number::from(42));
        assert!(pascal_case_filter(&num, &empty_args()).is_err());
    }

    #[test]
    fn camel_case_filter_rejects_non_string() {
        let num = Value::Number(serde_json::Number::from(42));
        assert!(camel_case_filter(&num, &empty_args()).is_err());
    }

    #[test]
    fn plural_filter_rejects_non_string() {
        let num = Value::Number(serde_json::Number::from(42));
        assert!(plural_filter(&num, &empty_args()).is_err());
    }

    // ── TemplateEngine creation ───────────────────────────────────────

    #[test]
    fn template_engine_creates_successfully() {
        let _engine = TemplateEngine::new().unwrap();
    }

    // ── Rendering with custom filters via the engine ──────────────────

    #[test]
    fn render_inline_template_with_snake_case() {
        let mut tera = Tera::default();
        tera.register_filter("snake_case", snake_case_filter);
        tera.add_raw_template("test", "{{ name | snake_case }}").unwrap();
        let mut ctx = Context::new();
        ctx.insert("name", "ProductCategory");
        let result = tera.render("test", &ctx).unwrap();
        assert_eq!(result, "product_category");
    }

    #[test]
    fn render_inline_template_with_plural() {
        let mut tera = Tera::default();
        tera.register_filter("plural", plural_filter);
        tera.add_raw_template("test", "{{ name | plural }}").unwrap();
        let mut ctx = Context::new();
        ctx.insert("name", "Category");
        let result = tera.render("test", &ctx).unwrap();
        assert_eq!(result, "Categories");
    }

    #[test]
    fn render_inline_template_with_multiple_filters() {
        let mut tera = Tera::default();
        tera.register_filter("snake_case", snake_case_filter);
        tera.register_filter("pascal_case", pascal_case_filter);
        tera.register_filter("camel_case", camel_case_filter);
        tera.add_raw_template(
            "test",
            "snake={{ name | snake_case }} pascal={{ name | pascal_case }} camel={{ name | camel_case }}",
        )
        .unwrap();
        let mut ctx = Context::new();
        ctx.insert("name", "blog_post");
        let result = tera.render("test", &ctx).unwrap();
        assert_eq!(result, "snake=blog_post pascal=BlogPost camel=blogPost");
    }

    #[test]
    fn render_inline_template_with_context_variables() {
        let mut tera = Tera::default();
        tera.add_raw_template("test", "Hello {{ name }}, port {{ port }}").unwrap();
        let mut ctx = Context::new();
        ctx.insert("name", "Romance");
        ctx.insert("port", &3000);
        let result = tera.render("test", &ctx).unwrap();
        assert_eq!(result, "Hello Romance, port 3000");
    }
}
