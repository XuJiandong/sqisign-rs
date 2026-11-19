//! Level I Montgomery multiplication in radix 2^55 for CKB-VM.
#[cfg(all(target_arch = "riscv64", feature = "ckb-vm"))]
use super::ckb_mop::split_product;
use super::p324_3::{MASK, NLIMBS, PTOP, RADIX, TWO_PTOP};
use super::p324_3_riscv64::{modsub, reduce_signed};

type Limbs = [u64; NLIMBS];

// Host differential tests exercise the same column schedule and limb bounds.
#[cfg(not(all(target_arch = "riscv64", feature = "ckb-vm")))]
fn split_product(shifted: i64, b: i64) -> (i64, i64) {
    let product = i128::from(shifted) * i128::from(b);
    ((product as u64 >> 7) as i64, (product >> 64) as i64)
}

/// Combine the low and high contributions of one product column.
#[inline(always)]
fn column(carry: &mut i64, diagonal: i64, lo: i64, hi: i64, digit: u64) -> u64 {
    let fold = (3 * digit) as i64;
    let t = diagonal + *carry + lo + ((fold & 63) << 49);
    *carry = (t >> RADIX) + 4 * hi + (fold >> 6);
    t as u64 & MASK
}

/// Montgomery product using six diagonal products and fifteen products of
/// differences. Column i+j gets a_i*b_j + a_j*b_i from d_i + d_j +
/// (a_i-a_j)*(b_j-b_i), where d_i = a_i*b_i.
///
/// In the resulting convolution d_i appears in columns i through i+5.
/// Write d_i = l_i + B*h_i. Its contribution is l_i in column i,
/// l_i+h_i in the five middle columns, then h_i in column i+6.
/// Thus the upper six diagonal sums are the total minus their lower
/// counterparts. Only one signed word per column needs to stay live.
#[inline(always)]
pub(crate) fn modmul(out: &mut Limbs, a: &Limbs, b: &Limbs) {
    // a consists of normalized digits or sums of two such digits (< 2B).
    // b may also hold signed digits of the biased Fp2 difference. Its
    // magnitude and all pairwise differences are at most 2B. A scaled a digit
    // and its differences fit i64, each high contribution is bounded by
    // 4B, and each low part is below 4B. Even six diagonal contributions
    // and three cross products fit comfortably in a signed column.
    let a = a.map(|x| (x << 7) as i64);
    let b = b.map(|x| x as i64);
    let cross = |i: usize, j: usize| split_product(a[i] - a[j], b[j] - b[i]);
    let mut total = 0;
    let mut diagonal = |i: usize| {
        let (lo, hi) = split_product(a[i], b[i]);
        let d = total + lo;
        total = d + 4 * hi;
        d
    };
    let mut c = 0;
    let d0 = diagonal(0);
    let v0 = column(&mut c, d0, 0, 0, 0);
    let d1 = diagonal(1);
    let (l01, h01) = cross(0, 1);
    let v1 = column(&mut c, d1, l01, h01, 0);
    let d2 = diagonal(2);
    let (l02, h02) = cross(0, 2);
    let v2 = column(&mut c, d2, l02, h02, 0);
    let d3 = diagonal(3);
    let (l03, h03) = cross(0, 3);
    let (l12, h12) = cross(1, 2);
    let v3 = column(&mut c, d3, l03 + l12, h03 + h12, 0);
    let d4 = diagonal(4);
    let (l04, h04) = cross(0, 4);
    let (l13, h13) = cross(1, 3);
    let v4 = column(&mut c, d4, l04 + l13, h04 + h13, 0);
    let d5 = diagonal(5);
    let (l05, h05) = cross(0, 5);
    let (l14, h14) = cross(1, 4);
    let (l23, h23) = cross(2, 3);
    let v5 = column(&mut c, d5, l05 + l14 + l23, h05 + h14 + h23, v0);
    let (l15, h15) = cross(1, 5);
    let (l24, h24) = cross(2, 4);
    out[0] = column(&mut c, total - d0, l15 + l24, h15 + h24, v1);
    let (l25, h25) = cross(2, 5);
    let (l34, h34) = cross(3, 4);
    out[1] = column(&mut c, total - d1, l25 + l34, h25 + h34, v2);
    let (l35, h35) = cross(3, 5);
    out[2] = column(&mut c, total - d2, l35, h35, v3);
    let (l45, h45) = cross(4, 5);
    out[3] = column(&mut c, total - d3, l45, h45, v4);
    out[4] = column(&mut c, total - d4, 0, 0, v5);
    out[5] = (c + total - d5) as u64;
}

