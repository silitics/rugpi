#![no_main]

use libfuzzer_sys::fuzz_target;

use byte_calc::{NumBits, NumBlocks, NumBytes};

fuzz_target!(|pair: (u64, u64)| {
    let (left, right) = pair;
    test_ops::<NumBits>(left, right);
    test_ops::<NumBlocks>(left, right);
    test_ops::<NumBytes>(left, right);
});

fn test_ops<T: Type>(left: u64, right: u64) {
    if let Some(value) = left.checked_add(right) {
        assert_eq!(T::new(left) + T::new(right), T::new(value));
    }
    if let Some(value) = left.checked_sub(right) {
        assert_eq!(T::new(left) - T::new(right), T::new(value));
    }
    if let Some(value) = left.checked_mul(right) {
        assert_eq!(T::new(left) * T::new(right), T::new(value));
    }
    if let Some(value) = left.checked_div(right) {
        assert_eq!(T::new(left) / T::new(right), T::new(value));
    }
}

trait Type:
    Sized
    + PartialEq
    + std::fmt::Debug
    + std::ops::Add<Output = Self>
    + std::ops::Sub<Output = Self>
    + std::ops::Mul<Output = Self>
    + std::ops::Div<Output = Self>
{
    fn new(raw: u64) -> Self;
}

impl Type for NumBytes {
    fn new(raw: u64) -> Self {
        NumBytes::new(raw)
    }
}

impl Type for NumBits {
    fn new(raw: u64) -> Self {
        NumBits::new(raw)
    }
}

impl Type for NumBlocks {
    fn new(raw: u64) -> Self {
        NumBlocks::new(raw)
    }
}
