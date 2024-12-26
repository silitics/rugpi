// spellchecker:ignore Tolnay Luca Palmieri Errorstack

//! A library for error handling and reporting.
//!
//! **Disclaimer: This library is in an exploratory stage of development and should be
//! considered experimental.**
//!
//! Reportify helps you create rich, user-friendly error reports that clarify issues and
//! provide valuable feedback to users.
//!
//! Here's an example of an error report generated with Reportify for a missing
//! configuration file:
#![doc = r#"
<pre>
<span style="color:#4E9A06;"><b>❯</b></span> RUST_BACKTRACE=1 cargo run --example simple-report
    <span style="color:#4E9A06;"><b>Finished</b></span> `dev` profile [unoptimized + debuginfo] target(s) in 0.48s
     <span style="color:#4E9A06;"><b>Running</b></span> `target/debug/examples/simple-report`

<span style="color:#DD3311;"><b>unable to load configuration</b></span>
├╴<span style="color:#888888;">at crates/reportify/examples/simple-report.rs:25:10</span>
├╴path: "path/does/not/exist.toml"
│   
╰─▶ <span style="color:#DD3311;"><b>configuration file not found</b></span>
    ├╴<span style="color:#888888;">at crates/reportify/examples/simple-report.rs:14:48</span>
    ├╴BACKTRACE (1)
    │   
    ╰─▶ <span style="color:#DD3311;"><b>No such file or directory (os error 2)</b></span>


━━━━ BACKTRACE (1)

   ⋮  skipped 10 frames

  11: <span style="color: #0099DD;">simple_report::read_config</span> <span style="color:#888888;">(0x100004467)</span>
      at reportify/examples/simple-report.rs:14:18
  12: <span style="color: #0099DD;">simple_report::run</span> <span style="color:#888888;">(0x1000045ef)</span>
      at reportify/examples/simple-report.rs:27:5
  13: <span style="color: #0099DD;">simple_report::main</span> <span style="color:#888888;">(0x10000467b)</span>
      at reportify/examples/simple-report.rs:34:26

   ⋮  skipped 6 frames