/// Subtract q*p = q*PTOP*B^5 - q, choosing q from the top digit H
/// so that H - q*PTOP lies in [PTOP/2, 3*PTOP/2). Propagating the
/// lower digits of a Hadamard sum moves the top by at most four,
/// leaving the result inside [0, 2p).
#[inline(always)]
fn normalize_sum(out: &mut Limbs, digits: [i64; NLIMBS]) {
    // PTOP = 3 * 2^49. For an input in (-4p, 8p), the bias makes the
    // dividend positive and `units` fits in a byte.
    let biased = (digits[5] + (4 * PTOP + PTOP / 2) as i64) as u64;
    let units = (biased >> 49) as u8;
    let q = i64::from(units / 3) - 5;
    let mut carry = q;
    for i in 0..5 {
        let digit = digits[i] + carry;
        out[i] = digit as u64 & MASK;
        carry = digit >> RADIX;
    }
    out[5] = (digits[5] - q * PTOP as i64 + carry) as u64;
}

/// Karatsuba over Fp. REDC(T) = (T + m*p)/R with 0 <= m < R.
/// Both unreduced sums are below 4p, so their product reduces below
/// p + 16p²/R < 2p (p/R < 3/64).
#[inline]
pub(super) fn fp2_mul(
    re: &mut Limbs,
    im: &mut Limbs,
    ar: &Limbs,
    ai: &Limbs,
    br: &Limbs,
    bi: &Limbs,
) {
    let sa = core::array::from_fn(|i| ar[i] + ai[i]);
    let sb = core::array::from_fn(|i| br[i] + bi[i]);
    let mut rr = [0; NLIMBS];
    let mut ii = [0; NLIMBS];
    let mut sum = [0; NLIMBS];
    modmul(&mut sum, &sa, &sb);
    modmul(&mut rr, ar, br);
    modmul(&mut ii, ai, bi);
    modsub(re, &rr, &ii);
    // sum - rr - ii is above -2p: the cross terms are nonnegative and
    // each subtracted Montgomery correction is less than p.
    reduce_signed(
        im,
        core::array::from_fn(|i| sum[i] as i64 - rr[i] as i64 - ii[i] as i64),
    );
}

#[inline]
pub(super) fn fp2_sqr(out_re: &mut Limbs, out_im: &mut Limbs, re: &Limbs, im: &Limbs) {
    let sum = core::array::from_fn(|i| re[i] + im[i]);
    // Add 2p to the difference, allowing signed digits in this operand.
    // Both factors of the real product are nonnegative and below 4p.
    let mut diff: Limbs = core::array::from_fn(|i| re[i].wrapping_sub(im[i]));
    diff[0] = diff[0].wrapping_sub(2);
    diff[5] = diff[5].wrapping_add(TWO_PTOP);
    let twice = core::array::from_fn(|i| 2 * re[i]);
    modmul(out_im, &twice, im);
    modmul(out_re, &sum, &diff);
}

/// Evaluate both butterfly stages on signed digits and normalize each output once.
#[inline]
pub(super) fn hadamard(x: &mut Limbs, y: &mut Limbs, z: &mut Limbs, t: &mut Limbs) {
    let sum_xy: [i64; NLIMBS] = core::array::from_fn(|i| x[i] as i64 + y[i] as i64);
    let dif_xy: [i64; NLIMBS] = core::array::from_fn(|i| x[i] as i64 - y[i] as i64);
    let sum_zt: [i64; NLIMBS] = core::array::from_fn(|i| z[i] as i64 + t[i] as i64);
    let dif_zt: [i64; NLIMBS] = core::array::from_fn(|i| z[i] as i64 - t[i] as i64);
    normalize_sum(x, core::array::from_fn(|i| sum_xy[i] + sum_zt[i]));
    normalize_sum(y, core::array::from_fn(|i| dif_xy[i] + dif_zt[i]));
    normalize_sum(z, core::array::from_fn(|i| sum_xy[i] - sum_zt[i]));
    normalize_sum(t, core::array::from_fn(|i| dif_xy[i] - dif_zt[i]));
}
