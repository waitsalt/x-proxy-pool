use anyhow::Result;
use tokio::signal;
use tracing::{error, info};
use x_proxy_pool::{
    common::{self, config::CONFIG},
    protocol::{http::http_proxy, model::Protocol, socks5::socks5_proxy},
    proxy::{self, model::PROXY_POOL},
    util::check_proxy_protocol,
};

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化通用模块
    common::init()?;
    proxy::init().await?;

    // 启动服务
    let server_handle = tokio::spawn(async move {
        let local_address = format!("{}:{}", CONFIG.server.host, CONFIG.server.port);
        if let Err(e) = run(&local_address).await {
            error!("服务器错误: {}", e);
        }
    });

    tokio::select! {
        _ = signal::ctrl_c() => {
            info!("接收到 Ctrl+C 信号，正在关闭服务...");
        }
    }

    // 中止服务器任务
    server_handle.abort();

    Ok(())
}

pub async fn run(address: &str) -> Result<()> {
    let listener = tokio::net::TcpListener::bind(address).await?;
    info!("服务启动在: {}", address);

    loop {
        match listener.accept().await {
            Ok((source_stream, source_address)) => {
                info!("接受到新连接: {}", source_address);
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(source_stream).await {
                        error!("连接处理出错: {}", e);
                    }
                });
            }
            Err(e) => {
                error!("接受连接失败: {}", e);
            }
        }
    }
}

pub async fn handle_connection(mut stream: tokio::net::TcpStream) -> Result<()> {
    let source_connect_protocol = check_proxy_protocol(&mut stream).await?;
    let (mut reader, mut writer) = stream.split();
    info!("代理协议为: {:?}", source_connect_protocol);
    match source_connect_protocol {
        Protocol::Http => {
            // 处理 HTTP 请求
            let proxy = PROXY_POOL.get(Protocol::Http).await?;
            if let Err(e) = http_proxy(&mut reader, &mut writer, &proxy).await {
                error!("处理 HTTP 请求出错: {}", e);
            }
        }
        Protocol::Socks5 => {
            // 处理 SOCKS5 请求
            let proxy = PROXY_POOL.get(Protocol::Socks5).await?;
            if let Err(e) = socks5_proxy(&mut reader, &mut writer, &proxy).await {
                error!("处理 SOCKS5 请求出错: {}", e);
            }
        }
    }
    Ok(())
}
