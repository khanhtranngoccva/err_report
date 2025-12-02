use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Deref, DerefMut};
use std::panic::Location;

pub type AnyError = dyn Error + Send + Sync + 'static;

pub struct Layer {
    pub context: Option<Box<dyn Display + Send + Sync + 'static>>,
    pub location: &'static Location<'static>,
}

impl Display for Layer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.context {
            Some(context) => write!(f, "{} @ {}", context, self.location),
            None => write!(f, "@ {}", self.location),
        }
    }
}

pub struct Report<E>
where
    E: ?Sized,
{
    pub inner: Box<E>,
    pub layers: Vec<Layer>,
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
        let layer_string = self
            .layers
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        write!(f, "{}: {}", self.inner, layer_string)
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
            layers: vec![Layer {
                context: None,
                location: Location::caller(),
            }],
        }
    }

    pub fn into_untyped(self) -> Report<AnyError>
    where
        E: Error + Sync + Send + Sized + 'static,
    {
        Report {
            inner: self.inner,
            layers: self.layers,
        }
    }

    pub fn context<Ctx>(self, context: Ctx) -> Report<E>
    where
        Ctx: Display + Send + Sync + 'static,
    {
        let mut layers = self.layers;
        let first_layer = layers
            .first_mut()
            .expect("Report objects must have at least one layer");
        first_layer.context = Some(Box::new(context));
        Report {
            inner: self.inner,
            layers,
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
            layers: vec![Layer {
                context: None,
                location: Location::caller(),
            }],
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
            layers: vec![Layer {
                context: None,
                location: Location::caller(),
            }],
        }
    }
}

pub trait ResultIntoReportExt<T, E> {
    fn report(self) -> Result<T, Report<E>>
    where
        Self: Sized;

    fn report_with_context<Ctx>(self, context: Ctx) -> Result<T, Report<E>>
    where
        Self: Sized,
        Ctx: Display + Sync + Send + 'static;

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
            Err(e) => Err(Report::new(e)),
        }
    }

    #[track_caller]
    #[inline]
    fn report_with_context<Ctx>(self, context: Ctx) -> Result<T, Report<E>>
    where
        Self: Sized,
        Ctx: Display + Sync + Send + 'static,
    {
        match self {
            Ok(r) => Ok(r),
            Err(e) => Err(Report::new(e).context(context)),
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
            Err(e) => Err(Report::new(e).into_untyped()),
        }
    }
}

/// Specialization for Report containers so that we do not end up with a Report wrapper for a Report.
impl<T, E> ResultIntoReportExt<T, E> for Result<T, Report<E>> {
    #[track_caller]
    #[inline]
    fn report(self) -> Result<T, Report<E>>
    where
        Self: Sized,
    {
        match self {
            Ok(r) => Ok(r),
            Err(e) => {
                let new_context = Layer {
                    context: None,
                    location: Location::caller(),
                };
                let mut layers = e.layers;
                layers.insert(0, new_context);
                Err(Report {
                    inner: e.inner,
                    layers,
                })
            }
        }
    }

    fn report_with_context<Ctx>(self, context: Ctx) -> Result<T, Report<E>>
    where
        Self: Sized,
        Ctx: Display + Sync + Send + 'static,
    {
        match self {
            Ok(r) => Ok(r),
            Err(e) => {
                let new_context = Layer {
                    context: Some(Box::new(context)),
                    location: Location::caller(),
                };
                let mut layers = e.layers;
                layers.insert(0, new_context);
                Err(Report {
                    inner: e.inner,
                    layers,
                })
            }
        }
    }

    fn untyped_report(self) -> Result<T, Report<AnyError>>
    where
        E: Error + Send + Sync + 'static,
        Self: Sized,
    {
        match self {
            Ok(r) => Ok(r),
            Err(e) => {
                let new_context = Layer {
                    context: None,
                    location: Location::caller(),
                };
                let mut layers = e.layers;
                layers.insert(0, new_context);
                Err(Report {
                    inner: e.inner,
                    layers,
                })
            }
        }
    }
}

pub trait ResultReportExt<T, E> {
    fn untyped_err(self) -> Result<T, Report<AnyError>>
    where
        Self: Sized;

    fn context<Ctx>(self, context: Ctx) -> Result<T, Report<E>>
    where
        Self: Sized,
        Ctx: Display + Sync + Send + 'static;
}

impl<T, E> ResultReportExt<T, E> for Result<T, Report<E>>
where
    E: Error + Send + Sync + 'static,
{
    fn untyped_err(self) -> Result<T, Report<AnyError>> {
        let res = self.map_err(|e| e.into_untyped());
        res
    }

    fn context<Ctx>(self, context: Ctx) -> Result<T, Report<E>>
    where
        Self: Sized,
        Ctx: Display + Sync + Send + 'static,
    {
        self.map_err(|e| e.context(context))
    }
}
