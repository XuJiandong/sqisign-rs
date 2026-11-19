//! Level I field kernels for RV64, using the portable radix-55 layout.

use super::p324_3::{MASK, NLIMBS, RADIX, TWO_PTOP};

type Limbs = [u64; NLIMBS];

#[cfg(all(target_arch = "riscv64", feature = "ckb-vm"))]
pub(crate) use super::p324_3_ckb::modmul;

#[cfg(not(all(target_arch = "riscv64", feature = "ckb-vm")))]
/// Montgomery product using six diagonal products and fifteen signed products
/// of differences. In column i+j the two off-diagonal terms are
/// d_i + d_j + (a_i - a_j)(b_j - b_i), where d_i = a_i b_i.
#[inline]
pub(crate) fn modmul(out: &mut Limbs, a: &Limbs, b: &Limbs) {
    let [d0, d1, d2, d3, d4, d5] = core::array::from_fn(|i| i128::from(a[i]) * i128::from(b[i]));
    let cross = |i: usize, j: usize| {
        i128::from(a[i] as i64 - a[j] as i64) * i128::from(b[j] as i64 - b[i] as i64)
    };
    // Each column has at most six 55-by-55-bit terms plus one Montgomery
    // fold and the preceding carry. Shifting it by 55 leaves at most 58 bits.
    let mut carry = 0u64;
    let mut emit = |column: i128| {
        let t = column + i128::from(carry);
        carry = (t >> RADIX) as u64;
        t as u64 & MASK
    };
    // PTOP = 3 * 2^49. Triple the 55-bit limb before widening.
    let fold = |v| i128::from(3u64 * v) << 49;
    let mut diagonal = d0;
    let v0 = emit(diagonal);
    diagonal += d1;
    let v1 = emit(diagonal + cross(0, 1));
    diagonal += d2;
    let v2 = emit(diagonal + cross(0, 2));
    diagonal += d3;
    let v3 = emit(diagonal + cross(0, 3) + cross(1, 2));
    diagonal += d4;
    let v4 = emit(diagonal + cross(0, 4) + cross(1, 3));
    diagonal += d5;
    let v5 = emit(diagonal + cross(0, 5) + cross(1, 4) + cross(2, 3) + fold(v0));
    diagonal -= d0;
    out[0] = emit(diagonal + cross(1, 5) + cross(2, 4) + fold(v1));
    diagonal -= d1;
    out[1] = emit(diagonal + cross(2, 5) + cross(3, 4) + fold(v2));
    diagonal -= d2;
    out[2] = emit(diagonal + cross(3, 5) + fold(v3));
    diagonal -= d3;
    out[3] = emit(diagonal + cross(4, 5) + fold(v4));
    diagonal -= d4;
    out[4] = emit(diagonal + fold(v5));
    out[5] = carry;
}

/// Normalize signed radix-55 coefficients whose value is in [-2p, 2p).
#[inline]
pub(super) fn reduce_signed(out: &mut Limbs, limbs: [i64; NLIMBS]) {
    // Determine the sign using only carries. The correction for a negative
    // result is sparse: 2p = TWO_PTOP * (2^55)^5 - 2.
    let mut carry = 0;
    for &limb in &limbs[..5] {
        carry = (limb + carry) >> RADIX;
    }
    let negative = (limbs[5] + carry) >> 63;
    carry = -(negative & 2);
    for i in 0..5 {
        let digit = limbs[i] + carry;
        out[i] = digit as u64 & MASK;
        carry = digit >> RADIX;
    }
    out[5] = (limbs[5] + carry + (negative & TWO_PTOP as i64)) as u64;
}

#[inline]
pub(crate) fn modadd(out: &mut Limbs, a: &Limbs, b: &Limbs) {
    // a + b - 2p lies in [-2p, 2p), the same interval as a subtraction.
    let mut sum = core::array::from_fn(|i| (a[i] + b[i]) as i64);
    sum[0] += 2;
    sum[5] -= TWO_PTOP as i64;
    reduce_signed(out, sum);
}

#[inline]
pub(crate) fn modsub(out: &mut Limbs, a: &Limbs, b: &Limbs) {
    reduce_signed(out, core::array::from_fn(|i| a[i] as i64 - b[i] as i64));
}

#[cfg(all(test, not(target_arch = "riscv64")))]
mod tests {
    extern crate std;

