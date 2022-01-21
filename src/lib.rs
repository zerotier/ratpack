pub mod app;
pub mod handler;
pub mod macros;
pub mod path;
pub mod router;

use http::{Request, Response};
use std::pin::Pin;

pub(crate) type PinBox<F> = Pin<Box<F>>;

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
    pub fn new<T>(message: T) -> Self
    where
        T: ToString,
    {
        Self::InternalServerError(message.to_string())
    }

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

pub type HTTPResult = Result<(Request<hyper::Body>, Option<Response<hyper::Body>>), Error>;
