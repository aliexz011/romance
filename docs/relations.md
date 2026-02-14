# Relations Guide

Romance supports three types of relations between entities: BelongsTo (foreign key), HasMany, and ManyToMany. Relations generate code on both sides -- when you add a foreign key to one entity, the reverse relation is automatically injected into the target entity.

## Relation Types Overview

| Type | CLI Syntax | Creates Column? | Creates Junction Table? |
|------|-----------|----------------|------------------------|
| BelongsTo | `field_name:type->Entity` | Yes (FK column) | No |
| HasMany | `name:has_many->Entity` | No | No |
| ManyToMany | `name:m2m->Entity` | No | Yes |

## BelongsTo (Foreign Key)

A BelongsTo relation creates a foreign key column on the current entity and automatically injects a has-many reverse relation into the target entity.

### Syntax

```bash
romance generate entity <Entity> <fk_field>:<type>-><TargetEntity>
```

The `->` arrow indicates the target entity. The field name typically ends with `_id` by convention.

### Example

```bash
romance generate entity Post title:string body:text author_id:uuid->User
```

This generates:

**On the Post side (source entity):**

1. A `author_id` column of type `UUID` in the migration and model
2. A `Relation::User` variant in the `Relation` enum
3. An `impl Related<user::Entity> for Entity` block in the Post model

**On the User side (target entity, if it already exists):**

1. An `impl Related<post::Entity> for Entity` block injected into the User model (via the `ROMANCE:RELATIONS` marker)
2. A `list_posts` handler injected into the User handlers (via the `ROMANCE:RELATION_HANDLERS` marker)
3. A route for `GET /api/users/:id/posts` injected into User routes (via the `ROMANCE:RELATION_ROUTES` marker)

### Generated Backend Code

For `author_id:uuid->User` on a Post entity:

**Post model (`backend/src/entities/post.rs`):**

```rust
// In the Model struct
pub author_id: Uuid,

// In the Relation enum
#[sea_orm(
    belongs_to = "super::user::Entity",
    from = "Column::AuthorId",
    to = "super::user::Column::Id"
)]
User,

// Related impl
impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}
```

**User handlers (injected into `backend/src/handlers/user.rs`):**

```rust
pub async fn list_posts(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(params): Query<crate::pagination::PageRequest>,
) -> crate::errors::AppResult<crate::api::ApiResponse<Vec<crate::entities::post::Model>>> {
    // Paginated query filtering posts by author_id = id
}
```

**User routes (injected into `backend/src/routes/user.rs`):**

```rust
.route("/api/users/:id/posts", get(user::list_posts))
```

### Optional BelongsTo

Append `?` after the relation target to make the foreign key nullable:

```bash
romance generate entity Post title:string category_id:uuid->Category?
```

This generates:
- `category_id: Option<Uuid>` in the Rust model
- A nullable `UUID` column in the migration
- The relation is still created, but the field can be `NULL`

### BelongsTo with Validation

Validation annotations go in brackets before the `->` arrow:

```bash
romance generate entity Post author_id:uuid[required]->User
```

### Generated API Endpoints for BelongsTo

When Post has `author_id:uuid->User`, these endpoints are available:

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/users/:id/posts` | List all posts by a specific user (paginated) |

The Post's own CRUD endpoints accept `author_id` as a field in create/update requests.

### Nested Serialization with `?include=`

When an entity has BelongsTo relations, the generated `get` handler supports nested serialization via the `?include=` query parameter:

```
GET /api/posts/123?include=user
```

This returns the Post with the related User object embedded in the response, rather than just the `author_id` UUID. The backend generates a `DetailResponse` struct that includes the nested relation data.

Multiple relations can be included:

```
GET /api/posts/123?include=user,category
```

## HasMany

A HasMany relation is the reverse side of a BelongsTo. In most cases, you do not need to declare it explicitly -- Romance automatically injects the has-many reverse when you create a BelongsTo relation.

### When HasMany Is Auto-Inferred

When you generate Post with `author_id:uuid->User` and the User entity already exists, Romance automatically injects:
- `Related<post::Entity>` into User's model
- A `list_posts` handler into User's handlers
- A route for `GET /api/users/:id/posts` into User's routes

You get the has-many relationship without declaring it.

### Explicit HasMany

Use explicit HasMany only when you need to declare the reverse side at the same time you create the parent entity (before the child entity exists):

```bash
romance generate entity User name:string email:string posts:has_many->Post
```

**Important:** HasMany does not create a database column. It only creates a relation declaration. The actual foreign key must exist on the child entity (Post must have a `user_id` column).

### Order of Entity Generation

The order in which you generate entities matters for relation injection:

**Scenario 1: Target exists first (recommended)**

```bash
# 1. Generate User first
romance generate entity User name:string email:string

