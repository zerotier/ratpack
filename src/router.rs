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
    fn new(method: http::Method, path: String, handler: Handler) -> Self {
        Self {
            method,
            handler,
            path: Path::new(path),
        }
    }

    #[allow(dead_code)]
    async fn dispatch(&self, provided: String, req: Request<hyper::Body>) -> HTTPResult {
        let params = self.path.extract(provided)?;

        if self.method != req.method() {
            return Err(Error(http::StatusCode::NOT_FOUND.to_string()));
        }

        self.handler.perform(req, None, params).await
    }
}

#[derive(Clone)]
pub struct Router(Vec<Route>);

impl Router {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn add(&mut self, method: http::Method, path: String, ch: Handler) -> Self {
        self.0.push(Route::new(method, path, ch));
        self.clone()
    }

    pub fn find(&self, req: &'static Request<hyper::Body>) -> Result<Handler, Error> {
        let path = req.uri().path();

        for route_path in self.0.clone() {
            if route_path.path.matches(path.to_string()) && route_path.method.eq(req.method()) {
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
    async fn handler_static(
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

    #[allow(dead_code)]
    async fn handler_dynamic(
        req: Request<Body>,
        _response: Option<Response<Body>>,
        params: Params,
    ) -> HTTPResult {
        return Ok((
            req,
            Some(Response::builder().status(400).body(Body::from(format!(
                "hello, {}",
                *params.get("name").unwrap()
            )))?),
        ));
    }

    #[tokio::test]
    async fn test_route_dynamic() {
        use super::Route;
        use crate::handler::Handler;
        use http::Method;

        let route = Route::new(
            Method::GET,
            "/a/:name/c".to_string(),
            Handler::new(
                |req, resp, params| Box::pin(handler_dynamic(req, resp, params)),
                None,
            ),
        );

        assert!(route
            .dispatch("/a".to_string(), Request::default())
            .await
            .is_err());
        assert!(route
            .dispatch(
                "/a/b/c".to_string(),
                Request::builder()
                    .method(Method::POST)
                    .body(Body::from("one=two".as_bytes()))
                    .unwrap(),
            )
            .await
            .is_err());

        for name in vec!["erik", "adam", "sean", "travis", "joseph", "grant"] {
            assert!(route
                .dispatch("/a/:name/c".to_string(), Request::default())
                .await
                .is_ok());

            let path = format!("/a/{}/c", name);

            let body = hyper::body::to_bytes(
                route
                    .dispatch(path.clone(), Request::default())
                    .await
                    .unwrap()
                    .1
                    .unwrap()
                    .body_mut(),
            )
            .await
            .unwrap();

            assert_eq!(body, format!("hello, {}", name).as_bytes());

            let status = route
                .dispatch(path, Request::default())
                .await
                .unwrap()
                .1
                .unwrap()
                .status();

            assert_eq!(status, 400);
        }
    }

    #[tokio::test]
    async fn test_route_static() {
        use super::Route;
        use crate::handler::Handler;
        use http::Method;

        let route = Route::new(
            Method::GET,
            "/a/b/c".to_string(),
            Handler::new(
                |req, resp, params| Box::pin(handler_static(req, resp, params)),
                None,
            ),
        );

        assert!(route
            .dispatch("/a".to_string(), Request::default())
            .await
            .is_err());
        assert!(route
            .dispatch(
                "/a/b/c".to_string(),
                Request::builder()
                    .method(Method::POST)
                    .body(Body::from("one=two".as_bytes()))
                    .unwrap(),
            )
            .await
            .is_err());

        assert!(route
            .dispatch("/a/b/c".to_string(), Request::default())
            .await
            .is_ok());

        let body = hyper::body::to_bytes(
            route
                .dispatch("/a/b/c".to_string(), Request::default())
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
            .dispatch("/a/b/c".to_string(), Request::default())
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
