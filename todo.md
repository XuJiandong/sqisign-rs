# CKB-VM verification priorities

The baseline is commit `10235c5`, built with `make ckb-build` and measured with
ckb-debugger 1.1.1 on the standard Level 1, count 0 KAT. Fast mode returned 0
in **315,381,398 cycles**. The full-mode flamegraph accounted for 315,228,456
cycles. Priorities use exclusive cycles aggregated by the leaf symbol so a
callee's work is counted once.

| Priority | Level 1 bottleneck | Exclusive cycles | Profile share | Status |
| --- | --- | ---: | ---: | --- |
| 1 | `p324_3::modmul` (six-limb Montgomery multiplication) | 112,407,594 | 35.66% | Done |
| 2 | `FpBackend::sub` (limb subtraction and modular correction) | 70,707,835 | 22.43% | Done |
| 3 | `FpBackend::add` (limb addition and modular correction) | 68,861,226 | 21.84% | Done |

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

## Measured improvements

| Priority | Change | Fast-mode cycles | Saved from previous build |
| --- | --- | ---: | ---: |
| 1 | Share diagonal products across Montgomery multiplication columns (21 wide products per call, down from 36) | 297,435,342 | 17,946,056 |
| 2 | Compute the subtraction borrow in registers and apply the sparse `2p` correction while writing the result | 218,260,463 | 79,174,879 |
| 3 | Reduce the biased sum `a + b - 2p` with the same signed-limb correction | 144,078,977 | 74,181,486 |

The three changes saved **171,302,421 cycles (54.32%)**. Each measured
build returned guest exit 0. The final full-mode profile attributed
94,461,534 cycles to Montgomery multiplication, down from 112,407,594.

## CKB-VM macro-op fusion

A fresh Level 1 flamegraph at `d47e520` attributed 94,461,534 of
143,931,397 execution cycles (65.63%) to Montgomery multiplication.
The CKB build now splits each of its 21 signed products at bit 57, using
`WIDE_MUL` and accumulating each radix-55 column in registers. The
generated schoolbook kernels pair `WIDE_MULU` with `ADD3A` to accumulate
unsigned products. The extension-field and Hadamard kernels defer modular
corrections until their outputs. [VM v1 fusion rules](https://github.com/nervosnetwork/rfcs/blob/master/rfcs/0033-ckb-vm-version-1/0033-ckb-vm-version-1.md#42-mop) · [VM v2 fusion rules](https://github.com/nervosnetwork/rfcs/blob/master/rfcs/0049-ckb-vm-version-2/0049-ckb-vm-version-2.md#specification)

Fast-mode `make ckb-smoke` measurements at `6a3bcc6` with rustc 1.100.0 and
ckb-debugger 1.1.1 used script version 2 and count 0 at each level.

| Level | Before | After | Reduction |
| --- | ---: | ---: | ---: |
| 1 | 144,078,977 | 80,456,673 | 44.16% |
| 3 | 962,459,760 | 341,312,953 | 64.54% |
| 5 | 1,730,559,787 | 653,981,688 | 62.21% |

Running the same optimized Level 1 ELF and arguments in ckb-vm 0.24.14
with only `ISA_MOP` toggled gave 97,245,137 execution cycles with fusion
disabled and 80,209,253 with it enabled. The VM executed 3,289,260
`WIDE_MUL`, 67,725 `WIDE_MULU` and 67,725 `ADD3A` operations.
The final flamegraph's largest leaves were extension-field multiplication
(43,461,093 cycles) and squaring (19,720,509 cycles).

Inlining the field kernels and fixed-size memory copies increased the
unstripped release ELF from 661,424 to 1,135,160 bytes at `6a3bcc6`.

Restoring LLVM's default `memcpy` threshold while retaining MOP and
`-max-store-memset=96` reduced the ELF to **699,536 bytes**. With this build,
the count-0 vectors take **85,471,175** cycles at Level 1, **366,104,127**
at Level 3, and **691,169,558** at Level 5. The truncated signature returns
guest exit 2 in 188,676 cycles.
