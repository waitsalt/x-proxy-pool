use anyhow::Result;
use model::PROXY_POOL;

pub mod model;

pub async fn init() -> Result<()> {
    PROXY_POOL.load().await?;
    PROXY_POOL.test().await?;
    Ok(())
}
