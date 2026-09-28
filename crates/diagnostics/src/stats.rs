// Integer statistics, so a diagnosis never depends on float rounding.

// Whether the Wilson score 95% lower bound of k/n is at least a/b. With p = a/b the bound is at
// least p exactly when k/n > p and (k - n p)^2 >= z^2 n p (1 - p); z^2 = 3.8416 (z = 1.96).
pub fn wilson_at_least(k: u32, n: u32, a: u32, b: u32) -> bool {
    if a == 0 {
        return true;
    }
    if n == 0 || b == 0 || a >= b || k > n {
        return false;
    }
    let (k, n, a, b) = (k as u128, n as u128, a as u128, b as u128);
    if k * b <= n * a {
        return false;
    }
    let d = k * b - n * a;
    // The right side stays below 2^112, so a left side too large for u128 means the bound holds.
    10_000u128
        .checked_mul(d)
        .and_then(|x| x.checked_mul(d))
        .is_none_or(|lhs| lhs >= 38_416 * n * a * (b - a))
}

// The lower bound in tenths of a percent, rounded down: "a rate of at least 8.5%".
pub fn wilson_floor_permille(k: u32, n: u32) -> u16 {
    let (mut lo, mut hi) = (0u32, 999u32);
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        if wilson_at_least(k, n, mid, 1000) {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    lo as u16
}

// Rule of three: with no event in n trials, a rate above about 3/n would very likely have shown.
pub fn rule_of_three_permille(n: u32) -> u16 {
    if n == 0 {
        return 1000;
    }
    3000u32.div_ceil(n).min(1000) as u16
}
