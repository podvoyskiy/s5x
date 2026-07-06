server:
	cargo run --bin s5x

server-auth:
	cargo run --bin s5x -- --auth admin:12345

server-xor:
	cargo run --bin s5x -- --xor 0xAA

client:
	cargo run --bin s5t -- --target http://34.234.10.121/get?key=value

client-https:
	cargo run --bin s5t -- --target https://httpbin.org/post --data '{"key":"value"}'

client-xor:
	cargo run --bin s5t -- --xor 0xAA --target https://httpbin.org/post --data '{"key":"value"}'

client-tun:
	cargo build --release --target x86_64-unknown-linux-musl --bin s5t
	#sudo setcap cap_net_admin,cap_net_raw=+ep target/x86_64-unknown-linux-musl/release/s5t
	sudo RUST_LOG=trace target/x86_64-unknown-linux-musl/release/s5t --mode tun2socks --address 10.0.0.9

test: test-server test-client test-lib

test-server:
	cargo test -p s5x

test-client:
	cargo test -p s5t

test-lib:
	cargo test -p s5l

build:
	cargo build --release --target x86_64-unknown-linux-musl

s: server
sa: server-auth
sx: server-xor
c: client
ch: client-https
cx: client-xor
ct: client-tun
t: test
ts: test-server
tc: test-client
tl: test-lib
b: build