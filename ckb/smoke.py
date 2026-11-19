"""Run the first standard round-3 KAT for each level in CKB-VM."""

from pathlib import Path
import subprocess
import sys


KATS = Path(__file__).resolve().parents[1] / "crates/kat/kat"


def first_vector(path):
    with path.open() as stream:
        fields = {}
        for line in stream:
            name, _, value = line.strip().partition(" = ")
            if name in ("msg", "pk", "sm"):
                fields[name] = bytes.fromhex(value)
            if name == "sm":
                break
    msg, pk, sm = (fields[name] for name in ("msg", "pk", "sm"))
    sig_len = len(sm) - len(msg)
    if sig_len <= 0 or sm[sig_len:] != msg:
        raise ValueError(f"invalid signed-message fixture in {path}")
    return msg, sm[:sig_len], pk


def check(binary, label, args, expected):
    result = subprocess.run(
        ["ckb-debugger", "--bin", binary, "--", *args],
        stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True,
    )
    print(f"{label}\n{result.stdout}", flush=True)
    # Fast mode reports both guest rejections and VM failures as a nonzero
    # process status. Check the guest exit code to distinguish them.
    if (result.returncode == 0) != (expected == 0) or (
        f"Run result: {expected}" not in result.stdout.splitlines()
    ):
        raise SystemExit(f"{label} expected guest exit {expected}")


def main(binary):
    for level, filename in [
        (1, "PQCsignKAT_270_SQIsign_p324_3.rsp"),
        (3, "PQCsignKAT_417_SQIsign_p500_27.rsp"),
        (5, "PQCsignKAT_549_SQIsign_p664_17.rsp"),
    ]:
        msg, sig, pk = first_vector(KATS / filename)
        check(binary, f"Level {level}, count 0", [msg.hex(), sig.hex(), pk.hex()], 0)
        if level == 1:
            changed = bytes([msg[0] ^ 1]) + msg[1:]
            check(binary, "Changed message", [changed.hex(), sig.hex(), pk.hex()], 1)
            check(binary, "Truncated signature", [msg.hex(), sig[:-1].hex(), pk.hex()], 2)


if __name__ == "__main__":
    main(sys.argv[1])
