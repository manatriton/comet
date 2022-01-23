use anyhow::Result;

fn main() -> Result<()> {
    comet::start_ui()?;
    Ok(())
}
