use anyhow::Result;

use super::config::CONFIG;

pub fn init() -> Result<()> {
    // if std::env::var_os("RUST_LOG").is_none() {
    //     let app_name =
    //         std::env::var("CARGO_PKG_NAME").unwrap_or_else(|_| CONFIG.server.name.clone());
    //     let level = CONFIG.logger.level.as_str();
    //     let env = format!("{app_name}={level}");
    //     unsafe {
    //         std::env::set_var("RUST_LOG", env);
    //     }
    // }
    tracing_subscriber::fmt()
        // .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    Ok(())
}
