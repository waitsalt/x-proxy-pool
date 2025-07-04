use anyhow::Result;
use model::PROXY_POOL;
use tracing::info;

pub mod model;

pub async fn init() -> Result<()> {
    PROXY_POOL.load().await?;
    PROXY_POOL.test().await?;

    info!("代理池启动成功");
    Ok(())
}
