#!/bin/cmake

build:
	cargo build --release

# if FastNoise2 is not installed
install-fn2:
	install -st /usr/local/lib/ FastNoise2/lib/libFastNoise.so