# Hermetic, verify-only build of the SQIsign library crates.
#
# This is a verification gate, not an artifact build: the multi-stage
# Containerfile builds and tests the crates offline in a digest-pinned stagex
# toolchain (`--locked`, `--network=none`), so a successful `docker build`
# means the workspace builds and its tests pass hermetically. There is no
# shipped image, so no OCI exporter is needed.
.PHONY: verify
verify:
	DOCKER_BUILDKIT=1 \
	docker build \
		--build-arg VERSION=$(VERSION) \
		--tag anchorageoss-sqisign-rs/sqisign-rs \
		--progress=plain \
		--label "org.opencontainers.image.source=https://github.com/anchorageoss/sqisign-rs" \
		$(if $(filter 1,$(NOCACHE)),--no-cache) \
		-f images/hermetic-build/Containerfile \
		.

CKB_TARGET := riscv64imac-unknown-none-elf
# Inline fixed-size clears and use LLVM's default memcpy threshold to limit ELF size.
CKB_RUSTFLAGS := -C target-feature=-a,+zba,+zbb,+zbc,+zbs -C passes=lower-atomic -C llvm-args=-max-store-memset=96
CKB_TARGET_DIR := $(CURDIR)/target/ckb

.PHONY: ckb-build ckb-smoke
ckb-build:
	RUSTFLAGS="$(CKB_RUSTFLAGS)" cargo build --manifest-path ckb/Cargo.toml \
		--locked --release --target $(CKB_TARGET) --target-dir "$(CKB_TARGET_DIR)"

ckb-smoke: ckb-build
	python3 ckb/smoke.py "$(CKB_TARGET_DIR)/$(CKB_TARGET)/release/ckb-sqisign-verify"
