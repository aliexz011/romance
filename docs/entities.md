# Entity Generation Guide

Entities are the core building blocks of a Romance project. Each entity generates a complete CRUD stack: a Rust model with SeaORM, handlers and routes for the Axum backend, a database migration, and a full set of React components with TypeScript types, API client, and TanStack Query hooks.

## Command Syntax

```bash
romance generate entity <Name> [field:type...]
```

- **`<Name>`** -- Entity name in PascalCase (e.g., `Product`, `BlogPost`, `UserProfile`).
- **`[field:type...]`** -- Zero or more field definitions. If no fields are given, Romance enters interactive mode.

### Examples

```bash
# Simple entity with required fields
romance generate entity Product title:string price:decimal in_stock:bool

# Entity with optional fields (? suffix)
romance generate entity Article title:string body:text summary:text? published_at:datetime?

# Entity with a foreign key relation
romance generate entity Post title:string body:text author_id:uuid->User

# Entity with validation rules
romance generate entity User name:string[min=2,max=50] email:string[email,unique]

# Entity with enum field
romance generate entity Order status:enum(pending,processing,shipped,delivered) total:decimal

# Entity with searchable fields
romance generate entity Article title:string[searchable] body:text[searchable] published:bool

# Entity with file and image fields
romance generate entity Document title:string file:file preview:image?

# Entity with visibility restrictions
romance generate entity Employee name:string salary:decimal[admin_only] ssn:string[roles=hr;admin]

# Complex entity combining multiple features
romance generate entity Product \
  title:string[min=3,max=100,searchable] \
  description:text[searchable]? \
  price:decimal[min=0] \
  sku:string[unique] \
  category_id:uuid->Category \
  tags:m2m->Tag \
  cover_image:image? \
  status:enum(draft,active,archived)
```

## Interactive Mode

When you run `romance generate entity Product` without any field definitions, Romance enters interactive mode. It prompts you for each field:

1. **Field name** -- Enter a name or press Enter with an empty name to finish.
2. **Field type** -- Select from the list of available types.
3. **Optional?** -- Whether the field is nullable.
4. **Foreign key** -- Optionally specify a related entity name.

After defining fields, you are asked whether to add relations (has_many, m2m).

Interactive mode is useful for exploring field types and building entities step by step, but does not currently support validation rules, searchable annotations, or visibility settings. Use CLI mode for those features.

## Field Types

Romance supports 14 field types. Each type maps to a Rust type, TypeScript type, PostgreSQL column type, and a shadcn/ui form component.

### Complete Field Type Reference

| Type | CLI Aliases | Description |
|------|-------------|-------------|
| `String` | `string`, `str` | Short text, max 255 characters |
| `Text` | `text` | Unlimited-length text |
| `Bool` | `bool`, `boolean` | True/false value |
| `Int32` | `i32`, `int`, `int32`, `integer` | 32-bit signed integer |
| `Int64` | `i64`, `int64`, `bigint` | 64-bit signed integer |
| `Float64` | `f64`, `float`, `float64`, `double` | 64-bit floating point |
| `Decimal` | `decimal`, `money` | Arbitrary-precision decimal |
| `Uuid` | `uuid` | UUID v4 identifier |
| `DateTime` | `datetime`, `timestamp` | Timestamp with timezone |
| `Date` | `date` | Date without time |
| `Json` | `json`, `jsonb` | JSON binary data |
| `Enum` | `enum(a,b,c)` | Enumeration with named variants |
| `File` | `file` | File upload (stores path as string) |
| `Image` | `image` | Image upload (stores path as string, with image validation) |

### Type Mappings Across the Stack

| FieldType | Rust | TypeScript | PostgreSQL | SeaORM Column | shadcn Component | HTML Input |
|-----------|------|------------|------------|---------------|-----------------|------------|
| `String` | `String` | `string` | `VARCHAR(255)` | `ColumnType::String(StringLen::N(255))` | `Input` | `text` |
| `Text` | `String` | `string` | `TEXT` | `ColumnType::Text` | `Textarea` | `text` |
| `Bool` | `bool` | `boolean` | `BOOLEAN` | `ColumnType::Boolean` | `Switch` | `text` |
| `Int32` | `i32` | `number` | `INTEGER` | `ColumnType::Integer` | `Input` | `number` |
| `Int64` | `i64` | `number` | `BIGINT` | `ColumnType::BigInteger` | `Input` | `number` |
| `Float64` | `f64` | `number` | `DOUBLE PRECISION` | `ColumnType::Double` | `Input` | `number` |
| `Decimal` | `Decimal` | `number` | `DECIMAL` | `ColumnType::Decimal(None)` | `Input` | `number` |
| `Uuid` | `Uuid` | `string` | `UUID` | `ColumnType::Uuid` | `Input` | `text` |
| `DateTime` | `DateTimeWithTimeZone` | `string` | `TIMESTAMPTZ` | `ColumnType::TimestampWithTimeZone` | `Input` | `datetime-local` |
| `Date` | `Date` | `string` | `DATE` | `ColumnType::Date` | `Input` | `date` |
| `Json` | `Json` | `unknown` | `JSONB` | `ColumnType::JsonBinary` | `Textarea` | `text` |
| `Enum(...)` | `String` | `string` | `VARCHAR(255)` | `ColumnType::String(StringLen::N(255))` | `Select` | `text` |
| `File` | `String` | `string` | `VARCHAR(512)` | `ColumnType::String(StringLen::N(512))` | `FileInput` | `file` |
| `Image` | `String` | `string` | `VARCHAR(512)` | `ColumnType::String(StringLen::N(512))` | `ImageInput` | `file` |