# 2. Generate Post with FK to User
romance generate entity Post title:string author_id:uuid->User
# Romance detects that User exists, injects has-many into User automatically
```

**Scenario 2: Target does not exist yet**

```bash
# 1. Generate Post with FK to User (User doesn't exist yet)
romance generate entity Post title:string author_id:uuid->User
# Romance creates Post, but cannot inject into User (it doesn't exist)

# 2. Generate User later
romance generate entity User name:string email:string
# The has-many reverse is NOT automatically injected in this case
```

For BelongsTo relations, generate the target entity first. For ManyToMany relations, pending relations handle the ordering automatically (see below).

## ManyToMany

A ManyToMany relation creates a junction table between two entities. Both sides get relation code, handlers, and routes for managing the association.

### Syntax

```bash
romance generate entity <Entity> <name>:m2m-><TargetEntity>
```

### Example

```bash
romance generate entity Post title:string body:text tags:m2m->Tag
```

### What Gets Generated

If both Post and Tag entities exist, Romance generates:

**1. Junction table entity (`backend/src/entities/post_tag.rs`):**

```rust
// Model with id, post_id, tag_id, created_at
pub struct Model {
    pub id: Uuid,
    pub post_id: Uuid,
    pub tag_id: Uuid,
    pub created_at: DateTimeWithTimeZone,
}

// Relations to both Post and Tag
enum Relation {
    Post,
    Tag,
}
```

**2. Junction migration (`backend/migration/src/m{timestamp}_create_post_tag_table.rs`):**

Creates a table with:
- `id` (UUID, primary key)
- `post_id` (UUID, foreign key to posts)
- `tag_id` (UUID, foreign key to tags)
- `created_at` (TIMESTAMPTZ)

**3. Related impls via junction (injected into both entity models):**

In Post's model:
```rust
impl Related<super::tag::Entity> for Entity {
    fn to() -> RelationDef {
        super::post_tag::Relation::Tag.def()
    }
    fn via() -> Option<RelationDef> {
        Some(super::post_tag::Relation::Post.def().rev())
    }
}
```

In Tag's model:
```rust
impl Related<super::post::Entity> for Entity {
    fn to() -> RelationDef {
        super::post_tag::Relation::Post.def()
    }
    fn via() -> Option<RelationDef> {
        Some(super::post_tag::Relation::Tag.def().rev())
    }
}
```

**4. M2M handlers (injected into both entities' handler files):**

For Post:
- `list_tags` -- List all tags associated with a post
- `add_tag` -- Associate a tag with a post
- `remove_tag` -- Remove a tag association from a post

For Tag:
- `list_posts` -- List all posts associated with a tag
- `add_post` -- Associate a post with a tag
- `remove_post` -- Remove a post association from a tag

**5. M2M routes (injected into both entities' route files):**

For Post:
```rust
.route("/api/posts/:id/tags", get(post::list_tags))
.route("/api/posts/:id/tags/:tag_id", post(post::add_tag))
.route("/api/posts/:id/tags/:tag_id", delete(post::remove_tag))
```

For Tag:
```rust
.route("/api/tags/:id/posts", get(tag::list_posts))
.route("/api/tags/:id/posts/:post_id", post(tag::add_post))
.route("/api/tags/:id/posts/:post_id", delete(tag::remove_post))
```

**6. Frontend relation hooks (`frontend/src/features/post/tag_hooks.ts`):**

TanStack Query hooks for managing the M2M relation from the frontend:
- `usePostTags(postId)` -- Fetch tags for a post
- `useAddTagToPost()` -- Add a tag to a post
- `useRemoveTagFromPost()` -- Remove a tag from a post

### Generated API Endpoints for ManyToMany

For a Post-Tag M2M relation:

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/posts/:id/tags` | List all tags for a post |
| `POST` | `/api/posts/:id/tags/:tag_id` | Associate a tag with a post |
| `DELETE` | `/api/posts/:id/tags/:tag_id` | Remove a tag from a post |
| `GET` | `/api/tags/:id/posts` | List all posts for a tag |
| `POST` | `/api/tags/:id/posts/:post_id` | Associate a post with a tag |
| `DELETE` | `/api/tags/:id/posts/:post_id` | Remove a post from a tag |

