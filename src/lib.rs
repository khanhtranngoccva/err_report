use std::any::{type_name};
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Deref, DerefMut};
use std::panic::Location;

pub type AnyContext = dyn Display + Send + Sync + 'static;
pub type AnyError = dyn Error + Sync + Send + 'static;

/// A transparent contextualized error wrapper over an inner error type, with an optional context
/// message and a location specifying where the error occurred.
///
/// Avoid stacking a report inside another report.
pub struct Report<E>
where
    E: ?Sized,
{
    pub inner: Box<E>,
    pub ctx: Option<Box<AnyContext>>,
    pub location: &'static Location<'static>,
}

impl<E> Debug for Report<E>
where
    E: Debug + ?Sized,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(&format!("Report<{}>", type_name::<E>()))
            .field("inner", &self.inner)
            .field("context", &self.ctx.as_ref().map(|f| f.to_string()))
            .field("location", &self.location)
            .finish()
    }
}

impl<E> From<E> for Report<E> {
    #[track_caller]
    #[inline]
    fn from(value: E) -> Self {
        Self {
            inner: Box::new(value),
            ctx: None,
            location: Location::caller(),
        }
    }
}

impl<E> Report<E>
where
    E: ?Sized,
{
    pub fn context<Context>(self, context: Context) -> Self
    where
        Context: Display + Send + Sync + 'static,
    {
        Self {
            inner: self.inner,
            ctx: Some(Box::new(context)),
            location: self.location,
        }
    }
}

impl<E> Report<E>
where
    E: Error + Sync + Send + 'static,
{
    pub fn into_untyped(self) -> Report<AnyError> {
        Report {
            inner: self.inner,
            ctx: self.ctx,
            location: self.location,
        }
    }
}

pub trait IntoReportExt
where
    Self: Error + Sync + Send + 'static,
{
    /// Create a new Report error wrapper object on top of an existing error.
    /// Do not invoke this method on an existing report.
    fn into_report(self) -> Report<Self>;
}

impl<E> IntoReportExt for E
where
    E: Error + Sync + Send + 'static,
{
    #[track_caller]
    #[inline]
    fn into_report(self) -> Report<Self> {
        Report {
            inner: Box::new(self),
            ctx: None,
            location: Location::caller(),
        }
    }
}

pub trait ResultIntoReportExt<T, E>
where
    E: Error + Sync + Send + 'static,
{
    fn report(self) -> Result<T, Report<E>>;
}

impl<T, E> ResultIntoReportExt<T, E> for Result<T, E>
where
    E: Error + Sync + Send + 'static,
{
    /// Attach a report object with the location of the error if
    /// the result type contains an error.
    #[track_caller]
    #[inline]
    fn report(self) -> Result<T, Report<E>> {
        match self {
            Ok(r) => Ok(r),
            Err(e) => Err(e.into_report()),
        }
    }
}

pub trait ResultReportExt<T, E>
where
    E: Error + Sync + Send + 'static,
{
    /// Attach a displayable context object to a result object that may contain an error.
    fn context<Context>(self, context: Context) -> Self
    where
        Context: Display + Send + Sync + 'static;

    /// Convert the error report inside the result object into an untyped error report.
    fn untyped_err(self) -> Result<T, Report<AnyError>>;
}

impl<T, E> ResultReportExt<T, E> for Result<T, Report<E>>
where
    E: Error + Sync + Send + 'static,
{
    fn context<Context>(self, context: Context) -> Self
    where
        Context: Display + Send + Sync + 'static,
    {
        match self {
            Ok(r) => Ok(r),
            Err(e) => Err(e.context(context)),
        }
    }

    fn untyped_err(self) -> Result<T, Report<AnyError>> {
        match self {
            Ok(r) => Ok(r),
            Err(e) => Err(e.into_untyped()),
        }
    }
}

impl<T> Display for Report<T>
where
    T: Display + ?Sized,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.ctx {
            Some(ctx) => f.write_fmt(format_args!("{}: {} @ {}", self.inner, ctx, self.location)),
            None => f.write_fmt(format_args!("{} @ {}", self.inner, self.location)),
        }
    }
}

impl<T> Deref for Report<T>
where
    T: Error + ?Sized,
{
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for Report<T>
where
    T: Error + ?Sized,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T> Error for Report<T>
where
    T: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.inner.source()
    }
}

impl Error for Report<AnyError> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.inner.source()
    }
}