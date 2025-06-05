use std::{
    collections::HashSet,
    fs::{self, File},
    io::{self, BufRead},
    sync::Arc,
    time::Duration,
};

use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use once_cell::sync::Lazy;
use tokio::{sync::RwLock, time::timeout};
use tracing::info;

use crate::{common::config::CONFIG, protocol::model::Protocol};

pub struct ProxyPool {
    pub http_index: Arc<RwLock<usize>>,
    pub http_proxy_list: Arc<RwLock<Vec<Proxy>>>,
    pub socks5_index: Arc<RwLock<usize>>,
    pub socks5_proxy_list: Arc<RwLock<Vec<Proxy>>>,
}

#[derive(Debug, Clone)]
pub struct Proxy {
    pub scheme: Protocol,
    pub host: String,
    pub port: u16,
}

impl Proxy {
    pub fn new(scheme: Protocol, host: String, port: u16) -> Self {
        Proxy { scheme, host, port }
    }

    pub fn from(str: &str) -> Result<Self> {
        let parts: Vec<&str> = str.split(':').collect();
        let length = parts.len();
        if length == 3 {
            let scheme = match parts[0] {
                "http" => Protocol::Http,
                "socks5" => Protocol::Socks5,
                _ => return Err(anyhow::anyhow!("不支持的协议")),
            };
            let host = parts[1].to_string().split_off(2);
            let port = parts[2].parse().unwrap_or(80);
            return Ok(Proxy { scheme, host, port });
        } else if length == 2 {
            let scheme = Protocol::Http;
            let host = parts[0].to_string();
            let port = parts[1].parse().unwrap_or(80);
            return Ok(Proxy { scheme, host, port });
        } else {
            return Err(anyhow::anyhow!(
                "期待的格式为: scheme://host:port 或 host:port"
            ));
        }
    }