### Junction Table Naming Convention

Junction tables are named by concatenating the two entity names in **alphabetical order** (snake_case), separated by an underscore:

| Entity A | Entity B | Junction Table |
|----------|----------|----------------|
| Post | Tag | `post_tag` |
| Article | Category | `article_category` |
| User | Role | `role_user` |
| Product | Warehouse | `product_warehouse` |

This convention ensures that regardless of which entity declares the M2M relation, the junction table has a deterministic, consistent name.

### Pending Relations (Deferred M2M)

If the target entity does not exist when you declare a ManyToMany relation, Romance stores the relation as "pending" in `.romance/pending_relations.json` and defers the junction table generation.

```bash
# Tag doesn't exist yet
romance generate entity Post title:string tags:m2m->Tag
# Output: Target entity 'Tag' not found -- storing pending M2M relation

# Later, when Tag is generated:
romance generate entity Tag name:string
# Output: Applied pending M2M: Post <-> Tag
# Junction table post_tag is now generated, and both entities get their M2M code
```

This means you can declare ManyToMany relations in any order. The pending relation is automatically applied when the target entity is generated.

Pending relations are stored in `.romance/pending_relations.json`:

```json
[
  {
    "source_entity": "Post",
    "target_entity": "Tag",
    "relation_type": "ManyToMany"
  }
]
```

The file is automatically cleaned up after pending relations are resolved.

### Idempotent Junction Generation

If the junction table already exists (e.g., from a previous generation or a circular M2M declaration), Romance skips the junction table creation but still ensures the `Related` impls, handlers, and routes are injected into both entities. This prevents duplicate tables while ensuring both sides have complete relation code.

## Complete Relation Example

Here is a realistic example with multiple entities and relation types:

```bash
# Step 1: Create Category (standalone entity)
romance generate entity Category name:string description:text?

# Step 2: Create Tag (standalone entity)
romance generate entity Tag name:string[unique] color:string?

# Step 3: Create User with auth
romance generate auth

# Step 4: Create Post with multiple relations
romance generate entity Post \
  title:string[min=3,max=200,searchable] \
  body:text[searchable] \
  published:bool \
  author_id:uuid->User \
  category_id:uuid->Category? \
  tags:m2m->Tag
```

After running these commands, the following relation infrastructure exists:

**BelongsTo relations (with auto-injected has-many reverse):**
- Post -> User (via `author_id`), User gets `GET /api/users/:id/posts`
- Post -> Category (via `category_id`, optional), Category gets `GET /api/categories/:id/posts`

**ManyToMany relations:**
- Post <-> Tag (via `post_tag` junction table)
  - Post gets `GET/POST/DELETE /api/posts/:id/tags[/:tag_id]`
  - Tag gets `GET/POST/DELETE /api/tags/:id/posts[/:post_id]`

**Frontend components automatically include:**
- Post form with User and Category dropdown selects (for FK fields)
- Post detail page showing related tags with add/remove functionality
- User detail page showing a list of their posts
- Category detail page showing posts in that category
- Tag detail page showing posts with that tag

## Frontend Components for Relations

### Detail Page with Related Items

The generated `Detail.tsx` component for an entity with relations includes sections for related items. For a Post with `tags:m2m->Tag`:

- The detail page shows a "Tags" section listing all associated tags
- Each tag links to its own detail page
- Add/remove buttons allow managing the M2M association inline
- The relation hooks (e.g., `tag_hooks.ts`) provide the TanStack Query mutations

### Form with FK Dropdowns

The generated `Form.tsx` component for an entity with BelongsTo relations includes dropdown selects for FK fields. For `category_id:uuid->Category`:

- The form fetches all categories and renders them in a `<Select>` dropdown
- The selected category's ID is sent as `category_id` in the create/update request
- For optional BelongsTo relations, the dropdown includes an empty "None" option

### List Page with Relation Columns

The generated `List.tsx` component displays FK relation names instead of raw UUIDs when possible, and includes filter options for relation fields.

## Relation Summary Table

| Relation | Who Declares | Column Created | Files Generated | Code Injected Into Target |
|----------|-------------|----------------|-----------------|--------------------------|
| BelongsTo | Source entity | FK column on source | Migration column | `Related` impl, list handler, route on target |
| HasMany | Either side (usually auto-inferred) | None | None | Relation declaration only |
| ManyToMany | Either side | Junction table (separate entity) | Junction model + migration, relation hooks (frontend) | `Related` via junction, list/add/remove handlers + routes on both sides |
