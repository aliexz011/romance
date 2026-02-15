pub mod add;
pub mod addon;
pub mod check;
pub mod db;
pub mod destroy;
pub mod dev;
pub mod doctor;
pub mod generate;
pub mod new;
pub mod remove;
pub mod run;
pub mod test;
pub mod update;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "romance", version, about = "Full-stack code generator: Rust (Axum + SeaORM) + React (TypeScript + shadcn/ui)")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create a new full-stack project
    New {
        /// Project name
        name: String,
    },
    /// Generate code (entity, types, openapi, auth, admin)
    Generate {
        #[command(subcommand)]
        command: GenerateCommands,
    },
    /// Add a feature to the project
    Add {
        #[command(subcommand)]
        command: AddCommands,
    },
    /// Remove an installed addon
    Remove {
        #[command(subcommand)]
        command: RemoveCommands,
    },
    /// Run development servers (backend + frontend)
    Dev,
    /// Run checks (cargo check, cargo test, tsc)
    Check,
    /// Run tests with temporary database
    Test,
    /// Database operations
    Db {
        #[command(subcommand)]
        command: DbCommands,
    },
    /// Update scaffold files to latest template versions
    Update {
        /// Initialize manifest for existing project (first-time setup)
        #[arg(long)]
        init: bool,
    },
    /// Run a custom management command
    Run {
        /// Command name
        command: String,
        /// Additional arguments
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Remove generated code
    Destroy {
        #[command(subcommand)]
        command: DestroyCommands,
    },
    /// List or check status of addons
    Addon {
        #[command(subcommand)]
        command: AddonCommands,
    },
    /// Check project health and dependencies
    Doctor,
    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        shell: clap_complete::Shell,
    },
}

#[derive(Subcommand)]
pub enum GenerateCommands {
    /// Generate a new entity with CRUD
    Entity {
        /// Entity name (PascalCase)
        name: String,
        /// Field definitions: name:type, e.g. title:string price:decimal
        #[arg(trailing_var_arg = true)]
        fields: Vec<String>,
    },
    /// Generate TypeScript types from Rust structs
    Types,
    /// Generate OpenAPI spec
    Openapi,
    /// Generate authentication (User entity, JWT, login/register)
    Auth,
    /// Generate admin panel
    Admin,
}

#[derive(Subcommand)]
pub enum AddCommands {
    /// Add validation (backend: validator crate, frontend: Zod schemas)
    Validation,
    /// Add soft-delete support (deleted_at column, restore/force-delete endpoints)
    SoftDelete,
    /// Add audit logging (tracks all create/update/delete with user attribution)
    AuditLog,
    /// Add file/image upload with pluggable storage (local or S3)
    Storage,
    /// Add full-text search (PostgreSQL tsvector + GIN index)
    Search,
    /// Add OAuth social authentication
    Oauth {
        /// OAuth provider (google, github, discord)
        provider: String,
    },
    /// Add security middleware (rate limiting, security headers)
    Security,
    /// Add observability (structured logging, request ID, tracing)
    Observability,
    /// Add developer dashboard (/dev route)
    Dashboard,
    /// Add email system (SMTP via lettre, password reset handler)
    Email,
    /// Add internationalization (i18n) support with locale detection
    I18n,
    /// Add caching layer (Redis-backed cache service)
    Cache,
    /// Add background tasks (PostgreSQL-backed task queue)
    Tasks,
    /// Add WebSocket support (real-time communication with EventBus bridge)
    Websocket,
    /// Add API key authentication for machine-to-machine auth
    ApiKeys,
    /// Add row-level multitenancy (tenant_id on all entities, JWT tenant resolution)
    Multitenancy,
}

#[derive(Subcommand)]
pub enum RemoveCommands {
    /// Remove validation addon
    Validation,
    /// Remove soft-delete addon
    SoftDelete,
    /// Remove audit logging addon
    AuditLog,
    /// Remove file storage addon
    Storage,
    /// Remove full-text search addon
    Search,
    /// Remove OAuth addon
    Oauth,
    /// Remove security middleware addon
    Security,
    /// Remove observability addon
    Observability,
    /// Remove developer dashboard addon
    Dashboard,
    /// Remove email system addon
    Email,
    /// Remove i18n addon
    I18n,
    /// Remove caching layer addon
    Cache,
    /// Remove background tasks addon
    Tasks,
    /// Remove WebSocket addon
    Websocket,
    /// Remove API key authentication addon
    ApiKeys,
    /// Remove multitenancy addon
    Multitenancy,
}