</pre>
"#]
//!
//! ## Preamble
//!
//! Let's first clarify what we mean by error handling and error reporting, respectively.
//! While these terms may be used differently by different people, we are going to use the
//! following characterizations for the purpose of this documentation:
//!
//! - _Error handling_ concerns the ability of a program to internally deal with errors.
//! - _Error reporting_ concerns the ability of a program to present errors to humans.
//!
//! For instance, when trying to read a configuration file that is missing, a program may
//! either _handle_ the error internally by falling back to a default configuration or
//! _report_ the error to the user informing them about the missing file. Note that the
//! recipient of an error report may also be a developer or a member of a support team
//! helping a user troubleshoot a problem.
//!
//! While being related, error handling and reporting are different concerns and should be
//! treated as such. Error handling requires _well-structured_, explicitly-typed errors
//! that can be programmatically inspected. In contrast, error reporting often benefits
//! from _freeform information_, such as suggestions, further explanations, or backtraces,
//! that is not required for error handling but essential for good error reports, i.e.,
//! error reports that are easy to understand and helpful for troubleshooting problems.
//!
//! Effective error handling and reporting is crucial for applications to be robust and
//! user-friendly.
//!
//!
//! ## Rationale and Use Case
//!
//! Much has been said about error handling and reporting in Rust and, as a result, the
//! Rust ecosystem already provides a myriad of error-related libraries for diverse use
//! cases. So, why on earth do we need yet another such library?
//!
//! Let's first look at two of the most popular libraries for error handling and
//! reporting, [Thiserror](https://docs.rs/thiserror) and [Anyhow](https://docs.rs/anyhow).
//! While being written by the same author, David Tolnay, they are very different.
//! Thiserror provides a derive macro for conveniently defining _specific error types_,
//! e.g., enumerations of all possible errors that may occur in a given context. In
//! contrast, Anyhow provides a single _type-erased error type_ making it easy to
//! propagate arbitrary errors up to the user for reporting while adding helpful, freeform
//! information along the way. In general, specific error types have the following
//! advantages over a single type-erased error type:
//!
//! - They are required to handle errors selectively without down-casting.
//! - They force a considerate and explicit propagation of errors.
//! - They are open to extension while maintaining backwards compatibility.
//!
//! With a single type-erased error type, we seemingly trade those advantages for the
//! following conveniences:
//!
//! - Straightforward propagation of arbitrary errors regardless of their type.
//! - Ad-hoc creation of one-of-a-kind errors.
//! - Ability to add freeform context information to errors.
//!
//! Due to these characteristics, Thiserror is typically used by libraries to define
//! specific error types enabling selective handling of errors by callers whereas Anyhow
//! is typically used by applications where a vast array of errors may occur and most of
//! them will simply be propagated and eventually be reported to a user. In
//! [Error Handling In Rust - A Deep Dive](https://www.lpalmieri.com/posts/error-handling-rust/),
//! Luca Palmieri argues that the choice between Thiserror and Anyhow depends on whether
//! you expect that a caller might need to handle different kinds of errors differently.
//! With the earlier introduced distinction between error handling and reporting,
//! Thiserror is for errors that might need to be selectively handled while Anyhow is for
//! errors that will eventually be reported. When using Anyhow you deprive callers of the
//! ability to handle errors selectively and when using Thiserror you do not get the
//! conveniences of Anyhow.
//!
//! Now, as usual in the Rust world, our goal is to have our cake and eat it too.[^1] So,
//! can we have some of the conveniences of a single type-erased error type while also
//! retaining the advantages of specific error types? This library explores a design
//! around specific error types that strives to carry over the conveniences of a single
//! type-erased error type in a way that gives developers the choice to opt-in into
//! specific conveniences, such as straightforward propagation of errors of arbitrary
//! types.
//!
//! The intended use case of this library are applications written in _library-style_,
//! where application logic is implemented as a reusable library and the executable is a
//! thin wrapper around it. Take [Cargo](https://doc.rust-lang.org/cargo) as an example
//! of such an application. Cargo is written around a [Cargo library](https://docs.rs/cargo)
//! which can be used independently of the Cargo application. Cargo, like many other Rust
//! applications, uses Anyhow, including in their library. In case of applications written
//! in library-style, most of the errors will be reported, one-of-a-kind errors are
//! common, and we would like the ability to add freeform information to errors and often
//! report errors of arbitrary types. That's why using Anyhow makes sense for this use
//! case. However, Anyhow is still not ideal for the following reasons:
//!
//! - A single type-erased error type makes it far too easy to implicitly---using the `?`
//!   operator---propagate any errors without any consideration of whether context should
//!   be added at a given point or whether the error could be handled. Without a huge
//!   amount of programming discipline this can quickly lead to hard to understand error
//!   reports.[^2]
//! - One does not always know or want to decide at a given point whether an error might
//!   be selectively handled down the callstack or be eventually reported. In such cases,
//!   it makes sense to return an error that has a specific type and can be handled while
//!   also having the ability to add freeform information should the error be eventually
//!   reported.
//! - As an application evolves one may need to add the ability to selectively handle
//!   specific errors to certain parts of the application. If Anyhow is used pervasively
//!   throughout the entire codebase this may require a huge refactoring effort because
//!   there are no specific error types which could be extended in a backwards-compatible
//!   way.
//!
//! The design of this library aims to address these shortcomings by promoting the usage
//! of specific error types in applications. It does so by providing functionality that
//! makes it more convenient to deal with them.
//!
//! [Errorstack](https://docs.rs/error-stack/) is another, more mature library in the Rust
//! ecosystem that has a philosophy similar to this library. If you find the above
//! considerations convincing, you should check it out. In fact, Errorstack was a huge
//! inspiration for this library. While similar in philosophy, this library explores a
//! different API with the explicit aim to reduce a bit of the friction introduced by
//! Errorstack's design while still keeping enough friction to force developers to be
//! considerate when it comes to error propagation.[^3]
//!
//!
//! ## Errors and Reports
//!
//! This library is build around the type [`Report<E>`] where `E` is a specific error type
//! for error handling and [`Report`] augments `E` with additional freeform information
//! for error reporting, thereby cleanly separating both concerns.
//!
//! The functionality provided by this library serves two main purposes, _report creation_
//! and _report propagation_. Report creation is about creating a `Report<E>` based on
//! some other value, e.g., a plain error of type `E`. Report propagation is about taking
//! a `Report<E>` and turning it into a `Report<F>` when crossing an _error boundary_
//! beyond which error handling requires a different type, e.g., when multiple different
//! types of errors are possible and they need to be combined, or when one wants to
//! abstract over the details of some error type. Report creation and propagation are
//! almost always explicit.
//!
//! The extension traits [`ErrorExt`] and [`ResultExt`], enable the convenient creation
//! and propagation of reports:
//!
//! - [`ErrorExt::report`]: Creates a `Report<E>` from an error of type `F` where `F:
//!   ReportAs<E>`. [`ReportAs`] has a blanket implementation for all `F: Into<E>` hooking
//!   into Rust's own conversion system.
//! - [`ErrorExt::whatever`]: Creates a `Report<E>` from an arbitrary error and a
//!   description if `E: Whatever`.
//! - [`ErrorExt::whatever_with`]: Same as [`ErrorExt::whatever`] but with a lazily
//!   computed description.
//!
//! - [`ResultExt::report`]: Same as [`ErrorExt::report`] but for results.
//! - [`ResultExt::whatever`]: Same as [`ErrorExt::whatever`] but for results.
//! - [`ResultExt::whatever_with`]: Same as [`ErrorExt::whatever_with`] but for results.
//! - [`ResultExt::propagate`]: Convert a result with `Report<E>` to a result with
//!   `Report<F>` where `E: PropagateAs<F>`.
//! - [`ResultExt::with_info`]: Attach freeform information to a result with a report.
//!
//! ### [`Whatever`] Trait
//!
//! Errors that can be created from arbitrary other errors or be constructed from strings
//! as one-of-a-kind errors must implement the trait [`Whatever`]. Implementing this trait
//! makes it convenient to construct reports of these errors in various ways.
//!
//! ```
//! # use std::path::Path;
//! use reportify::{bail, Report, ResultExt};
//!
//! // Define a simple `Whatever` error type.
//! reportify::new_whatever_type! {
//!     /// Application error.
//!     AppError
//! }
//!
//! // Report creation from arbitrary errors using `.whatever`.
//! fn read_file(path: &Path) -> Result<String, Report<AppError>> {
//!     std::fs::read_to_string(path)
//!         .whatever("unable to read file to string")
//!         .with_info(|_| format!("path: {path:?}"))
//! }
//!
//! // Report creation for one-of-a-kind errors using `bail!`.
//! fn do_something() -> Result<(), Report<AppError>> {
//!     bail!("unable to do something")
//! }
//! ```
//!
//! By implementing [`Whatever`] one opts-in into the usual conveniences of Anyhow. Note
//! that these errors cannot easily be handled selectively, however, should that need
//! arise in the future, they can be extended in a backwards-compatible way:
//!
//! ```
//! # use std::path::Path;
//! use reportify::{bail, ErrorExt, Report, ResultExt};
//! use thiserror::Error;
//!
//! #[derive(Debug, Error)]
//! enum AppError {
//!     #[error("invalid configuration")]
//!     InvalidConfig,
//!     #[error("other error")]
//!     Other,
//! }
//!
//! impl reportify::Whatever for AppError {
//!     fn new() -> Self {
//!         Self::Other
//!     }
//! }
//!
//! // Report creation from arbitrary errors using `.whatever`.
//! fn read_file(path: &Path) -> Result<String, Report<AppError>> {
//!     std::fs::read_to_string(path)
//!         .whatever("unable to read file to string")
//!         .with_info(|_| format!("path: {path:?}"))
//! }
//!
//! // Report creation for one-of-a-kind errors using `bail!`.
//! fn do_something() -> Result<(), Report<AppError>> {
//!     bail!("unable to do something")
//! }
//!
//! // Function exploiting the specific error type.
//! fn load_config() -> Result<(), Report<AppError>> {
//!     Err(AppError::InvalidConfig.report())
//! }
//! ```
//!
//! Calling `whatever` on an error or result will create a report using the [`Whatever`]
//! implementation of the error type. The respective methods always require a description
//! of the error thereby forcing developers to describe the error.
//!
//!
//! [^1]: <https://blog.rust-lang.org/2017/05/05/libz-blitz.html>
//! [^2]: The creators of [Errorstack](https://docs.rs/error-stack/) found that adding
//!       friction to error propagation forces developers to be more considerate and
//!       provide context, resulting in improved error reports (see
//!       <https://hash.dev/blog/announcing-error-stack#what's-the-catch>).
//! [^3]: Recall that this library is still in an exploratory stage, so this might not
//!       actually work out.

