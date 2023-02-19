build:
	cargo b

test PID: build
	sudo RUST_BACKTRACE=full RUST_LOG=info ./target/debug/dumper {{PID}} -N 0x83fd578 -O 0x849f2b0
