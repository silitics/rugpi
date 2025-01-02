//! A vector with typed indices.

use std::fmt::Debug;
use std::marker::PhantomData;

/// An index into an [`IdxVec`].
pub trait Idx: Copy {
    /// Convert the index to [`usize`].
    fn as_usize(self) -> usize;

    /// Convert [`usize`] to an index.
    fn from_usize(idx: usize) -> Self;
}

/// A vector with a typed index.
#[derive(Clone)]
#[repr(transparent)]
pub struct IdxVec<I, T> {
    /// The actual data of the vector.
    vec: Vec<T>,
    _phantom_idx: PhantomIdx<I>,
}

impl<I, T> IdxVec<I, T> {
    /// Create an empty vector.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a typed index vector from the given vector.
    pub fn from_vec(vec: Vec<T>) -> Self {
        Self {
            vec,
            _phantom_idx: PhantomIdx::default(),
        }
    }

    /// The length of the vector.
    pub fn len(&self) -> usize {
        self.vec.len()
    }

    /// Check whether the vector is empty.
    pub fn is_empty(&self) -> bool {
        self.vec.is_empty()
    }
}

impl<I: Idx, T> IdxVec<I, T> {
    /// The next index.
    pub fn next_idx(&self) -> I {
        I::from_usize(self.len())
    }

    /// The index of the first element.
    ///
    /// Panics if the vector is empty.
    pub fn first_idx(&self) -> I {
        if self.is_empty() {
            panic!("there is no first element, the vector is empty");
        }
        I::from_usize(0)
    }

    /// The index of the last element.
    ///
    /// Panics if the vector is empty.
    pub fn last_idx(&self) -> I {
        if self.is_empty() {
            panic!("there is no last element, the vector is empty");
        }
        // Cannot underflow as the vector is non-empty.
        I::from_usize(self.len() - 1)
    }

    /// Push an element onto the vector and return its index.
    pub fn push(&mut self, element: T) -> I {
        self.vec.push(element);
        self.last_idx()
    }

    /// Construct an element, push it onto the vector, and return its index.
    pub fn push_with(&mut self, make: impl FnOnce(I) -> T) -> I {
        self.push(make(self.next_idx()))
    }

    /// Map the values of the vector to create a new typed vector.
    pub fn map<U>(self, mut map: impl FnMut(I, T) -> U) -> IdxVec<I, U> {
        IdxVec::from_vec(
            self.vec
                .into_iter()
                .enumerate()
                .map(|(idx, element)| map(I::from_usize(idx), element))
                .collect(),
        )
    }

    /// Iterator over the indices and elements of the vector.
    pub fn iter(&self) -> impl Iterator<Item = (I, &T)> {
        self.vec
            .iter()
            .enumerate()
            .map(|(idx, element)| (I::from_usize(idx), element))
    }
}

impl<I, T: Debug> Debug for IdxVec<I, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("IdxVec").field(&self.vec).finish()
    }
}

impl<I, T> Default for IdxVec<I, T> {
    fn default() -> Self {
        Self {
            vec: Vec::new(),
            _phantom_idx: PhantomIdx::default(),
        }
    }
}

impl<I: Idx, T> std::ops::Index<I> for IdxVec<I, T> {
    type Output = T;

    fn index(&self, index: I) -> &Self::Output {
        &self.vec[index.as_usize()]
    }
}

impl<I: Idx, T> std::ops::IndexMut<I> for IdxVec<I, T> {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        &mut self.vec[index.as_usize()]
    }
}

/// [`PhantomData`] for the index type.
type PhantomIdx<I> = PhantomData<fn(&I)>;

/// Creates a new index type and implements [`Idx`] for it.
macro_rules! new_idx_type {
    ($(#[$doc:meta])* $vis:vis $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        $vis struct $name(usize);

        impl $crate::utils::idx_vec::Idx for $name {
            fn as_usize(self) -> usize {
                self.0
            }

            fn from_usize(idx: usize) -> Self {
                Self(idx)
            }
        }
    };
}

pub(crate) use new_idx_type;
