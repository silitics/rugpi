sidex::include_bundle! {
    #[doc(hidden)]
    rugix_bundle as generated
}
// Re-export the generated data structures.
pub use generated::manifest::*;
