SHELL := /bin/bash
PROJECT=clinic


.PHONY: all build_release docker_build

build_release:
	TARGET_CC='x86_64-unknown-linux-gnu-gcc' RUSTFLAGS='-C target-feature=+crt-static'  cargo build --release --target x86_64-unknown-linux-gnu

docker_build: build_release
	docker buildx build --platform linux/amd64 -f Dockerfile -t ${IMG} .

all: build_release docker_build
