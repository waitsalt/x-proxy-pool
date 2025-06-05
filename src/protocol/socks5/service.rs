use anyhow::Result;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tracing::{error, trace};

use crate::proxy::model::Proxy;

pub async fn socks5_proxy<R, W>(reader: &mut R, writer: &mut W, proxy: &Proxy) -> Result<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    trace!("启动 SOCKS5 代理");

    // 1. 处理握手
    trace!("启动 SOCKS5 握手处理");
    // 读取客户端支持的认证方法
    let mut method_selection = [0u8; 2];
    reader.read_exact(&mut method_selection).await?;

    let nmethods = method_selection[1];
    let mut methods = vec![0u8; nmethods as usize];
    reader.read_exact(&mut methods).await?;

    // 不需要认证，回复使用无认证方法
    writer.write_all(&[0x05, 0x00]).await?;
    writer.flush().await?;

    trace!("结束 SOCKS5 握手处理");

    // 读取SOCKS5请求
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf).await?;

    if buf[0] != 0x05 || buf[1] != 0x01 {
        return Err(anyhow::anyhow!("不支持的SOCKS5命令"));
    }

    // 读取目标地址
    let addr_type = buf[3];
    let target_addr = match addr_type {
        0x01 => {
            // IPv4
            let mut addr = [0u8; 4];
            reader.read_exact(&mut addr).await?;
            format!("{}.{}.{}.{}", addr[0], addr[1], addr[2], addr[3])
        }
        0x03 => {
            // 域名
            let len = reader.read_u8().await? as usize;
            let mut domain = vec![0u8; len];
            reader.read_exact(&mut domain).await?;
            String::from_utf8(domain)?
        }
        0x04 => {
            // IPv6
            let mut addr = [0u8; 16];
            reader.read_exact(&mut addr).await?;
            format!(
                "{:02x}{:02x}:{:02x}{:02x}:{:02x}{:02x}:{:02x}{:02x}:{:02x}{:02x}:{:02x}{:02x}:{:02x}{:02x}:{:02x}{:02x}",
                addr[0],
                addr[1],
                addr[2],
                addr[3],
                addr[4],
                addr[5],
                addr[6],
                addr[7],
                addr[8],
                addr[9],
                addr[10],
                addr[11],
                addr[12],
                addr[13],
                addr[14],
                addr[15]
            )
        }
        _ => return Err(anyhow::anyhow!("不支持的地址类型")),
    };

    // 读取端口
    let port = reader.read_u16().await?;
    let _target = format!("{}:{}", target_addr, port);

    // 获取代理
    let mut upstream = match tokio::net::TcpStream::connect(proxy.address()).await {
        Ok(stream) => stream,
        Err(e) => {
            error!("代理连接失败: {} - {}", proxy.address(), e);
            // 发送失败响应
            let response = [0x05, 0x04, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
            writer.write_all(&response).await?;
            return Ok(());
        }
    };

    // 与上游SOCKS5服务器进行握手
    upstream.write_all(&[0x05, 0x01, 0x00]).await?;
    let mut response = [0u8; 2];
    upstream.read_exact(&mut response).await?;

    if response[0] != 0x05 || response[1] != 0x00 {
        eprintln!("上游代理握手失败");
        return Ok(());
    }

    // 发送连接请求到上游代理
    let mut request = Vec::new();
    request.extend_from_slice(&[0x05, 0x01, 0x00]); // VER, CMD, RSV

    match addr_type {
        0x01 => {
            // IPv4
            request.push(0x01);
            for octet in target_addr.split('.') {
                request.push(octet.parse::<u8>()?);
            }
        }
        0x03 => {
            // Domain
            request.push(0x03);
            request.push(target_addr.len() as u8);
            request.extend_from_slice(target_addr.as_bytes());
        }
        0x04 => {
            // IPv6
            request.push(0x04);

            // 将 IPv6 地址字符串转换为 16 字节的数组
            let addr = target_addr
                .split(':')
                .map(|part| u16::from_str_radix(part, 16).unwrap_or(0))
                .collect::<Vec<_>>();

            // 将每组 2 字节的十六进制数转换为大端字节序并添加到请求中
            for &value in &addr {
                request.extend_from_slice(&value.to_be_bytes());
            }
        }
        _ => unreachable!(),
    }

    // 添加端口
    request.extend_from_slice(&port.to_be_bytes());

    // 发送请求到上游代理
    upstream.write_all(&request).await?;

    // 读取上游代理响应
    let mut response = [0u8; 4];
    upstream.read_exact(&mut response).await?;

    if response[1] != 0x00 {
        error!("上游代理连接目标失败");
        let response = [0x05, 0x04, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        writer.write_all(&response).await?;
        return Ok(());
    }

    // 跳过绑定地址和端口
    match response[3] {
        0x01 => {
            // IPv4
            let mut addr = [0u8; 4];
            upstream.read_exact(&mut addr).await?;
        }
        0x03 => {
            // Domain
            let len = upstream.read_u8().await?;
            let mut domain = vec![0u8; len as usize];
            upstream.read_exact(&mut domain).await?;
        }
        0x04 => {
            // IPv6
            let mut addr = [0u8; 16];
            upstream.read_exact(&mut addr).await?;
        }
        _ => return Err(anyhow::anyhow!("上游代理返回了不支持的地址类型")),
    }
    let mut port = [0u8; 2];
    upstream.read_exact(&mut port).await?;

    // 发送成功响应给客户端
    let response = [0x05, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    writer.write_all(&response).await?;

    // 双向转发数据
    let (mut upstream_reader, mut upstream_writer) = upstream.into_split();
    let client_to_proxy = tokio::io::copy(reader, &mut upstream_writer);
    let proxy_to_client = tokio::io::copy(&mut upstream_reader, writer);

    tokio::select! {
        res = client_to_proxy => {
            if let Err(e) = res {
                error!("客户端到代理传输错误: {}", e);
            }
        },
        res = proxy_to_client => {
            if let Err(e) = res {
                error!("代理到客户端传输错误: {}", e);
            }
        }
    }
    Ok(())
}
