/// Application/Server-level management and routing configuration; outermost functionality.
pub mod app;
/// Handler construction and prototypes
pub mod handler;
/// Macros for quality-of-life when interacting with Handlers
pub mod macros;
/// Path management for Routes
pub mod path;
/// Router, Route management and organization
pub mod router;

use http::{Request, Response};
use std::pin::Pin;

pub(crate) type PinBox<F> = Pin<Box<F>>;

/// An error for server-related issues.
#[derive(Debug, Clone)]
pub struct ServerError(String);

impl<T> From<T> for ServerError
where
    T: ToString,
{
    fn from(t: T) -> Self {
        ServerError(t.to_string())
    }
}

/// General errors for ratpack handlers. Yield either a StatusCode for a literal status, or a
/// String for a 500 Internal Server Error. Other status codes should be yielded through
/// [http::Response] returns.
#[derive(Clone, Debug)]
pub enum Error {
    StatusCode(http::StatusCode),
    InternalServerError(String),
}

impl Default for Error {
    fn default() -> Self {
        Self::InternalServerError("internal server error".to_string())
    }
}

impl Error {
    /// Convenience method to pass anything in that accepts a .to_string method.
    pub fn new<T>(message: T) -> Self
    where
        T: ToString,
    {
        Self::InternalServerError(message.to_string())
    }

    /// A convenient way to return status codes.
    pub fn new_status(error: http::StatusCode) -> Self {
        Self::StatusCode(error)
    }
}

impl<T> From<T> for Error
where
    T: ToString,
{
    fn from(t: T) -> Self {
        Self::new(t.to_string())
    }
}

/// HTTPResult is the return type for handlers. If a handler terminates at the end of its chain
/// with [std::option::Option::None] as the [http::Response], a 500 Internal Server Error will be
/// returned.
pub type HTTPResult = Result<(Request<hyper::Body>, Option<Response<hyper::Body>>), Error>;