use std::{
    any::Any,
    error::Error as StdError,
    fmt::{Debug, Display},
    future::Future,
};

use backtrace::BacktraceImpl;

mod backtrace;
mod renderer;

/// Abstraction for types that can be used as the error type of a report.
pub trait Error: 'static + Send + Any + Debug {
    /// Optional, human-readable description of the error.
    ///
    /// For types implementing [`StdError`], this is the [`Display`] implementation of the
    /// error itself.
    ///
    /// Errors that do not implement [`StdError`] can choose to not provide a description.
    fn description(&self) -> Option<&dyn Display>;

    /// Casts the error into an [`StdError`], if the error is an [`StdError`].
    ///
    /// **You should not override this method.**
    fn as_std_error(&self) -> Option<&(dyn 'static + StdError + Send)> {
        None
    }

    /// The name of the error type.
    ///
    /// Defaults to [`std::any::type_name`].
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    /// Turns the error into [`Box<dyn Error>`].
    fn boxed_dyn(self) -> Box<dyn Error>
    where
        Self: Sized,
    {
        Box::new(self)
    }
}

// Implementation of `Error` for standard error types.
impl<E: 'static + StdError + Send> Error for E {
    fn description(&self) -> Option<&dyn Display> {
        Some(self)
    }

    fn as_std_error(&self) -> Option<&(dyn 'static + StdError + Send)> {
        Some(self)
    }
}