#[derive(Subcommand)]
pub enum DbCommands {
    /// Run pending migrations
    Migrate,
    /// Rollback last migration
    Rollback,
    /// Show migration status
    Status,
    /// Run seed data
    Seed,
}

#[derive(Subcommand)]
pub enum DestroyCommands {
    /// Remove a generated entity and its files
    Entity {
        /// Entity name (PascalCase)
        name: String,
    },
}

#[derive(Subcommand)]
pub enum AddonCommands {
    /// List all available addons
    List,
    /// Show installation status of addons in the current project
    Status,
}

pub fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::New { name } => new::run(&name),
        Commands::Generate { command } => match command {
            GenerateCommands::Entity { name, fields } => generate::run_entity(&name, &fields),
            GenerateCommands::Types => generate::run_types(),
            GenerateCommands::Openapi => generate::run_openapi(),
            GenerateCommands::Auth => generate::run_auth(),
            GenerateCommands::Admin => generate::run_admin(),
        },
        Commands::Add { command } => match command {
            AddCommands::Validation => add::run_validation(),
            AddCommands::SoftDelete => add::run_soft_delete(),
            AddCommands::AuditLog => add::run_audit_log(),
            AddCommands::Storage => add::run_storage(),
            AddCommands::Search => add::run_search(),
            AddCommands::Oauth { provider } => add::run_oauth(&provider),
            AddCommands::Security => add::run_security(),
            AddCommands::Observability => add::run_observability(),
            AddCommands::Dashboard => add::run_dashboard(),
            AddCommands::Email => add::run_email(),
            AddCommands::I18n => add::run_i18n(),
            AddCommands::Cache => add::run_cache(),
            AddCommands::Tasks => add::run_tasks(),
            AddCommands::Websocket => add::run_websocket(),
            AddCommands::ApiKeys => add::run_api_keys(),
            AddCommands::Multitenancy => add::run_multitenancy(),
        },
        Commands::Remove { command } => match command {
            RemoveCommands::Validation => remove::run_validation(),
            RemoveCommands::SoftDelete => remove::run_soft_delete(),
            RemoveCommands::AuditLog => remove::run_audit_log(),
            RemoveCommands::Storage => remove::run_storage(),
            RemoveCommands::Search => remove::run_search(),
            RemoveCommands::Oauth => remove::run_oauth(),
            RemoveCommands::Security => remove::run_security(),
            RemoveCommands::Observability => remove::run_observability(),
            RemoveCommands::Dashboard => remove::run_dashboard(),
            RemoveCommands::Email => remove::run_email(),
            RemoveCommands::I18n => remove::run_i18n(),
            RemoveCommands::Cache => remove::run_cache(),
            RemoveCommands::Tasks => remove::run_tasks(),
            RemoveCommands::Websocket => remove::run_websocket(),
            RemoveCommands::ApiKeys => remove::run_api_keys(),
            RemoveCommands::Multitenancy => remove::run_multitenancy(),
        },
        Commands::Dev => dev::run(),
        Commands::Check => check::run(),
        Commands::Test => test::run(),
        Commands::Db { command } => match command {
            DbCommands::Migrate => db::migrate(),
            DbCommands::Rollback => db::rollback(),
            DbCommands::Status => db::status(),
            DbCommands::Seed => db::seed(),
        },
        Commands::Update { init } => update::run(init),
        Commands::Run { command, args } => run::run(&command, &args),
        Commands::Destroy { command } => match command {
            DestroyCommands::Entity { name } => destroy::run_entity(&name),
        },
        Commands::Addon { command } => match command {
            AddonCommands::List => addon::run_list(),
            AddonCommands::Status => addon::run_status(),
        },
        Commands::Doctor => doctor::run(),
        Commands::Completions { shell } => {
            use clap::CommandFactory;
            let mut cmd = Cli::command();
            clap_complete::generate(shell, &mut cmd, "romance", &mut std::io::stdout());
            Ok(())
        }
    }
}
