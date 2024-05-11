//! Simple unit system to get a bit more guarantees via the type system.

use std::marker::PhantomData;

/// Trait to be implemented by units.
pub trait Unit {
    /// Name of the unit.
    ///
    /// Should be capitalized.
    fn name() -> &'static str;

    /// Symbol of the unit.
    fn symbol() -> &'static str;
}

/// A quantity has a value of some type and a unit.
pub struct Quantity<N, U> {
    value: N,
    unit: PhantomData<fn(&U)>,
}

impl<N, U> Quantity<N, U> {
    /// Construct the quantity from the given value.
    pub const fn from_value(raw: N) -> Self {
        Self {
            value: raw,
            unit: PhantomData,
        }
    }

    /// Convert the quantity to the raw value.
    pub const fn into_value(self) -> N
    where
        N: Copy,
    {
        self.value
    }
}

impl<N, U> From<N> for Quantity<N, U> {
    fn from(value: N) -> Self {
        Self::from_value(value)
    }
}

impl<N: std::fmt::Display, U: Unit> std::fmt::Display for Quantity<N, U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{} {}", self.value, U::symbol()))
    }
}

impl<N: std::fmt::Display, U: Unit> std::fmt::Debug for Quantity<N, U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}({})", U::name(), self.value))
    }
}

impl<N: Clone, U> Clone for Quantity<N, U> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            unit: PhantomData,
        }
    }
}

impl<N: Copy, U> Copy for Quantity<N, U> {}

impl<N: Copy + std::ops::Add<N, Output = N>, U> std::ops::Add for Quantity<N, U> {
    type Output = Quantity<N, U>;

    fn add(self, rhs: Self) -> Self::Output {
        Quantity::from_value(self.value + rhs.value)
    }
}

impl<N: Copy + std::ops::Sub<N, Output = N>, U> std::ops::Sub for Quantity<N, U> {
    type Output = Quantity<N, U>;

    fn sub(self, rhs: Self) -> Self::Output {
        Quantity::from_value(self.value - rhs.value)
    }
}

impl<N: Copy + std::ops::Div<N, Output = N>, U> std::ops::Div for Quantity<N, U> {
    type Output = N;

    fn div(self, rhs: Self) -> Self::Output {
        self.value / rhs.value
    }
}

impl<N: PartialEq, U> PartialEq for Quantity<N, U> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<N: Eq, U> Eq for Quantity<N, U> {}

impl<N: PartialOrd, U> PartialOrd for Quantity<N, U> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.value.partial_cmp(&other.value)
    }
}

impl<N: Ord, U> Ord for Quantity<N, U> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.value.cmp(&other.value)
    }
}

impl<N: std::hash::Hash, U> std::hash::Hash for Quantity<N, U> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

/// Number of bytes unit.
pub struct NumBytesUnit(());

impl Unit for NumBytesUnit {
    fn name() -> &'static str {
        "NumBytes"
    }

    fn symbol() -> &'static str {
        "B"
    }
}

/// Number of bytes.
pub type NumBytes = Quantity<u64, NumBytesUnit>;
