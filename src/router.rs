use http::{Request, Response};
use hyper::Body;

use crate::{app::App, handler::Handler, path::Path, Error, HTTPResult};

#[derive(Clone)]
pub struct Route<S: Clone + Send> {
    method: http::Method,
    path: Path,
    handler: Handler<S>,
}

impl<S: Clone + Send> PartialEq for Route<S> {
    fn eq(&self, other: &Self) -> bool {
        self.method.to_string() == other.method.to_string() && self.path.eq(&other.path)
    }
}

impl<S: Clone + Send> Eq for Route<S> {}

impl<S: Clone + Send> PartialOrd for Route<S> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<S: Clone + Send> Ord for Route<S> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let left = self.method.to_string() + " " + &self.path.to_string();
        let right = other.method.to_string() + " " + &other.path.to_string();

        left.to_string().cmp(&right.to_string())
    }
}

impl<S: Clone + Send> Route<S> {
    fn new(method: http::Method, path: String, handler: Handler<S>) -> Self {
        Self {
            method,
            handler,
            path: Path::new(path),
        }
    }

    #[allow(dead_code)]
    async fn dispatch(
        &self,
        provided: String,
        req: Request<hyper::Body>,
        app: App<S>,
    ) -> HTTPResult {
        let params = self.path.extract(provided)?;

        if self.method != req.method() {
            return Err(Error::StatusCode(http::StatusCode::NOT_FOUND));
        }

        self.handler.perform(req, None, params, app).await
    }
}

#[derive(Clone)]
pub struct Router<S: Clone + Send>(Vec<Route<S>>);

impl<S: Clone + Send> Router<S> {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub(crate) fn add(&mut self, method: http::Method, path: String, ch: Handler<S>) -> Self {
        self.0.push(Route::new(method, path, ch));
        self.clone()
    }

    pub(crate) async fn dispatch(
        &self,
        req: Request<Body>,
        app: App<S>,
    ) -> Result<Response<Body>, Error> {
        let path = req.uri().path().to_string();

        for route in self.0.clone() {
            if route.path.matches(path.to_string()) && route.method.eq(req.method()) {
                let params = route.path.extract(path)?;
                let (_, response) = route.handler.perform(req, None, params, app).await?;
                if response.is_none() {
                    return Err(Error::StatusCode(http::StatusCode::INTERNAL_SERVER_ERROR));
                }

                return Ok(response.unwrap());
            }
        }

        Err(Error::StatusCode(http::StatusCode::NOT_FOUND))
    }
}

mod tests {