trait AnyReport: Send {
    fn error(&self) -> &dyn Error;

    fn meta(&self) -> &ReportMeta;
}

impl<E: Error> AnyReport for ReportInner<E> {
    fn error(&self) -> &dyn Error {
        &self.error
    }

    fn meta(&self) -> &ReportMeta {
        &self.meta
    }
}

/// Type-erased report.
struct ErasedReport {
    inner: Box<dyn AnyReport>,
}

impl AnyReport for ErasedReport {
    fn error(&self) -> &dyn Error {
        self.inner.error()
    }

    fn meta(&self) -> &ReportMeta {
        self.inner.meta()
    }
}

/// Error report with an error of type `E` for handling.
pub struct Report<E> {
    // We box everything to keep the size of the report small. The representation of
    // reports is certainly not optimal yet as we focus on the API first.
    inner: Box<ReportInner<E>>,
}

impl<E: Error> AnyReport for Report<E> {
    fn error(&self) -> &dyn Error {
        &self.inner.error
    }

    fn meta(&self) -> &ReportMeta {
        &self.inner.meta
    }
}

struct ReportInner<E> {
    error: E,
    meta: ReportMeta,
}

#[derive(Default)]
struct ReportMeta {
    /// Optional description of the error.
    description: Option<String>,
    /// Location where the report has been created.
    location: Option<&'static std::panic::Location<'static>>,
    backtrace: Option<backtrace::Backtrace>,
    #[cfg(feature = "spantrace")]
    spantrace: Option<tracing_error::SpanTrace>,
    causes: Vec<ErasedReport>,
    info: Vec<Box<dyn Printable>>,
}

/// Builder for reports.
#[must_use]
pub struct ReportBuilder<E = ()> {
    inner: ReportInner<E>,
}

