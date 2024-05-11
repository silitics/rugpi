//! Macros for dealing with limitations of constant functions.
#[allow(unused_macros)]

/// Constant unwrapping of options.
macro_rules! const_unwrap_option {
    ($value:expr) => {
        match $value {
            Some(value) => value,
            None => panic!("value is None"),
        }
    };
}

#[allow(unused_imports)]
pub(crate) use const_unwrap_option;

/// Constant unwrapping of results
macro_rules! const_unwrap_result {
    ($value:expr) => {
        match $value {
            Ok(value) => value,
            Err(_) => panic!("value is an error"),
        }
    };
}

pub(crate) use const_unwrap_result;

/// Constant version of the `?` operator.
macro_rules! const_try {
    ($expr:expr) => {
        match $expr {
            Err(error) => return Err(error),
            Ok(value) => value,
        }
    };
}

pub(crate) use const_try;

/// Constant loop over a given set of values.
macro_rules! const_for {
    ($var:ident in [$($value:expr),*] $expr:expr) => {
        $(
            let $var = $value;
            $expr
        )*
    };
    ($idx:ident, $var:ident in [$($value:expr),*] $expr:expr) => {
        let mut idx = 0;
        $(
            let $var = $value;
            let $idx = idx;
            $expr;
            #[allow(unused_assignments)]
            {
                idx += 1;
            }
        )*
    };
}

pub(crate) use const_for;
