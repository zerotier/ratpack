pub mod handler;
pub mod macros;
pub mod path;
pub mod router;

use handler::Handler;
use http::{Request, Response};

use std::{collections::BTreeMap, pin::Pin};

pub(crate) type PinBox<F> = Pin<Box<F>>;

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

impl From<http::Error> for Error {
    fn from(e: http::Error) -> Self {
        Self::new(e)
    }
}

pub type HTTPResult = Result<(Request<hyper::Body>, Option<Response<hyper::Body>>), Error>;

pub struct App {
    #[allow(dead_code)] // FIXME remove
    routes: BTreeMap<String, Handler>,
}