### String vs Text

Use `string` for short values like names, titles, and slugs (stored as `VARCHAR(255)`). Use `text` for longer content like descriptions, article bodies, and comments (stored as `TEXT` with no length limit). On the frontend, `string` renders as an `<Input>` and `text` renders as a `<Textarea>`.

### Enum Fields

Enum fields are defined inline with their variants:

```bash
romance generate entity Post status:enum(draft,published,archived)
```

This generates:
- A `VARCHAR(255)` column in PostgreSQL (no native enum type for portability)
- A `string` type in TypeScript
- A `Select` dropdown in the frontend form, pre-populated with the defined variants

Variants are case-sensitive and stored as provided. At least one variant is required.

### File and Image Fields

File and image fields store the file path as a string (`VARCHAR(512)`). The actual file upload handling requires the storage addon:

```bash
romance add storage
romance generate entity Document title:string file:file preview:image?
```

On the frontend, `File` renders a `FileInput` component and `Image` renders an `ImageInput` component (which includes image preview and validation).

## Optional Fields

Append `?` to a field definition to make it nullable:

```bash
romance generate entity Product title:string description:text? published_at:datetime?
```

Optional fields:
- Use `Option<T>` in Rust
- Use `T | null` or `T | undefined` in TypeScript
- Are `NULL`-able in PostgreSQL
- Render as non-required form fields on the frontend

## Validation Rules

Validation rules are specified in square brackets after the field type. Multiple rules are comma-separated:

```bash
name:type[rule1,rule2,rule3]
```

### Available Validation Rules

| Rule | Syntax | Description | Example |
|------|--------|-------------|---------|
| **Min** | `min=N` | Minimum value (numbers) or minimum length (strings) | `title:string[min=3]` |
| **Max** | `max=N` | Maximum value (numbers) or maximum length (strings) | `title:string[max=100]` |
| **Email** | `email` | Must be a valid email address | `email:string[email]` |
| **URL** | `url` | Must be a valid URL | `website:string[url]` |
| **Regex** | `regex=pattern` | Must match the given regular expression | `slug:string[regex=^[a-z0-9-]+$]` |
| **Required** | `required` | Must not be empty or null (explicit enforcement) | `name:string[required]` |
| **Unique** | `unique` | Must be unique across all records | `email:string[unique]` |

### Combining Validation Rules

Rules can be combined freely:

```bash
romance generate entity User \
  name:string[min=2,max=50,required] \
  email:string[email,unique] \
  website:string[url]? \
  username:string[min=3,max=20,regex=^[a-zA-Z0-9_]+$,unique]
```

When the `validation` addon is installed (`romance add validation`), these rules generate:
- **Backend:** `validator` crate derive macros on the create/update structs
- **Frontend:** Zod schemas for client-side form validation

### Validation with Relations

Validation annotations can be combined with foreign key relations. Place the brackets before the `->` arrow:

```bash
romance generate entity Post author_id:uuid[required]->User
```

## Field Visibility

Visibility annotations control which users can see a field in API responses. They are specified inside the same square brackets as validation rules:

```bash
name:type[visibility_annotation]
```

### Visibility Options

| Annotation | Description | Example |
|------------|-------------|---------|
| `admin_only` | Only users with the admin role can see this field | `salary:decimal[admin_only]` |
| `authenticated` | Only logged-in users can see this field | `email:string[authenticated]` |
| `roles=role1;role2` | Only users with one of the specified roles (semicolon-separated) | `ssn:string[roles=hr;admin]` |

Fields without a visibility annotation default to `Public` (visible to everyone).

Visibility can be combined with validation rules:

```bash
romance generate entity Employee \
  name:string[min=2,max=50] \
  salary:decimal[admin_only,min=0] \
  ssn:string[roles=hr;admin,unique]
```

**Note:** The `roles` annotation uses semicolons (`;`) to separate role names, since commas are used to separate annotations from each other.

## Searchable Fields

Mark fields as searchable to include them in full-text search queries:

```bash
romance generate entity Article title:string[searchable] body:text[searchable] author:string
```

When the `search` addon is installed (`romance add search`), searchable fields are indexed with PostgreSQL `tsvector` and `GIN` indexes for fast full-text search.

Searchable can be combined with other annotations:

```bash
title:string[min=3,max=100,searchable]
```

## Generated Files

When you run `romance generate entity Product title:string price:decimal`, the following files are created or updated:

### Backend Files (Created)