    use super::*;
    use crate::fp::p324_3::{self, TWO_PTOP};

    // Compare the raw lazy representatives, including long carry chains near
    // powers of the radix and the upper end of [0, 2p).
    fn cases() -> std::vec::Vec<Limbs> {
        let mut cases = std::vec![
            [0; NLIMBS],
            [MASK - 2, MASK, MASK, MASK, MASK, TWO_PTOP - 1]
        ];
        for i in 0..NLIMBS {
            let mut power = [0; NLIMBS];
            power[i] = 1;
            cases.push(power);
            power[i] = 0;
            power[..i].fill(MASK);
            cases.push(power);
        }
        let mut state = 0x7e31_62fc_491b_856d_u64;
        for _ in 0..64 {
            let mut limbs = core::array::from_fn(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                state & MASK
            });
            limbs[5] %= TWO_PTOP;
            cases.push(limbs);
        }
        cases
    }

    fn compare(kernel: fn(&mut Limbs, &Limbs, &Limbs), reference: fn(&mut Limbs, &Limbs, &Limbs)) {
        let cases = cases();
        for a in &cases {
            for b in &cases {
                let mut actual = [0; NLIMBS];
                let mut expected = [0; NLIMBS];
                kernel(&mut actual, a, b);
                reference(&mut expected, a, b);
                assert_eq!(actual, expected, "a={a:x?}, b={b:x?}");
            }
        }
    }

    #[test]
    fn addition_matches_portable() {
        compare(modadd, p324_3::modadd);
    }

    #[test]
    fn subtraction_matches_portable() {
        compare(modsub, p324_3::modsub);
    }

    #[test]
    fn multiplication_matches_schoolbook() {
        compare(modmul, p324_3::modmul);
    }

    #[test]
    fn ckb_product_matches_schoolbook() {
        compare(crate::fp::p324_3_ckb::modmul, p324_3::modmul);
    }

    #[test]
    fn ckb_extension_and_hadamard_match_portable() {
        use crate::fp::p324_3_ckb as ckb;
        type Op = fn(&mut Limbs, &Limbs, &Limbs);
        fn apply(op: Op, a: &Limbs, b: &Limbs) -> Limbs {
            let mut value = [0; NLIMBS];
            op(&mut value, a, b);
            value
        }
        fn check(mut actual: Limbs, mut expected: Limbs) {
            let two_p = [MASK - 1, MASK, MASK, MASK, MASK, TWO_PTOP - 1];
            assert!(actual[..5].iter().all(|&limb| limb <= MASK));
            assert!(actual.iter().rev().lt(two_p.iter().rev()));
            p324_3::modfsb(&mut actual);
            p324_3::modfsb(&mut expected);
            assert_eq!(actual, expected);
        }
        let add: fn(&Limbs, &Limbs) -> Limbs = |a, b| apply(p324_3::modadd, a, b);
        let sub: fn(&Limbs, &Limbs) -> Limbs = |a, b| apply(p324_3::modsub, a, b);
        let mul: fn(&Limbs, &Limbs) -> Limbs = |a, b| apply(p324_3::modmul, a, b);
        let cases = cases();
        for (i, a) in cases.iter().enumerate() {
            for (j, b) in cases.iter().enumerate() {
                let c = &cases[(2 * i + j) % cases.len()];
                let d = &cases[(i + 3 * j) % cases.len()];
                let (mut re, mut im) = ([0; NLIMBS], [0; NLIMBS]);
                ckb::fp2_mul(&mut re, &mut im, a, b, c, d);
                check(re, sub(&mul(a, c), &mul(b, d)));
                check(im, add(&mul(a, d), &mul(b, c)));
                ckb::fp2_sqr(&mut re, &mut im, a, b);
                check(re, sub(&mul(a, a), &mul(b, b)));
                check(im, add(&mul(a, b), &mul(a, b)));
                let (mut x, mut y, mut z, mut t) = (*a, *b, *c, *d);
                ckb::hadamard(&mut x, &mut y, &mut z, &mut t);
                check(x, add(&add(a, b), &add(c, d)));
                check(y, add(&sub(a, b), &sub(c, d)));
                check(z, sub(&add(a, b), &add(c, d)));
                check(t, sub(&sub(a, b), &sub(c, d)));
            }
        }
    }
}
