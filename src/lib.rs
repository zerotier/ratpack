pub mod handler;

use http::{Request, Response};

use crate::handler::BasicHandler;

use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct Params(BTreeMap<String, String>);

impl Default for Params {
    fn default() -> Self {
        Self(BTreeMap::default())
    }
}

#[derive(Clone, Debug)]
pub struct Error(String);

impl Default for Error {
    fn default() -> Self {
        Self(String::from("internal server error"))
    }
}

impl Error {
    pub fn new<T>(message: T) -> Self
    where
        T: ToString,
    {
        Self(message.to_string())
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
    routes: Vec<&'static BasicHandler>,
}
