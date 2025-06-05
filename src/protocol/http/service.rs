use anyhow::Result;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tracing::{error, info, trace};

use crate::proxy::model::Proxy;

pub async fn http_proxy<R, W>(reader: &mut R, writer: &mut W, proxy: &Proxy) -> Result<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    trace!("启动 HTTP 代理");

    // 读取客户端请求
    let mut request = Vec::new();
    let mut buf = [0u8; 1024];
    loop {
        let n = reader.read(&mut buf).await?;
        request.extend_from_slice(&buf[..n]);
        if n < 1024 {
            break;
        }
    }
    trace!("请求头: {}", String::from_utf8_lossy(&request));

    // 解析请求头
    let mut headers = vec![httparse::EMPTY_HEADER; 16];
    let mut req = httparse::Request::new(&mut headers);

    let _res = loop {
        match req.parse(&request) {
            Ok(status) => break status,
            Err(httparse::Error::TooManyHeaders) => {
                headers.extend_from_slice(&[httparse::EMPTY_HEADER; 16]);
                req = httparse::Request::new(&mut headers);
            }
            Err(e) => return Err(anyhow::anyhow!("请求解析失败: {:?}", e)),
        }
    };

    // 获取请求方法和路径
    let method = req.method.ok_or_else(|| anyhow::anyhow!("缺失请求方法"))?;
    let path = req.path.ok_or_else(|| anyhow::anyhow!("缺失请求路径"))?;

    // 连接目标服务器
    let mut proxy_stream = match tokio::net::TcpStream::connect(proxy.address()).await {
        Ok(stream) => {
            info!("成功连接到目标服务器: {}", proxy.show());
            stream
        }
        Err(e) => {
            error!("无法连接到目标服务器: {}", e);
            let response = b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\n\r\n";
            writer.write_all(response).await?;
            return Ok(());
        }
    };

    if method == "CONNECT" {
        trace!("处理 CONNECT 请求: {}", path);
        let addr = path.split(':').collect::<Vec<_>>();
        if addr.len() != 2 {
            return Err(anyhow::anyhow!("无效的 CONNECT 目标地址: {}", path));
        }
        // 发送 CONNECT 请求到代理服务器
        let connect_request = format!("CONNECT {} HTTP/1.1\r\nHost: {}\r\n\r\n", path, path);
        proxy_stream.write_all(connect_request.as_bytes()).await?;
    } else {
        // 转发客户端请求到目标服务器
        proxy_stream.write_all(&request).await?;
        info!("已转发客户端请求");
    }

    // 转发目标服务器响应到客户端
    let (mut proxy_reader, mut proxy_writer) = proxy_stream.into_split();
    let client_to_proxy = tokio::io::copy(reader, &mut proxy_writer);
    let proxy_to_client = tokio::io::copy(&mut proxy_reader, writer);

    tokio::select! {
        res = client_to_proxy => {
            if let Err(e) = res {
                error!("客户端到服务器传输错误: {}", e);
            }
        },
        res = proxy_to_client => {
            if let Err(e) = res {
                error!("服务器到客户端传输错误: {}", e);
            }
        }
    }

    info!("结束 HTTP 代理");
    Ok(())
}
