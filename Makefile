.PHONY: \
	server \
	server-auth \
	server-xor \
	client \
	client-https \
	client-xor \
	client-tun \
	client-tun-nofake \
	clean-routes \
	test \
	test-server \
	test-client \
	test-lib \
	build

-include .env

#variables used only for client-tun* and clean-routes
SERVER ?= 127.0.0.1:1080
SERVER_IP = $(firstword $(subst :, ,$(SERVER)))
TUN_DEV ?= tun0
TUN_ADDRESS ?= 10.0.0.9
XOR ?= 0xAA
AUTH ?= admin:12345

# ---------- Server ----------
server:
	cargo run --bin s5x

server-auth:
	cargo run --bin s5x -- --auth $(AUTH)

server-xor:
	cargo run --bin s5x -- --xor $(XOR)

# ---------- Client with mode socks5 ----------
client:
	cargo run --bin s5t -- --target http://34.234.10.121/get?key=value

client-https:
	cargo run --bin s5t -- --target https://httpbin.org/post --data '{"key":"value"}'

client-xor:
	cargo run --bin s5t -- --xor $(XOR) --target https://httpbin.org/post --data '{"key":"value"}'

# ---------- Client with mode tun2socks ----------
client-tun:
	cargo build --release --target x86_64-unknown-linux-musl --bin s5t
	sudo -E env RUST_LOG=trace target/x86_64-unknown-linux-musl/release/s5t \
		--mode tun2socks \
		--server $(SERVER) \
		--address $(TUN_ADDRESS) \
		--xor $(XOR) \
		--auth $(AUTH)

client-tun-nofake:
	cargo build --release --target x86_64-unknown-linux-musl --bin s5t
	sudo setcap 'CAP_NET_ADMIN+ep' target/x86_64-unknown-linux-musl/release/s5t
	env RUST_LOG=trace target/x86_64-unknown-linux-musl/release/s5t \
		--mode tun2socks \
		--server $(SERVER) \
		--fakedns false \
		--address $(TUN_ADDRESS) \
		--xor $(XOR) \
		--auth $(AUTH)

# ---------- TUN Routes Management ----------
# Emergency cleanup if something went wrong
# Use this if program crashed and left routes/rules behind
clean-routes:
	@echo "Cleaning up routes and rules..."
	-sudo ip link del $(TUN_DEV) 2>/dev/null || true
	-sudo ip rule del table 12345 2>/dev/null || true
	-sudo ip rule del to $(SERVER_IP) lookup main 2>/dev/null || true
	-sudo iptables -t nat -D OUTPUT -p udp --dport 53 -j DNAT --to-destination $(TUN_ADDRESS):53 2>/dev/null || true
	@echo "Cleanup completed"

# ---------- Testing ----------
test: test-server test-client test-lib

test-server:
	cargo test -p s5x -- --nocapture

test-client:
	cargo test -p s5t -- --nocapture

test-lib:
	cargo test -p s5l -- --nocapture

# ---------- Build ----------
build:
	cargo build --release --target x86_64-unknown-linux-musl

# ---------- Aliases ----------
s:   server
sa:  server-auth
sx:  server-xor
c:   client
ch:  client-https
cx:  client-xor
ct:  client-tun
ctn: client-tun-nofake
cr:  clean-routes
t:   test
ts:  test-server
tc:  test-client
tl:  test-lib
b:   build
h:   help

# ---------- Help ----------
help:
	@echo "Available targets:"
	@echo ""
	@echo "Server:"
	@echo "  server (s)              - Run server"
	@echo "  server-auth (sa)        - Run server with auth"
	@echo "  server-xor (sx)         - Run server with XOR"
	@echo ""
	@echo "Client:"
	@echo "  client (c)              - One-time request (HTTP)"
	@echo "  client-https (ch)       - One-time request (HTTPS)"
	@echo "  client-xor (cx)         - One-time request with XOR"
	@echo "  client-tun (ct)         - Run client with mode 'tun2socks'"
	@echo "  client-tun-nofake (ctn) - Run client without using fake dns"
	@echo ""
	@echo "TUN routes management:"
	@echo "  clean-routes (cr)       - Emergency cleanup of routes and rules"
	@echo ""
	@echo "Testing:"
	@echo "  test (t)                - Run all tests"
	@echo "  test-server (ts)        - Test server"
	@echo "  test-client (tc)        - Test client"
	@echo "  test-lib (tl)           - Test library"
	@echo ""
	@echo "Build:"
	@echo "  build (b)               - Build release binary"
	@echo ""
	@echo ".env variables or defaults (used only for client-tun* and clean-routes):"
	@echo "  SERVER      = $(SERVER)"
	@echo "  XOR         = $(XOR)"
	@echo "  AUTH        = $(AUTH)"
	@echo "  TUN_ADDRESS = $(TUN_ADDRESS)"
	@echo "  TUN_DEV     = $(TUN_DEV)"