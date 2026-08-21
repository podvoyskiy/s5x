# s5x — SOCKS5 Proxy Tools

**Linux only** (other OS not tested)

The project contains the following crates:
- **s5x** — SOCKS5 proxy server
- **s5t** — SOCKS5 client with tun2socks support
- **s5l** — common library used by both

---

## s5x — SOCKS5 Proxy Server

### Installation

#### Option 1: Cargo

***Requires:***
- Rust
- build-essential (Debian/Ubuntu: `sudo apt install build-essential`)

```sh
cargo install s5x
```

#### Option 2: Prebuilt binary (no dependencies)

Download latest binary `s5x` from [Releases](https://github.com/podvoyskiy/s5x/releases)

```sh
chmod +x ./s5x
```

### Usage

```sh
s5x                             # listen on 127.0.0.1:1080 (default)
s5x --host 0.0.0.0 --port 9976  # listen on all interfaces
s5x --auth admin:12345          # with auth
s5x --xor 0xAA                  # enable XOR obfuscation
```

> **Note:** If using prebuilt binary, replace `s5x` with `./s5x`

### Options

| Option   | Description                      | Default     |
| -------- | -------------------------------- | ----------- |
| `--host` | bind address                     | `127.0.0.1` |
| `--port` | port to listen on                | `1080`      |
| `--auth` | username:password authentication | `None`      |
| `--xor`  | XOR key for obfuscation (hex)    | `None`      |

> **Note:** `--xor` obfuscation only works when both client and server use the same key. Client must be `s5t`

### Examples

```sh
curl -x socks5h://127.0.0.1:1080 https://httpbin.org/post -X POST -d '{"key":"value"}'

curl -x socks5://127.0.0.1:1080 http://httpbin.org/get

curl -x socks5://admin:12345@127.0.0.1:1080 http://httpbin.org/get
```

> **Note:** Use `socks5h://` for DNS resolving on the proxy side, `socks5://` for client-side DNS

---

## s5t — SOCKS5 Client

Currently supports two modes:
- **socks5** — One-time SOCKS5 request (HTTP/HTTPS)
- **tun2socks** — creates a TUN interface and forwards TCP traffic through a SOCKS5 proxy

### Installation

#### Option 1: Cargo

```sh
cargo install s5t

# Optional: allow running without sudo (for tun2socks with --fakedns false)
sudo setcap 'CAP_NET_ADMIN+ep' ~/.cargo/bin/s5t
```

#### Option 2: Prebuilt binary

Download latest binary `s5t` from [Releases](https://github.com/podvoyskiy/s5x/releases)

```sh
chmod +x ./s5t

# Optional: allow running without sudo (for tun2socks with --fakedns false)
sudo setcap 'CAP_NET_ADMIN+ep' ./s5t
```

### Usage

-  #### socks5 Mode (One-time request)

```sh
# HTTP GET request
s5t --target http://httpbin.org/get --server 127.0.0.1:1080

# HTTPS POST with JSON data
s5t --target https://httpbin.org/post --data '{"key":"value"}'

# With authentication
s5t --target https://httpbin.org/post --auth admin:12345 --data '{"key":"value"}'

# With custom headers
s5t --target https://httpbin.org/get --headers "User-Agent:curl/8.5.0" --headers "Authorization:Bearer qwerty123"

# With custom method
s5t --target https://httpbin.org/delete --method DELETE
```
> **Note:** If using prebuilt binary, replace `s5t` with `./s5t`

-  #### tun2socks Mode

```sh
# Basic usage (replace with your actual server)
sudo s5t --mode tun2socks --address 10.0.0.9 --server 127.0.0.1:1080

# With authentication and XOR obfuscation
sudo s5t --mode tun2socks --address 10.0.0.9 --server 127.0.0.1:1080 --auth admin:12345 --xor 0xAA

# Run without fake DNS interception (and without sudo if CAP_NET_ADMIN granted)
s5t --mode tun2socks --address 10.0.0.9 --server 127.0.0.1:1080 --fakedns false
```
> **Note:** If you installed via `cargo install`, `sudo` may not see the binary.  
> Use the full path: `sudo ~/.cargo/bin/s5t ...`

### Options

| Option      | Description                       | Mode support | Default          |
| ----------- | --------------------------------- | -------------| ---------------- |
| `--mode`    | `tun2socks` or `socks5`           | both         | `socks5`         |
| `--server`  | SOCKS5 server address (host:port) | both         | `127.0.0.1:1080` |
| `--auth`    | username:password authentication  | both         | `None`           |
| `--xor`     | XOR key for obfuscation (hex)     | both         | `None`           |
| `--address` | TUN interface IP                  | `tun2socks`  | `10.0.0.9`       |
| `--fakedns` | Enable fake DNS interception      | `tun2socks`  | `true`           |
| `--target`  | target URL                        | `socks5`     | required         |
| `--method`  | HTTP method (GET, POST, etc.)     | `socks5`     | `GET`            |
| `--data`    | request body data                 | `socks5`     | `None`           |
| `--headers` | custom HTTP headers               | `socks5`     | `None`           |

### tun2socks: How It Works

1. Creates a TUN interface with the specified IP
2. Sets up routing rules to forward all traffic through the TUN
3. Intercepts DNS requests and returns fake IPs (if `--fakedns true`)
4. Forwards TCP traffic through the SOCKS5 proxy
5. Cleanup is automatic on exit (Ctrl+C)

> **Note:** `sudo` is required to create TUN interface and modify routing tables. With `--fakedns false` granting `CAP_NET_ADMIN` is enough to run without `sudo`

### Example: Full Setup

```sh
# Terminal 1 (on the server): start the proxy server
s5x --host 0.0.0.0 --port 1080

# Terminal 2 (on the client): start the client (requires sudo)
# Replace 127.0.0.1:1080 with your actual server
sudo s5t --mode tun2socks --address 10.0.0.9 --server 127.0.0.1:1080

# Terminal 3: test
curl google.com # Should work through the proxy
dig google.com  # DNS should return fake IP (100.64.0.x)
```
