build:
	cargo b

test PID: build
	sudo RUST_BACKTRACE=1 RUST_LOG=trace ./target/debug/dumper {{PID}} -N 0x83fd578 -O 0x849f2b0
