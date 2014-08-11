debug:
	cargo build && RUST_LOG=debug ./target/chätd
run:
	cargo build && ./target/chätd
check:
	rustc --no-trans src/main.rs
