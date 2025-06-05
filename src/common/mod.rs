use anyhow::Result;

pub mod config;
pub mod logger;

pub fn init() -> Result<()> {
    logger::init()?;
    tracing::info!("日志记录初始化成功");
    Ok(())
}
