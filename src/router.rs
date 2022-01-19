use http::Request;

use crate::{handler::Handler, path::Path, Error, HTTPResult};

#[derive(Clone)]
pub struct Route {
    method: http::Method,
    path: Path,
    handler: Handler,
}

impl PartialEq for Route {
    fn eq(&self, other: &Self) -> bool {
        self.method.to_string() == other.method.to_string() && self.path.eq(&other.path)
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
    fn new(method: http::Method, path: &'static str, handler: Handler) -> Self {
        Self {
            method,
            handler,
            path: Path::new(path),
        }
    }

    #[allow(dead_code)]
    async fn dispatch(&self, provided: &'static str, req: Request<hyper::Body>) -> HTTPResult {
        let params = self.path.extract(provided)?;
        self.handler.perform(req, None, params).await
    }
}

#[derive(Clone)]
pub struct Router(Vec<Route>);

impl Router {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn add(&mut self, method: http::Method, path: &'static str, ch: Handler) -> Self {
        self.0.push(Route::new(method, path, ch));
        self.clone()
    }

    pub fn find(&self, req: &'static Request<hyper::Body>) -> Result<Handler, Error> {
        let path = req.uri().path();

        for route_path in self.0.clone() {
            if route_path.path.matches(path) && route_path.method.eq(req.method()) {
                return Ok(route_path.handler);
            }
        }

        Err(Error::new("no route found for request"))
    }
}

mod tests {
    use http::{Request, Response};
    use hyper::Body;

    use crate::{handler::Params, HTTPResult};

    #[allow(dead_code)]
    async fn handler_one(
        req: Request<Body>,
        _response: Option<Response<Body>>,
        _params: Params,
    ) -> HTTPResult {
        return Ok((
            req,
            Some(
                Response::builder()
                    .status(400)
                    .body(Body::from("hello, world".as_bytes()))?,
            ),
        ));
    }

    #[tokio::test]
    async fn test_route_static() {
        use super::Route;
        use crate::handler::Handler;
        use http::Method;

        let route = Route::new(
            Method::GET,
            "/a/b/c",
            Handler::new(
                |req, resp, params| Box::pin(handler_one(req, resp, params)),
                None,
            ),
        );

        assert!(route.dispatch("/a/b/c", Request::default()).await.is_ok());

        let body = hyper::body::to_bytes(
            route
                .dispatch("/a/b/c", Request::default())
                .await
                .unwrap()
                .1
                .unwrap()
                .body_mut(),
        )
        .await
        .unwrap();

        assert_eq!(body, "hello, world".as_bytes());

        let status = route
            .dispatch("/a/b/c", Request::default())
            .await
            .unwrap()
            .1
            .unwrap()
            .status();

        assert_eq!(status, 400);
    }

    #[tokio::test]
    async fn test_router() {}
}