    #[tokio::test]
    async fn test_route_dynamic() {
        use http::{Method, Request, Response};
        use hyper::Body;

        use crate::{
            app::App,
            handler::{Handler, Params},
            HTTPResult,
        };

        use super::Route;

        #[derive(Clone)]
        struct State;

        async fn handler_dynamic(
            req: Request<Body>,
            _response: Option<Response<Body>>,
            params: Params,
            _app: App<State>,
        ) -> HTTPResult {
            return Ok((
                req,
                Some(Response::builder().status(400).body(Body::from(format!(
                    "hello, {}",
                    *params.get("name").unwrap()
                )))?),
            ));
        }

        let route = Route::new(
            Method::GET,
            "/a/:name/c".to_string(),
            Handler::new(
                |req, resp, params, app| Box::pin(handler_dynamic(req, resp, params, app)),
                None,
            ),
        );

        assert!(route
            .dispatch("/a".to_string(), Request::default(), App::new())
            .await
            .is_err());
        assert!(route
            .dispatch(
                "/a/b/c".to_string(),
                Request::builder()
                    .method(Method::POST)
                    .body(Body::from("one=two".as_bytes()))
                    .unwrap(),
                App::new(),
            )
            .await
            .is_err());

        for name in vec![
            "erik", "adam", "sean", "travis", "joseph", "grant", "joy", "steve", "marc",
        ] {
            assert!(route
                .dispatch("/a/:name/c".to_string(), Request::default(), App::new())
                .await
                .is_ok());

            let path = format!("/a/{}/c", name);

            let body = hyper::body::to_bytes(
                route
                    .dispatch(path.clone(), Request::default(), App::new())
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
                .dispatch(path, Request::default(), App::new())
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
        use http::{Method, Request, Response};
        use hyper::Body;

        use crate::{
            app::App,
            handler::{Handler, Params},
            HTTPResult,
        };

        use super::Route;

        #[derive(Clone)]
        struct State;

        async fn handler_static(
            req: Request<Body>,
            _response: Option<Response<Body>>,
            _params: Params,
            _app: App<State>,
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

        let route = Route::new(
            Method::GET,
            "/a/b/c".to_string(),
            Handler::new(
                |req, resp, params, app| Box::pin(handler_static(req, resp, params, app)),
                None,
            ),
        );

        assert!(route
            .dispatch("/a".to_string(), Request::default(), App::new())
            .await
            .is_err());
        assert!(route
            .dispatch(
                "/a/b/c".to_string(),
                Request::builder()
                    .method(Method::POST)
                    .body(Body::from("one=two".as_bytes()))
                    .unwrap(),
                App::new(),
            )
            .await
            .is_err());

        assert!(route
            .dispatch("/a/b/c".to_string(), Request::default(), App::new())
            .await
            .is_ok());

        let body = hyper::body::to_bytes(
            route
                .dispatch("/a/b/c".to_string(), Request::default(), App::new())
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
            .dispatch("/a/b/c".to_string(), Request::default(), App::new())
            .await
            .unwrap()
            .1
            .unwrap()
            .status();

        assert_eq!(status, 400);
    }

    #[tokio::test]
    async fn test_router() {
        use super::Router;
        use crate::{
            app::App,
            handler::{Handler, Params},
            HTTPResult,
        };
        use http::{Method, Request, Response};
        use hyper::Body;

        #[derive(Clone)]
        struct State;

        async fn handler_dynamic(
            req: Request<Body>,
            _response: Option<Response<Body>>,
            params: Params,
            _app: App<State>,
        ) -> HTTPResult {
            return Ok((
                req,
                Some(Response::builder().status(400).body(Body::from(format!(
                    "hello, {}",
                    *params.get("name").unwrap()
                )))?),
            ));
        }

        async fn handler_static(
            req: Request<Body>,
            _response: Option<Response<Body>>,
            _params: Params,
            _app: App<State>,
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

        let mut router = Router::new();

        router.add(
            Method::GET,
            "/a/b/c".to_string(),
            Handler::new(
                |req, resp, params, app| Box::pin(handler_static(req, resp, params, app)),
                None,
            ),
        );

        router.add(
            Method::GET,
            "/c/b/a/:name".to_string(),
            Handler::new(
                |req, resp, params, app| Box::pin(handler_dynamic(req, resp, params, app)),
                None,
            ),
        );

        let response = router
            .dispatch(
                Request::builder()
                    .uri("/a/b/c")
                    .method(Method::GET)
                    .body(Body::default())
                    .unwrap(),
                App::new(),
            )
            .await;
        assert!(response.is_ok());

        let body = hyper::body::to_bytes(response.unwrap()).await.unwrap();
        assert_eq!(body, "hello, world".as_bytes());

        for name in vec![
            "erik", "adam", "sean", "travis", "joseph", "grant", "joy", "steve", "marc",
        ] {
            let response = router
                .dispatch(
                    Request::builder()
                        .uri(&format!("/c/b/a/{}", name))
                        .method(Method::GET)
                        .body(Body::default())
                        .unwrap(),
                    App::new(),
                )
                .await;
            assert!(response.is_ok());

            let body = hyper::body::to_bytes(response.unwrap()).await.unwrap();
            assert_eq!(body, format!("hello, {}", name).as_bytes());
        }

        for bad_route in vec!["/", "/bad", "/bad/route", "/a/b/c/param", "/c/b/a/0/bad"] {
            let response = router
                .dispatch(
                    Request::builder()
                        .uri(bad_route)
                        .method(Method::GET)
                        .body(Body::default())
                        .unwrap(),
                    App::new(),
                )
                .await;
            assert!(response.is_err());
        }
    }
}
