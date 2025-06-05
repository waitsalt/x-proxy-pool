use anyhow::Result;

use crate::protocol::model::Protocol;

pub async fn check_proxy_protocol(stream: &mut tokio::net::TcpStream) -> Result<Protocol> {
    let mut buf = [0u8; 8];
    let n = stream.peek(&mut buf).await?;

    if n >= 2 {
        // 1. 检查 SOCKS5 (第一个字节是 0x05)
        if buf[0] == 0x05 {
            return Ok(Protocol::Socks5);
        }
        // 2. 检查 HTTP (开头是 GET/POST/HEAD 等)
        else if let Ok(s) = std::str::from_utf8(&buf[..n]) {
            if s.starts_with("GET ")
                || s.starts_with("PUT ")
                || s.starts_with("POST ")
                || s.starts_with("HEAD ")
                || s.starts_with("DELETE ")
                || s.starts_with("CONNECT ")
            {
                return Ok(Protocol::Http);
            }
        }
    }

    // 如果都不匹配，返回错误
    Err(anyhow::anyhow!("暂不支持的现协议"))
}
