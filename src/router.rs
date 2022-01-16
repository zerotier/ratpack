use std::collections::{BTreeMap, BTreeSet, HashSet};

use http::Request;

use crate::{
    handler::{BasicHandler, Handler},
    path::Path,
    Error, HTTPResult,
};

#[derive(Clone)]
pub struct Route {
    method: http::Method,
    path: Path,
    handler: BasicHandler,
}

impl PartialEq for Route {
    fn eq(&self, other: &Self) -> bool {
        let left = self.method.to_string() + " " + &self.path.to_string();
        let right = other.method.to_string() + " " + &other.path.to_string();
        left == right
    }
}

impl Eq for Route {}

impl PartialOrd for Route {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Route {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let left = self.method.to_string() + " " + &self.path.to_string();
        let right = other.method.to_string() + " " + &other.path.to_string();

        left.to_string().cmp(&right.to_string())
    }
}

impl Route {
    fn new(method: http::Method, path: &'static str, handler: BasicHandler) -> Self {
        Self {
            method,
            handler,
            path: Path::new(path),
        }
    }

    async fn dispatch(&self, provided: &'static str, req: Request<hyper::Body>) -> HTTPResult {
        let params = self.path.extract(provided)?;
        self.handler.perform(req, None, params).await
    }
}

#[derive(Clone)]
pub struct Router(BTreeSet<Route>);

impl Router {
    pub fn new() -> Self {
        Self(BTreeSet::new())
    }

    pub fn add(&mut self, method: http::Method, path: &'static str, bh: BasicHandler) -> Self {
        self.0.insert(Route::new(method, path, bh));
        self.clone()
    }

    pub fn find(&self, req: &'static Request<hyper::Body>) -> Result<BasicHandler, Error> {
        let path = req.uri().path();

        for route_path in self.0.clone() {
            if route_path.path.matches(path) && route_path.method.eq(req.method()) {
                return Ok(route_path.handler);
            }
        }

        Err(Error::new("no route found for request"))
    }
}
