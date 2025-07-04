# x-proxy-pool

一个基于 Rust 实现的高性能代理池服务，支持 HTTP 和 SOCKS5 协议，提供代理自动切换、健康检查等功能。

## 功能特性

- **多协议支持**：支持 HTTP 和 SOCKS5 代理协议。
- **自动切换**：支持代理自动切换，提高可用性。
- **健康检查**：定期检测代理可用性，自动剔除无效代理。
- **高性能**：基于异步 I/O（`tokio`）实现，支持高并发。
- **配置灵活**：通过 `config.toml` 文件自定义服务参数。

## 快速开始

### 安装依赖

确保已安装 Rust 工具链（推荐使用 `rustup`）：

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 构建项目

```bash
cargo build --release
```

### 配置文件

项目根目录下的 `config.toml` 是配置文件，默认内容如下：

```toml
[server]
name = "proxy_pool"
host = "127.0.0.1"
port = 9000

[logger]
level = "info"

[proxy]
proxy_file = "proxy.txt"           # 代理列表文件路径
timeout = 3000                     # 代理测试超时时间（毫秒）
health_check_interval = 60         # 健康检查间隔（秒）
retry_count = 3                    # 失败重试次数
auto_switch = true                 # 是否自动切换代理
auto_switch_interval = 300         # 自动切换间隔（秒）
max_test_count = 200               # 最大并发测试数
```

### 代理列表

代理列表文件 `proxy.txt` 的格式如下（每行一个代理地址）：

```
http://139.159.106.134:443
socks5://192.111.137.35:4145
```

### 运行服务

```bash
cargo run --release
```

服务启动后，默认监听 `127.0.0.1:9000`。

## 使用示例

### HTTP 代理

```bash
curl -x http://127.0.0.1:9000 http://example.com
```

### SOCKS5 代理

```bash
curl --socks5 127.0.0.1:9000 http://example.com
```

## 项目结构

```
x-proxy-pool/
├── config.toml            # 配置文件
├── proxy.txt              # 代理列表文件
├── src/
│   ├── common/            # 通用模块（日志、配置）
│   ├── protocol/          # 协议实现（HTTP/SOCKS5）
│   ├── proxy/             # 代理池管理
│   ├── util/              # 工具函数
│   ├── lib.rs             # 库入口
│   └── main.rs            # 主程序入口
└── Cargo.toml             # 项目依赖配置
```

## 许可证

本项目采用 [GNU General Public License v3.0](LICENSE)。