impl ReportMeta {
    /// Capture a backtrace (and/or spantrace).
    fn capture_backtrace(&mut self) {
        self.backtrace = Some(backtrace::Backtrace::capture());
        #[cfg(feature = "spantrace")]
        {
            self.spantrace = Some(tracing_error::SpanTrace::capture());
        }
    }

    /// Capture the location of the caller.
    #[track_caller]
    fn capture_location(&mut self) {
        self.location = Some(std::panic::Location::caller());
    }

    /// Add freeform information to the metadata.
    fn add_info<I: Printable>(&mut self, info: I) {
        self.info.push(Box::new(info))
    }
}

impl ReportBuilder {
    /// Create a new report builder without an error and with default metadata.
    pub fn new() -> Self {
        Self {
            inner: ReportInner {
                error: (),
                meta: ReportMeta::default(),
            },
        }
    }
}

impl<E> ReportBuilder<E> {
    /// Add a backtrace (and/or spantrace) to the report.
    pub fn with_backtrace(mut self) -> Self {
        self.inner.meta.capture_backtrace();
        self
    }

    /// Add location information to the report.
    #[track_caller]
    pub fn with_location(mut self) -> Self {
        self.inner.meta.capture_location();
        self
    }

    /// Set the error of the report.
    pub fn with_error<F: Error>(self, error: F) -> ReportBuilder<F> {
        ReportBuilder {
            inner: ReportInner {
                error,
                meta: self.inner.meta,
            },
        }
    }

    /// Add some printable context information to the report.
    pub fn with_info<I: Printable>(mut self, info: I) -> Self {
        self.inner.meta.add_info(info);
        self
    }

    /// Add another report as a cause to the report.
    pub fn with_cause<C: private::Report>(mut self, cause: C) -> Self {
        self.inner.meta.causes.push(cause.into_report().erased());
        self
    }

    /// Overwrite the description of the error.
    pub fn with_description<D: Printable>(mut self, description: D) -> Self {
        self.inner.meta.description = Some(description.to_string());
        self
    }
}

impl<E: Error> ReportBuilder<E> {
    /// Build the report.
    pub fn build(self) -> Report<E> {
        Report {
            inner: Box::new(self.inner),
        }
    }
}

impl<E> Report<E> {
    /// The underlying error.
    pub fn error(&self) -> &E {
        &self.inner.error
    }

    /// The underlying error.
    pub fn error_mut(&mut self) -> &mut E {
        &mut self.inner.error
    }

    /// The underlying error.
    pub fn into_error(self) -> E {
        self.inner.error
    }
}

impl<E: Error> Report<E> {
    /// Create a new report with the given error.
    #[track_caller]
    pub fn new(error: E) -> Self {
        ReportBuilder::new()
            .with_backtrace()
            .with_location()
            .with_error(error)
            .build()
    }

    /// Erase the concrete error type.
    fn erased(self) -> ErasedReport {
        ErasedReport { inner: self.inner }
    }

    /// Add freeform context information to the report.
    pub fn add_info<I: Printable>(&mut self, info: I) {
        self.inner.meta.add_info(info);
    }

    /// Add freeform context information to the report.
    pub fn with_info<I: Printable>(mut self, info: I) -> Self {
        self.add_info(info);
        self
    }
}

impl<E: Error> Debug for Report<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            f.debug_struct("Report")
                .field("error", &self.inner.error)
                .finish_non_exhaustive()
        } else {
            f.write_str(&renderer::render_report(self))
        }
    }
}

/// Static values that implement [`Display`] and can be sent between threads.
pub trait Printable: 'static + Send + Display {}

impl<P: 'static + Send + Display> Printable for P {}

impl Debug for dyn Printable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Printable({:?})", self.to_string())
    }
}

/// Report an error as some other error.
pub trait ReportAs<E>: Error {
    fn report_as(self, builder: ReportBuilder) -> ReportBuilder<E>;
}

impl<E, F> ReportAs<E> for F
where
    F: Into<E> + Error,
    E: Error,
{
    fn report_as(self, builder: ReportBuilder) -> ReportBuilder<E> {
        builder.with_error(self.into())
    }
}

