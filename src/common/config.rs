use std::{fs, path::Path};

use anyhow::Result;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

// 全局访问config
pub static CONFIG: Lazy<Config> = Lazy::new(|| init().unwrap());

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub server: Server,
    pub logger: Logger,
    pub proxy: Proxy,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Server {
    pub name: String,
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Logger {
    pub level: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Proxy {
    pub proxy_file: String,
    pub timeout: usize,
    pub health_check_interval: usize,
    pub retry_count: usize,
    pub auto_switch: bool,
    pub auto_switch_interval: usize,
    pub max_test_count: usize,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            server: Server {
                name: "proxy_pool".to_string(),
                host: "127.0.0.1".to_string(),
                port: 9000,
            },
            logger: Logger {
                level: "info".to_string(),
            },
            proxy: Proxy {
                proxy_file: "proxy.txt".to_string(),
                timeout: 3000,
                health_check_interval: 60,
                retry_count: 3,
                auto_switch: true,
                auto_switch_interval: 300,
                max_test_count: 10,
            },
        }
    }
}

pub fn init() -> Result<Config> {
    // 配置文件路径
    let config_path = Path::new("config.toml");

    // 配置文件不存在时，创建默认配置文件
    if !config_path.exists() {
        fs::write(config_path, DEFAULT_CONFIG)?;
    }

    // 读取配置文件内容并解析为 Config 结构体
    let content = fs::read_to_string(config_path)?;
    let config: Config = toml::from_str(&content)?;

    Ok(config)
}

const DEFAULT_CONFIG: &str = r#"[server]
name = "proxy_pool"
host = "127.0.0.1"
port = 9000

[logger]
level = "info"

[proxy]
proxy_file = "proxy.txt"
timeout = 3000
health_check_interval = 60
retry_count = 3
auto_switch = true
auto_switch_interval = 300
max_test_count = 10
"#;
