//! Register-only RV64 arithmetic scheduled for CKB-VM macro-op fusion.

/// Split the signed product at bit 57. The caller shifts the first operand
/// left by seven so WIDE_MUL returns the high coefficient directly.
#[inline(always)]
#[allow(unsafe_code)]
pub(super) fn split_product(shifted: i64, b: i64) -> (i64, i64) {
    let hi: i64;
    let lo: u64;
    // SAFETY: RV64M register arithmetic. The early high output is distinct
    // from both inputs and the low output, as required by WIDE_MUL.
    unsafe {
        core::arch::asm!(
            "mulh {hi}, {a}, {b}",
            "mul {lo}, {a}, {b}",
            hi = out(reg) hi, lo = lateout(reg) lo,
            a = in(reg) shifted, b = in(reg) b,
            options(pure, nomem, nostack),
        );
    }
    ((lo >> 7) as i64, hi)
}

/// Accumulate an unsigned wide product modulo 2^128 using WIDE_MULU and ADD3A.
#[inline(always)]
#[allow(unsafe_code)]
pub(super) fn mul_add(acc: u128, a: u64, b: u64) -> u128 {
    let lo: u64;
    let hi: u64;
    // SAFETY: register-only RV64IM instructions. Keep the add/sltu/add
    // operand order even when the assembler could commute a compressed add.
    unsafe {
        core::arch::asm!(
            "mulhu {hi}, {a}, {b}",
            "mul {lo}, {a}, {b}",
            ".option push", ".option norvc",
            "add {lo}, {al}, {lo}",
            "sltu {carry}, {lo}, {al}",
            "add {hi}, {carry}, {hi}",
            ".option pop",
            hi = out(reg) hi, lo = out(reg) lo, carry = out(reg) _,
            a = in(reg) a, b = in(reg) b, al = in(reg) acc as u64,
            options(pure, nomem, nostack),
        );
    }
    (u128::from(hi.wrapping_add((acc >> 64) as u64)) << 64) | u128::from(lo)
}