impl<E, F> From<E> for Report<F>
where
    E: ReportAs<F> + Error,
    F: Error,
{
    #[track_caller]
    fn from(value: E) -> Self {
        value.report_as(ReportBuilder::new()).build()
    }
}

/// Propagate an error as some other error.
pub trait PropagateAs<E>: Sized {
    fn propagate_as(report: Report<Self>) -> Report<E>;
}

impl<E, F> PropagateAs<F> for E
where
    E: Error,
    F: Error + for<'error> From<&'error E>,
{
    fn propagate_as(report: Report<Self>) -> Report<F> {
        ReportBuilder::new()
            .with_error(F::from(report.error()))
            .with_cause(report)
            .build()
    }
}

/// Error type that can be constructed from any error.
pub trait Whatever: Error + Sized {
    /// Construct a new error.
    fn new() -> Self;

    /// Construct a new error from an existing error.
    #[expect(unused_variables)]
    fn from_error<E>(error: &E) -> Self
    where
        E: Error,
    {
        Self::new()
    }
}

mod private {
    use crate::Printable;

    pub trait Report {
        type Error: crate::Error;

        fn into_report(self) -> crate::Report<Self::Error>;

        fn as_report(&self) -> &crate::Report<Self::Error>;

        fn as_report_mut(&mut self) -> &mut crate::Report<Self::Error>;

        fn add_context<C>(&mut self, ctx: C)
        where
            C: Printable;
    }

    impl<E: crate::Error> Report for crate::Report<E> {
        type Error = E;

        fn into_report(self) -> crate::Report<Self::Error> {
            self
        }

        fn as_report(&self) -> &crate::Report<Self::Error> {
            self
        }

        fn as_report_mut(&mut self) -> &mut crate::Report<Self::Error> {
            self
        }

        fn add_context<C>(&mut self, ctx: C)
        where
            C: Printable,
        {
            self.inner.meta.add_info(ctx);
        }
    }

    pub enum MaybeReport<E> {
        Error(E),
        Report(crate::Report<E>),
    }

    pub trait ReportOrError {
        type Error: crate::Error;

        fn maybe_report(self) -> MaybeReport<Self::Error>;
    }

    impl<E: crate::Error> ReportOrError for E {
        type Error = E;

        #[inline(always)]
        fn maybe_report(self) -> MaybeReport<Self::Error> {
            MaybeReport::Error(self)
        }
    }

    impl<E: crate::Error> ReportOrError for crate::Report<E> {
        type Error = E;

        #[inline(always)]
        fn maybe_report(self) -> MaybeReport<Self::Error> {
            MaybeReport::Report(self)
        }
    }
}

/// Extension trait for errors.
pub trait ErrorExt {
    /// Report the error.
    fn report<F>(self) -> Report<F>
    where
        Self: ReportAs<F>,
        F: Error;

    /// Report the error using [`Whatever`].
    fn whatever<F, C>(self, description: C) -> Report<F>
    where
        F: Whatever,
        C: Printable;

    /// Report the error using [`Whatever`].
    fn whatever_with<F, C, X>(self, description: X) -> Report<F>
    where
        F: Whatever,
        X: FnOnce(&Self) -> C,
        C: Printable;
}

impl<E: private::ReportOrError> ErrorExt for E {
    #[track_caller]
    fn report<F>(self) -> Report<F>
    where
        Self: ReportAs<F>,
        F: Error,
    {
        self.report_as(ReportBuilder::new()).build()
    }

    #[track_caller]
    fn whatever<F, C>(self, description: C) -> Report<F>
    where
        F: Whatever,
        C: Printable,
    {
        self.whatever_with(|_| description)
    }

