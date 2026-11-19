# CKB-VM verification priorities

The baseline is commit `10235c5`, built with `make ckb-build` and measured with
ckb-debugger 1.1.1 on the standard Level 1, count 0 KAT. Fast mode returned 0
in **315,381,398 cycles**. The full-mode flamegraph accounted for 315,228,456
cycles. Priorities use exclusive cycles aggregated by the leaf symbol so a
callee's work is counted once.

| Priority | Level 1 bottleneck | Exclusive cycles | Profile share | Status |
| --- | --- | ---: | ---: | --- |
| 1 | `p324_3::modmul` (six-limb Montgomery multiplication) | 112,407,594 | 35.66% | Open |
| 2 | `FpBackend::sub` (limb subtraction and modular correction) | 70,707,835 | 22.43% | Open |
| 3 | `FpBackend::add` (limb addition and modular correction) | 68,861,226 | 21.84% | Open |

The next two leaf costs were the iterator's `nth` during carry propagation
(23,062,772 cycles) and the compiler's `memcpy` (14,678,261 cycles).

## Reproducing the profile

From the repository root, run `make ckb-build`, then use the smoke test's
argument order and its first Level 1 vector.

```sh
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=ckb python3 - <<'PY'
import subprocess
from smoke import KATS, first_vector

binary = "target/ckb/riscv64imac-unknown-none-elf/release/ckb-sqisign-verify"
args = [v.hex() for v in first_vector(KATS / "PQCsignKAT_270_SQIsign_p324_3.rsp")]
for options in ([], ["--mode", "full", "--enable-flamegraph",
                     "--flamegraph-output", "/tmp/sqisign-l1.folded"]):
    subprocess.run(["ckb-debugger", "--bin", binary, *options, "--", *args], check=True)
PY
```

For the release ELF, resolve the hexadecimal frame addresses against
`nm -nC` on the same binary. Sum the weight of each folded stack under its
last frame for the exclusive ranking. All optimization results below use
the fast-mode benchmark.