| File | Purpose |
|------|---------|
| `backend/src/entities/product.rs` | SeaORM entity model: `Model`, `ActiveModel`, `Column`, `Relation` enums, `Create`/`Update` structs |
| `backend/src/handlers/product.rs` | Axum handler functions: `list`, `get`, `create`, `update`, `delete`, `bulk_create`, `bulk_delete` |
| `backend/src/routes/product.rs` | Route definitions with `pub fn router() -> Router<AppState>` |
| `backend/migration/src/m{timestamp}_create_product_table.rs` | SeaORM migration with `up()` and `down()` |

### Backend Files (Updated via Markers)

| File | What Gets Inserted |
|------|--------------------|
| `backend/src/entities/mod.rs` | `pub mod product;` |
| `backend/src/handlers/mod.rs` | `pub mod product;` |
| `backend/src/routes/mod.rs` | `pub mod product;` and `.merge(product::router())` |
| `backend/migration/src/lib.rs` | `mod m{timestamp}_create_product_table;` and `Box::new(m{timestamp}::Migration),` |

### Frontend Files (Created)

| File | Purpose |
|------|---------|
| `frontend/src/features/product/types.ts` | TypeScript interfaces: `Product`, `CreateProduct`, `UpdateProduct` |
| `frontend/src/features/product/api.ts` | API client: fetch functions for all CRUD operations |
| `frontend/src/features/product/hooks.ts` | TanStack Query hooks: `useProducts`, `useProduct`, `useCreateProduct`, etc. |
| `frontend/src/features/product/ProductList.tsx` | List page with table, pagination, filtering, sorting, and bulk actions |
| `frontend/src/features/product/ProductForm.tsx` | Create/edit form with shadcn/ui components, validation, and FK dropdowns |
| `frontend/src/features/product/ProductDetail.tsx` | Detail page showing all fields, related entities, and action buttons |

### Frontend Files (Updated via Markers)

| File | What Gets Inserted |
|------|--------------------|
| `frontend/src/App.tsx` | Import statements, `<Route>` elements for list/new/detail/edit, and nav link |

### Generated API Endpoints

For an entity named `Product`, the following REST endpoints are created:

| Method | Path | Handler | Description |
|--------|------|---------|-------------|
| `GET` | `/api/products` | `list` | List all products (paginated, filterable, sortable) |
| `GET` | `/api/products/:id` | `get` | Get a single product by ID |
| `POST` | `/api/products` | `create` | Create a new product |
| `PUT` | `/api/products/:id` | `update` | Update an existing product |
| `DELETE` | `/api/products/:id` | `delete` | Delete a product |
| `POST` | `/api/products/bulk` | `bulk_create` | Create multiple products |
| `DELETE` | `/api/products/bulk` | `bulk_delete` | Delete multiple products by ID |

### Generated Frontend Routes

| Path | Component | Description |
|------|-----------|-------------|
| `/products` | `ProductList` | List view with table |
| `/products/new` | `ProductForm` | Create form |
| `/products/:id` | `ProductDetail` | Detail view |
| `/products/:id/edit` | `ProductForm` | Edit form (pre-filled) |

## The ROMANCE:CUSTOM Marker

Every generated entity file contains a special marker comment near the end:

```rust
// === ROMANCE:CUSTOM ===
```

```typescript
// === ROMANCE:CUSTOM ===
```

**Any code you write below this marker is preserved when the entity is re-generated.** This is the designated area for your custom business logic, additional handlers, helper functions, and anything else that should not be overwritten.

### How It Works

When Romance re-generates an entity file:

1. It reads the existing file and splits it at the `ROMANCE:CUSTOM` marker.
2. Everything above the marker is replaced with the newly generated content.
3. Everything below the marker (your custom code) is appended unchanged.

This means you can safely re-run `romance generate entity Product ...` with different fields, and your custom code will survive.

### Example

```rust
// ... generated code above ...

// === ROMANCE:CUSTOM ===

// Your custom code below -- this is preserved on re-generation:

impl Model {
    pub fn display_price(&self) -> String {
        format!("${:.2}", self.price)
    }
}

pub async fn featured_products(
    State(state): State<AppState>,
) -> AppResult<ApiResponse<Vec<Model>>> {
    // custom handler logic
}
```

## Re-generation Behavior

You can re-generate an entity at any time by running the same command again (with the same or different fields). The behavior is:

- **Generated code** (above `ROMANCE:CUSTOM`) is fully replaced with fresh output.
- **Custom code** (below `ROMANCE:CUSTOM`) is preserved exactly as-is.
- **Marker insertions** (module declarations, route registrations) are idempotent -- if the line already exists, it is not duplicated.
- **Migrations** are not re-generated. A new migration is created each time. If you need to change a column, generate a new entity definition and manually handle the migration delta, or delete the old migration and re-generate.

## Destroying Entities

To remove a generated entity and all its files:

```bash
romance destroy entity Product
```

This removes the entity's backend files (model, handlers, routes), frontend files (types, API, hooks, components), and cleans up marker-inserted lines from module files and `App.tsx`.
