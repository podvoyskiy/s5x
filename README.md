## s5x — SOCKS5 Proxy Server

**Linux only** (other OS not tested)

### Installation

#### Option 1: Cargo

***Requires:***
- Rust
- build-essential (Debian/Ubuntu: `sudo apt install build-essential`)

```sh
cargo install s5x
```

#### Option 2: Prebuilt binary (no dependencies)

```sh
wget https://github.com/podvoyskiy/s5x/releases/latest/download/s5x
chmod +x ./s5x
```

### Usage

```sh
s5x                             # listen on 127.0.0.1:1080 (default)
s5x --host 0.0.0.0 --port 9976  # listen on all interfaces
s5x --auth admin:12345          # with auth
```

> **Note:** If using prebuilt binary, replace `s5x` with `./s5x`

### Examples

```sh
curl -x socks5h://127.0.0.1:1080 https://httpbin.org/post -X POST -d '{"key":"value"}'
curl -x socks5://127.0.0.1:1080 http://httpbin.org/get
curl -x socks5://admin:12345@127.0.0.1:1080 http://httpbin.org/get
```

> **Note:** Use `socks5h://` for DNS resolving on the proxy side, `socks5://` for client-side DNS

## Other Crates

- `s5t` – client (currently: one-time SOCKS5 request to server, planned: TUN via SOCKS5, TUN with native protocol) – work in progress
- `s5l` – shared library