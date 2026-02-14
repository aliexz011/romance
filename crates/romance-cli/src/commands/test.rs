use anyhow::Result;
use std::path::Path;

pub fn run() -> Result<()> {
    let project_root = Path::new(".");

    if !project_root.join("romance.toml").exists() {
        anyhow::bail!("Not a Romance project (romance.toml not found)");
    }

    romance_core::test_runner::run_tests(project_root)
}
