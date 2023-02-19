build:
	cargo b

info PID: build
	sudo RUST_BACKTRACE=full RUST_LOG=info ./target/debug/dumper {{PID}} -N 0x83fd578 -O 0x849f2b0

debug PID: build
	sudo RUST_BACKTRACE=full RUST_LOG=debug ./target/debug/dumper {{PID}} -N 0x83fd578 -O 0x849f2b0

trace PID: build
	sudo RUST_BACKTRACE=full RUST_LOG=trace ./target/debug/dumper {{PID}} -N 0x83fd578 -O 0x849f2b0
