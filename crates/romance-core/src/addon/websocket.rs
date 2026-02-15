use crate::addon::Addon;
use anyhow::Result;
use std::path::Path;

pub struct WebsocketAddon;

impl Addon for WebsocketAddon {
    fn name(&self) -> &str {
        "websocket"
    }

    fn check_prerequisites(&self, project_root: &Path) -> Result<()> {
        super::check_romance_project(project_root)
    }

    fn is_already_installed(&self, project_root: &Path) -> bool {
        project_root.join("backend/src/ws.rs").exists()
    }

    fn install(&self, project_root: &Path) -> Result<()> {
        install_websocket(project_root)
    }

    fn uninstall(&self, project_root: &Path) -> Result<()> {
        use colored::Colorize;

        println!("{}", "Uninstalling WebSocket support...".bold());

        // Delete files
        if super::remove_file_if_exists(&project_root.join("backend/src/ws.rs"))? {
            println!("  {} backend/src/ws.rs", "delete".red());
        }
        if super::remove_file_if_exists(
            &project_root.join("frontend/src/lib/useWebSocket.ts"),
        )? {
            println!("  {} frontend/src/lib/useWebSocket.ts", "delete".red());
        }

        // Remove mod declaration from main.rs
        super::remove_mod_from_main(project_root, "ws")?;

        // Remove ws_handler route from routes/mod.rs
        super::remove_line_from_file(
            &project_root.join("backend/src/routes/mod.rs"),
            "ws_handler",
        )?;

        // Remove feature flag
        super::remove_feature_flag(project_root, "websocket")?;

        // Regenerate AI context
        crate::ai_context::regenerate(project_root).ok();

        println!();
        println!(
            "{}",
            "WebSocket support uninstalled successfully.".green().bold()
        );

        Ok(())
    }
}

fn install_websocket(project_root: &Path) -> Result<()> {
    use crate::template::TemplateEngine;
    use crate::utils;
    use colored::Colorize;
    use tera::Context;

    println!("{}", "Installing WebSocket support...".bold());

    let engine = TemplateEngine::new()?;
    let ctx = Context::new();

    // Generate backend ws module
    let content = engine.render("addon/websocket/ws.rs.tera", &ctx)?;
    utils::write_file(&project_root.join("backend/src/ws.rs"), &content)?;
    println!("  {} backend/src/ws.rs", "create".green());

    // Generate frontend useWebSocket hook
    let content = engine.render("addon/websocket/useWebSocket.ts.tera", &ctx)?;
    utils::write_file(
        &project_root.join("frontend/src/lib/useWebSocket.ts"),
        &content,
    )?;
    println!("  {} frontend/src/lib/useWebSocket.ts", "create".green());

    // Add `mod ws;` to main.rs if not present
    super::add_mod_to_main(project_root, "ws")?;

    // Inject WS route into routes/mod.rs via MIDDLEWARE marker
    utils::insert_at_marker(
        &project_root.join("backend/src/routes/mod.rs"),
        "// === ROMANCE:MIDDLEWARE ===",
        "        .route(\"/ws\", axum::routing::get(crate::ws::ws_handler))",
    )?;
    println!(
        "  {} backend/src/routes/mod.rs (added /ws route)",
        "update".green()
    );

    // Add WebSocketState to AppState in routes/mod.rs
    let routes_path = project_root.join("backend/src/routes/mod.rs");
    let routes_content = std::fs::read_to_string(&routes_path)?;

    if !routes_content.contains("pub ws:") {
        // Add use import for ws module
        let routes_content = if !routes_content.contains("use crate::ws::WebSocketState;") {
            routes_content.replace(
                "use crate::events::EventBus;",
                "use crate::events::EventBus;\nuse crate::ws::WebSocketState;",
            )
        } else {
            routes_content
        };

        // Add ws field to AppState struct
        let routes_content = routes_content.replace(
            "    pub event_bus: EventBus,\n}",
            "    pub event_bus: EventBus,\n    pub ws: WebSocketState,\n}",
        );

        // Add WebSocketState construction and event bridge spawn in create_router
        let routes_content = routes_content.replace(
            "    let event_bus = EventBus::new();\n    let state = AppState { db, event_bus };",
            "    let event_bus = EventBus::new();\n    let ws = WebSocketState::new();\n\n    // Bridge entity events to WebSocket clients\n    tokio::spawn(crate::ws::bridge_events(event_bus.clone(), ws.clone()));\n\n    let state = AppState { db, event_bus, ws };",
        );

        std::fs::write(&routes_path, routes_content)?;
        println!(
            "  {} backend/src/routes/mod.rs (added WebSocketState to AppState)",
            "update".green()
        );
    }

    // Add axum ws feature to Cargo.toml
    // The scaffold already has axum = { version = "0.8", features = ["json"] }
    // We need to add the "ws" feature
    let cargo_path = project_root.join("backend/Cargo.toml");
    let cargo_content = std::fs::read_to_string(&cargo_path)?;
    if cargo_content.contains("axum") && !cargo_content.contains("\"ws\"") {
        let new_content = cargo_content.replace(
            r#"features = ["json"]"#,
            r#"features = ["json", "ws"]"#,
        );
        std::fs::write(&cargo_path, new_content)?;
        println!(
            "  {} backend/Cargo.toml (added ws feature to axum)",
            "update".green()
        );
    }

    // Update romance.toml
    super::update_feature_flag(project_root, "websocket", true)?;

    println!();
    println!(
        "{}",
        "WebSocket support installed successfully!".green().bold()
    );
    println!("  Backend: WebSocket endpoint at /ws");
    println!("  Frontend: import {{ useWebSocket }} from '@/lib/useWebSocket'");
    println!("  Entity events are automatically broadcast to connected clients.");
    println!();
    println!("  Usage example (frontend):");
    println!("    const {{ messages, sendMessage, isConnected }} = useWebSocket('ws://localhost:3000/ws');");

    Ok(())
}
