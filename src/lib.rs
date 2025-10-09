use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Deref, DerefMut};
use std::panic::Location;

pub type AnyError = dyn Error + Send + Sync + 'static;

pub struct Report<E>
where
    E: ?Sized,
{
    pub inner: Box<E>,
    pub ctx: Option<Box<dyn Display + Send + Sync + 'static>>,
    pub location: &'static Location<'static>,
}

impl<E> Error for Report<E>
where
    E: Error + ?Sized,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.inner.source()
    }
}

impl<E> Debug for Report<E>
where
    E: Debug + ?Sized,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Report")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<E> Display for Report<E>
where
    E: Display + ?Sized,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.ctx {
            Some(ctx) => f.write_fmt(format_args!("{}: {} @ {}", self.inner, ctx, self.location)),
            None => f.write_fmt(format_args!("{} @ {}", self.inner, self.location)),
        }
    }
}

impl<E> Deref for Report<E>
where
    E: ?Sized,
{
    type Target = E;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<E> DerefMut for Report<E>
where
    E: ?Sized,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<E> Report<E>
where
    E: ?Sized,
{
    #[track_caller]
    #[inline]
    pub fn new(e: E) -> Self
    where
        E: Sized,
    {
        Self {
            inner: Box::new(e),
            location: Location::caller(),
            ctx: None,
        }
    }

    pub fn into_untyped(self) -> Report<AnyError>
    where
        E: Error + Sync + Send + Sized + 'static,
    {
        Report {
            inner: self.inner,
            ctx: self.ctx,
            location: self.location,
        }
    }

    pub fn context<Context>(self, context: Context) -> Report<E>
    where
        Context: Display + Send + Sync + 'static,
    {
        Report {
            inner: self.inner,
            ctx: Some(Box::new(context)),
            location: self.location,
        }
    }

    pub fn raw_message(&self) -> String
    where
        E: Display,
    {
        self.inner.to_string()
    }
}

impl<E> From<E> for Report<E> {
    #[track_caller]
    #[inline]
    fn from(e: E) -> Self {
        Self::new(e)
    }
}

impl<E> From<Report<E>> for Report<AnyError>
where
    E: Error + Sync + Send + 'static,
{
    fn from(value: Report<E>) -> Self {
        value.into_untyped()
    }
}

impl From<Box<AnyError>> for Report<AnyError> {
    #[track_caller]
    #[inline]
    fn from(value: Box<AnyError>) -> Self {
        Self {
            inner: value,
            location: Location::caller(),
            ctx: None,
        }
    }
}

pub trait IntoReportExt<E>
where
    E: ?Sized,
{
    fn into_report(self) -> Report<E>;
}

impl<E> IntoReportExt<E> for E {
    #[track_caller]
    #[inline]
    fn into_report(self) -> Report<E> {
        Report::new(self)
    }
}

impl IntoReportExt<AnyError> for Box<AnyError> {
    #[track_caller]
    #[inline]
    fn into_report(self) -> Report<AnyError> {
        Report {
            inner: self,
            location: Location::caller(),
            ctx: None,
        }
    }
}

pub trait ResultIntoReportExt<T, E> {
    fn report(self) -> Result<T, Report<E>>
    where
        Self: Sized;

    fn report_with_context<Context>(self, context: Context) -> Result<T, Report<E>>
    where
        Self: Sized,
        Context: Display + Sync + Send + 'static;

    fn untyped_report(self) -> Result<T, Report<AnyError>>
    where
        E: Error + Send + Sync + 'static,
        Self: Sized;
}

impl<T, E> ResultIntoReportExt<T, E> for Result<T, E> {
    #[track_caller]
    #[inline]
    fn report(self) -> Result<T, Report<E>> {
        match self {
            Ok(r) => Ok(r),
            Err(e) => Err(Report::new(e))
        }
    }

    #[track_caller]
    #[inline]
    fn report_with_context<Context>(self, context: Context) -> Result<T, Report<E>>
    where
        Self: Sized,
        Context: Display + Sync + Send + 'static,
    {
        match self {
            Ok(r) => Ok(r),
            Err(e) => Err(Report::new(e).context(context))
        }
    }

    #[track_caller]
    #[inline]
    fn untyped_report(self) -> Result<T, Report<AnyError>>
    where
        Self: Sized,
        E: Error + Send + Sync + 'static,
    {
        match self {
            Ok(r) => Ok(r),
            Err(e) => Err(Report::new(e).into_untyped())
        }
    }
}

pub trait ResultReportExt<T, E> {
    fn untyped_err(self) -> Result<T, Report<AnyError>>
    where
        Self: Sized;

    fn context<Context>(self, context: Context) -> Result<T, Report<E>>
    where
        Self: Sized,
        Context: Display + Sync + Send + 'static;
}

impl<T, E> ResultReportExt<T, E> for Result<T, Report<E>>
where
    E: Error + Send + Sync + 'static,
{
    fn untyped_err(self) -> Result<T, Report<AnyError>> {
        let res = self.map_err(|e| e.into_untyped());
        res
    }

    fn context<Context>(self, context: Context) -> Result<T, Report<E>>
    where
        Self: Sized,
        Context: Display + Sync + Send + 'static,
    {
        self.map_err(|e| e.context(context))
    }
}
