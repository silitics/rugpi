#![no_std]
#![cfg_attr(feature = "nightly", feature(try_trait_v2))]

#[cfg(feature = "nightly")]
pub use core::ops::{FromResidual, Try};
#[cfg(not(feature = "nightly"))]
pub use stable::*;

#[cfg_attr(
    feature = "nightly",
    expect(dead_code, reason = "unstable version from `std` is used")
)]
mod stable {
    use core::convert::Infallible;
    use core::ops::ControlFlow;

    /// Construct a return value from a residual of a try operation.
    pub trait FromResidual<R = <Self as Try>::Residual> {
        /// Construct a type from the provided residual.
        fn from_residual(residual: R) -> Self;
    }

    /// Stable version of [`core::ops::Try`] for use with the [`xtry!`][crate::xtry!]
    /// macro.
    pub trait Try: FromResidual {
        /// Output of the try operation.
        type Output;
        /// Residual of the try operation.
        type Residual;

        /// Construct the type from its `Output` type.
        fn from_output(output: Self::Output) -> Self;

        /// Decide which branch to take.
        fn branch(self) -> ControlFlow<Self::Residual, Self::Output>;
    }

    impl<T, E> Try for Result<T, E> {
        type Output = T;
        type Residual = Result<Infallible, E>;

        fn from_output(output: Self::Output) -> Self {
            Ok(output)
        }

        fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
            match self {
                Ok(value) => ControlFlow::Continue(value),
                Err(error) => ControlFlow::Break(Err(error)),
            }
        }
    }

    impl<T, E, F> FromResidual<Result<Infallible, E>> for Result<T, F>
    where
        E: Into<F>,
    {
        fn from_residual(residual: Result<Infallible, E>) -> Self {
            match residual {
                Err(error) => Err(error.into()),
            }
        }
    }

    impl<T> Try for Option<T> {
        type Output = T;
        type Residual = Option<Infallible>;

        fn from_output(output: Self::Output) -> Self {
            Some(output)
        }

        fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
            match self {
                Some(value) => ControlFlow::Continue(value),
                None => ControlFlow::Break(None),
            }
        }
    }

    impl<T> FromResidual<Option<Infallible>> for Option<T> {
        fn from_residual(residual: Option<Infallible>) -> Self {
            match residual {
                None => None,
            }
        }
    }
}

/// Stable replacement for the `?` operator using the stable[`Try`] trait.
#[cfg(feature = "nightly")]
#[macro_export]
macro_rules! xtry {
    ($expr:expr) => {
        $expr?
    };
}

/// Stable replacement for the `?` operator using the stable [`Try`] trait.
#[cfg(not(feature = "nightly"))]
#[macro_export]
macro_rules! xtry {
    ($expr:expr) => {
        match $crate::Try::branch($expr) {
            ::core::ops::ControlFlow::Break(residual) => {
                return $crate::FromResidual::from_residual(residual)
            }
            ::core::ops::ControlFlow::Continue(value) => value,
        }
    };
}