    #[track_caller]
    fn whatever_with<F, C, X>(self, description: X) -> Report<F>
    where
        F: Whatever,
        X: FnOnce(&Self) -> C,
        C: Printable,
    {
        let description = description(&self);
        match self.maybe_report() {
            private::MaybeReport::Error(error) => ReportBuilder::new()
                .with_backtrace()
                .with_location()
                .with_error(F::from_error(&error))
                .with_cause(ReportBuilder::new().with_error(error).build()),
            private::MaybeReport::Report(report) => ReportBuilder::new()
                .with_location()
                .with_error(F::from_error(report.error()))
                .with_cause(report),
        }
        .with_description(description)
        .build()
    }
}

#[track_caller]
pub fn whatever<E, C>(description: C) -> Report<E>
where
    E: Whatever,
    C: Printable,
{
    ReportBuilder::new()
        .with_backtrace()
        .with_location()
        .with_description(description)
        .with_error(E::new())
        .build()
}

///
/// ```plain
/// # use reportify::{bail, new_whatever_error};
/// #
/// new_whatever_error! {
///     Whatever
/// }
///
/// pub fn some_function() -> reportify::Result<(), Whatever> {
///     bail!("this is a one-of-a-kind-error");
/// }
/// ```
#[macro_export]
macro_rules! bail {
    ($($args:tt)*) => {
        return Err($crate::whatever(format!($($args)*)))
    };
}

#[macro_export]
macro_rules! whatever {
    ($($args:tt)*) => {
        $crate::whatever(format!($($args)*))
    };
}

#[macro_export]
macro_rules! ensure {
    ($cond:expr, $($args:tt)*) => {
        if !$cond {
            $crate::bail!($($args)*)
        }
    };
}

/// Extension trait for results.
pub trait ResultExt<T, E> {
    fn report<F>(self) -> Result<T, Report<F>>
    where
        E: ReportAs<F>,
        F: Error;

    fn whatever<F, C>(self, description: C) -> Result<T, Report<F>>
    where
        F: Whatever,
        E: private::ReportOrError,
        C: Printable;

    fn whatever_with<F, C, X>(self, description: X) -> Result<T, Report<F>>
    where
        F: Whatever,
        E: private::ReportOrError,
        X: FnOnce(&E) -> C,
        C: Printable;

    fn propagate<F>(self) -> Result<T, Report<F>>
    where
        F: Error,
        E: private::Report,
        E::Error: PropagateAs<F>;

    fn with_info<C, X>(self, ctx: X) -> Self
    where
        E: private::Report,
        X: FnOnce(&E::Error) -> C,
        C: Printable;
}

impl<T, E> ResultExt<T, E> for Result<T, E> {
    #[track_caller]
    fn report<F>(self) -> Result<T, Report<F>>
    where
        E: ReportAs<F>,
        F: Error,
    {
        self.map_err(|error| error.report_as(ReportBuilder::new()).build())
    }

    #[track_caller]
    fn whatever<F, C>(self, description: C) -> Result<T, Report<F>>
    where
        F: Whatever,
        E: private::ReportOrError,
        C: Printable,
    {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(error.whatever(description)),
        }
    }

    #[track_caller]
    fn whatever_with<F, C, X>(self, description: X) -> Result<T, Report<F>>
    where
        F: Whatever,
        E: private::ReportOrError,
        X: FnOnce(&E) -> C,
        C: Printable,
    {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(error.whatever_with(description)),
        }
    }

    fn propagate<F>(self) -> Result<T, Report<F>>
    where
        F: Error,
        E: private::Report,
        E::Error: PropagateAs<F>,
    {
        self.map_err(|report| E::Error::propagate_as(report.into_report()))
    }

    fn with_info<C, X>(mut self, ctx: X) -> Self
    where
        E: private::Report,
        X: FnOnce(&E::Error) -> C,
        C: Printable,
    {
        if let Err(report) = &mut self {
            let ctx = ctx(report.as_report().error());
            report.add_context(ctx);
        }
        self
    }
}

#[macro_export]
macro_rules! new_whatever_type {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug)]
        pub struct $name(());

        impl $crate::Error for $name {
            fn description(&self) -> Option<&dyn ::std::fmt::Display> {
                None
            }
        }

        impl $crate::Whatever for $name {
            fn new() -> Self {
                $name(())
            }
        }
    };
}
