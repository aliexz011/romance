use anyhow::Result;

pub fn run(name: &str) -> Result<()> {
    romance_core::scaffold::create_project(name)
}
