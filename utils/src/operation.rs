use num_bigint::BigInt;
use num_rational::BigRational;

/// Multiply `a` with `f` and round the result up to the nearest integer.
pub fn mul_ceil(a: u64, f: f64) -> BigInt {
    assert!(f.is_finite());

    let a = BigInt::from(a);
    let f = BigRational::from_float(f).expect("f is finite");
    (f * a).ceil().to_integer()
}

/// Multiply `a` with `f` and round the result down to the nearest integer.
pub fn mul_floor(a: u64, f: f64) -> BigInt {
    assert!(f.is_finite());

    let a = BigInt::from(a);
    let f = BigRational::from_float(f).expect("f is finite");
    (f * a).floor().to_integer()
}