    pub fn show(&self) -> String {
        let scheme = match self.scheme {
            Protocol::Http => "http",
            Protocol::Socks5 => "socks5",
        };
        format!("{}://{}:{}", scheme, self.host, self.port)
    }

    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub async fn test(&self) -> Result<bool> {
        // Implement test logic here
        let proxy = self.show();

        let client = reqwest::Client::builder()
            .proxy(reqwest::Proxy::all(proxy)?)
            .build()?;

        let res = timeout(
            Duration::from_millis(CONFIG.proxy.timeout as u64),
            client.get("https://api.ipify.org").send(),
        )
        .await??;

        let res = res.text().await?;

        if res == self.host {
            // info!("节点测试成功: {}", self.show());
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

impl ProxyPool {
    pub fn new() -> Self {
        Self {
            http_index: Arc::new(RwLock::new(0)),
            http_proxy_list: Arc::new(RwLock::new(Vec::new())),
            socks5_index: Arc::new(RwLock::new(0)),
            socks5_proxy_list: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn load(&self) -> Result<()> {
        let path = CONFIG.proxy.proxy_file.clone();
        let file = File::open(&path)?;
        let reader = io::BufReader::new(file);
        let mut proxies = HashSet::new();

        // 读取并去重代理地址
        for line in reader.lines() {
            let line = line?;
            if !line.trim().is_empty() {
                proxies.insert(line.trim().to_string());
            }
        }

        if proxies.is_empty() {
            return Err(anyhow::anyhow!("代理文件为空"));
        }

        let proxy_list: Vec<String> = proxies.into_iter().collect();

        let mut http_proxy_pool = Vec::new();
        let mut socks5_proxy_pool = Vec::new();

        for proxy in proxy_list {
            let proxy = if let Ok(proxy) = Proxy::from(&proxy) {
                proxy
            } else {
                continue;
            };
            match proxy.scheme {
                Protocol::Http => http_proxy_pool.push(proxy),
                Protocol::Socks5 => socks5_proxy_pool.push(proxy),
            }
        }

        let mut proxy_list = Vec::new();
        proxy_list.extend_from_slice(&http_proxy_pool);
        proxy_list.extend_from_slice(&socks5_proxy_pool);

        let mut http_proxy_list = self.http_proxy_list.write().await;
        let mut socks5_proxy_list = self.socks5_proxy_list.write().await;

        *http_proxy_list = http_proxy_pool.clone();
        *socks5_proxy_list = socks5_proxy_pool.clone();

        let mut http_index = self.http_index.write().await;
        let mut socks5_index = self.socks5_index.write().await;

        *http_index = 0;
        *socks5_index = 0;

        drop(http_index);
        drop(socks5_index);

        let mut proxy_list = Vec::new();
        proxy_list.extend_from_slice(&http_proxy_pool);
        proxy_list.extend_from_slice(&socks5_proxy_pool);

        let proxy_list: Vec<String> = proxy_list.iter().map(|p| p.show()).collect();
        fs::write(&path, proxy_list.join("\n"))?;

        Ok(())
    }

    pub async fn save(&self) -> Result<()> {
        let path = CONFIG.proxy.proxy_file.clone();
        let http_proxy_list = self.http_proxy_list.read().await;
        let socks5_proxy_list = self.socks5_proxy_list.read().await;

        let mut proxy_list = Vec::new();
        proxy_list.extend_from_slice(&http_proxy_list);
        proxy_list.extend_from_slice(&socks5_proxy_list);

        let proxy_list: Vec<String> = proxy_list.iter().map(|p| p.show()).collect();
        fs::write(&path, proxy_list.join("\n"))?;

        Ok(())
    }

    pub async fn update(&self, proxy_list: Vec<Proxy>) -> Result<()> {
        let mut http_proxy_pool = Vec::new();
        let mut socks5_proxy_pool = Vec::new();

        for proxy in proxy_list.into_iter() {
            match proxy.scheme {
                Protocol::Http => http_proxy_pool.push(proxy),
                Protocol::Socks5 => socks5_proxy_pool.push(proxy),
            }
        }

        let mut http_proxy_list = self.http_proxy_list.write().await;
        let mut socks5_proxy_list = self.socks5_proxy_list.write().await;

        *http_proxy_list = http_proxy_pool.clone();
        *socks5_proxy_list = socks5_proxy_pool.clone();

        let mut http_index = self.http_index.write().await;
        let mut socks5_index = self.socks5_index.write().await;

        *http_index = 0;
        *socks5_index = 0;
        Ok(())
    }

    pub async fn test(&self) -> Result<()> {
        let max_test_count = CONFIG.proxy.max_test_count;
        let http_proxy_list = self.http_proxy_list.read().await;
        let socks5_proxy_list = self.socks5_proxy_list.read().await;

        let mut proxy_list = Vec::new();
        proxy_list.extend_from_slice(&http_proxy_list);
        proxy_list.extend_from_slice(&socks5_proxy_list);

        let total = proxy_list.len();

        if total == 0 {
            return Ok(());
        }

        println!(
            "{} {} {}",
            format!("开始代理检测..."),
            format!(
                "共有代理: {} http代理： {}, socks5代理: {}",
                total,
                http_proxy_list.len(),
                socks5_proxy_list.len()
            ),
            format!("并发数: {}", max_test_count)
        );

        drop(http_proxy_list);
        drop(socks5_proxy_list);

        // 创建进度条
        let pb = if true {
            let pb = ProgressBar::new(total as u64);
            pb.set_style(ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
                .unwrap()
                .progress_chars("#>-"));
            Some(Arc::new(pb))
        } else {
            None
        };

        // 创建信号量控制并发数
        let semaphore = Arc::new(tokio::sync::Semaphore::new(max_test_count));
        let valid_proxies = Arc::new(tokio::sync::Mutex::new(Vec::new()));
        let mut handles = Vec::with_capacity(total);

        for proxy in proxy_list {
            let semaphore = semaphore.clone();
            let pb = pb.clone();
            let valid_proxies = valid_proxies.clone();
            // let (addr, entry) = each_item(proxy);

            let handle = tokio::spawn(async move {
                // 获取信号量许可
                let _permit = semaphore.acquire().await.unwrap();

                // 测试代理
                let result = proxy.test().await;

                // 更新进度条
                if let Some(pb) = &pb {
                    pb.inc(1);
                }

                // 如果测试成功，添加到有效代理列表
                if let Ok(res) = result {
                    if res {
                        let mut proxies = valid_proxies.lock().await;
                        proxies.push(proxy);
                    }
                }
            });

            handles.push(handle);
        }

        // 等待所有测试完成
        for handle in handles {
            _ = handle.await
        }

        // 结束进度条
        if let Some(pb) = pb {
            pb.finish_with_message(format!("测试完成"));
        }

        // 获取有效代理并排序
        let proxies = Arc::try_unwrap(valid_proxies)
            .expect("获取有效代理失败")
            .into_inner();

        // 按延迟排序
        // proxies.sort_by(|a, b| a.latency.cmp(&b.latency));

        self.update(proxies).await?;
        self.save().await?;

        Ok(())
    }

    pub async fn get(&self, scheme: Protocol) -> Result<Proxy> {
        let proxy = match scheme {
            Protocol::Http => {
                let http_proxy_list = self.http_proxy_list.read().await;
                let mut http_index = self.http_index.write().await;

                *http_index = (*http_index + 1) % http_proxy_list.len();
                http_proxy_list[*http_index].clone()
            }
            Protocol::Socks5 => {
                let socks5_proxy_list = self.socks5_proxy_list.read().await;
                let mut socks5_index = self.socks5_index.write().await;

                *socks5_index = (*socks5_index + 1) % socks5_proxy_list.len();
                socks5_proxy_list[*socks5_index].clone()
            }
        };
        info!("当前使用: {}", proxy.show());
        Ok(proxy)
    }

    pub async fn next(&self, scheme: Protocol) -> Result<Proxy> {
        let proxy = match scheme {
            Protocol::Http => {
                let http_proxy_list = self.http_proxy_list.read().await;
                let http_index = self.http_index.write().await;

                let index = (*http_index + 1) % http_proxy_list.len();
                http_proxy_list[index].clone()
            }
            Protocol::Socks5 => {
                let socks5_proxy_list = self.socks5_proxy_list.read().await;
                let socks5_index = self.socks5_index.write().await;

                let index = (*socks5_index + 1) % socks5_proxy_list.len();
                socks5_proxy_list[index].clone()
            }
        };
        Ok(proxy)
    }
}

pub fn init() -> Result<Arc<ProxyPool>> {
    let proxy_pool = Arc::new(ProxyPool::new());
    Ok(proxy_pool)
}

// 全局访问config
pub static PROXY_POOL: Lazy<Arc<ProxyPool>> = Lazy::new(|| init().unwrap());
