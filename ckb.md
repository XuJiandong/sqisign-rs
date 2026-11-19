# Verify in CKB-VM

The standalone `ckb-std` app verifies standard round-3 signatures at levels I,
III, and V. It builds the `no_std` verifier for little-endian RV64IMC with
Zba, Zbb, Zbc, and Zbs, and lowers atomics for the single-threaded VM.
Key generation and signing run on the host.

With Python 3 and `ckb-debugger` on your path, run the smoke tests from the repository root.

```sh
rustup target add riscv64imac-unknown-none-elf
make ckb-smoke
```

This builds the verifier and runs the first known-answer vector at each level.
It also checks rejection of a changed message and a truncated signature at level I.
To verify your own inputs, run `make ckb-build` and pass the message, detached
signature, and public key as hex strings without a `0x` prefix.

```sh
ckb-debugger --bin target/ckb/riscv64imac-unknown-none-elf/release/ckb-sqisign-verify \
  -- "$message_hex" "$signature_hex" "$public_key_hex"
```

The guest returns `0` for a valid signature, `1` when verification fails, and
`2` when it cannot decode the arguments.
