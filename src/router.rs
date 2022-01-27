use http::{Request, Response};
use hyper::Body;

use crate::{app::App, handler::Handler, path::Path, Error, HTTPResult, TransientState};

#[derive(Clone)]
pub(crate) struct Route<S: Clone + Send, T: TransientState + 'static> {
    method: http::Method,
    path: Path,
    handler: Handler<S, T>,
}

impl<S: Clone + Send, T: TransientState> PartialEq for Route<S, T> {
    fn eq(&self, other: &Self) -> bool {
        self.method.to_string() == other.method.to_string() && self.path.eq(&other.path)
    }
}

impl<S: Clone + Send, T: TransientState> Eq for Route<S, T> {}

impl<S: Clone + Send, T: TransientState> PartialOrd for Route<S, T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<S: Clone + Send, T: TransientState> Ord for Route<S, T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let left = self.method.to_string() + " " + &self.path.to_string();
        let right = other.method.to_string() + " " + &other.path.to_string();

        left.to_string().cmp(&right.to_string())
    }
}

impl<S: Clone + Send, T: TransientState> Route<S, T> {
    fn new(method: http::Method, path: String, handler: Handler<S, T>) -> Self {
        Self {
            method,
            handler,
            path: Path::new(path),
        }
    }

    async fn dispatch(
        &self,
        provided: String,
        req: Request<hyper::Body>,
        app: App<S, T>,
        state: T,
    ) -> HTTPResult<T> {
        let params = self.path.extract(provided)?;

        if self.method != req.method() {
            return Err(Error::StatusCode(
                http::StatusCode::NOT_FOUND,
                String::new(),
            ));
        }

        self.handler.perform(req, None, params, app, state).await
    }
}

#[derive(Clone)]
pub(crate) struct Router<S: Clone + Send, T: TransientState + 'static>(Vec<Route<S, T>>);

impl<S: Clone + Send, T: TransientState + Clone + Send> Router<S, T> {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub(crate) fn add(&mut self, method: http::Method, path: String, ch: Handler<S, T>) -> Self {
        self.0.push(Route::new(method, path, ch));
        self.clone()
    }

    pub(crate) async fn dispatch(
        &self,
        req: Request<Body>,
        app: App<S, T>,
    ) -> Result<Response<Body>, Error> {
        let path = req.uri().path().to_string();

        for route in self.0.clone() {
            if route.path.matches(path.to_string()) && route.method.eq(req.method()) {
                let (_, response, _) = route
                    .dispatch(path.to_string(), req, app, T::initial())
                    .await?;
                if response.is_none() {
                    return Err(Error::StatusCode(
                        http::StatusCode::INTERNAL_SERVER_ERROR,
                        String::new(),
                    ));
                }

                return Ok(response.unwrap());
            }
        }

        Err(Error::StatusCode(
            http::StatusCode::METHOD_NOT_ALLOWED,
            String::new(),
        ))
    }
}

mod tests {
    #[tokio::test]
    async fn test_route_dynamic() {
        use http::{Method, Request, Response};
        use hyper::Body;

        use crate::{app::App, handler::Handler, HTTPResult, NoState, Params};

        use super::Route;

        #[derive(Clone)]
        struct State;

        async fn handler_dynamic(
            req: Request<Body>,
            _response: Option<Response<Body>>,
            params: Params,
            _app: App<State, NoState>,
            _state: NoState,
        ) -> HTTPResult<NoState> {
            return Ok((
                req,
                Some(Response::builder().status(400).body(Body::from(format!(
                    "hello, {}",
                    *params.get("name").unwrap()
                )))?),
                NoState {},
            ));
        }

        let route = Route::new(
            Method::GET,
            "/a/:name/c".to_string(),
            Handler::new(
                |req, resp, params, app, state| {
                    Box::pin(handler_dynamic(req, resp, params, app, state))
                },
                None,
            ),
        );

        assert!(route
            .dispatch("/a".to_string(), Request::default(), App::new(), NoState {})
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
                NoState {},
            )
            .await
            .is_err());

        for name in vec![
            "erik", "adam", "sean", "travis", "joseph", "grant", "joy", "steve", "marc",
        ] {
            assert!(route
                .dispatch(
                    "/a/:name/c".to_string(),
                    Request::default(),
                    App::new(),
                    NoState {}
                )
                .await
                .is_ok());

            let path = format!("/a/{}/c", name);

            let body = hyper::body::to_bytes(
                route
                    .dispatch(path.clone(), Request::default(), App::new(), NoState {})
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
                .dispatch(path, Request::default(), App::new(), NoState {})
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

        use crate::{app::App, handler::Handler, HTTPResult, NoState, Params};

        use super::Route;

        #[derive(Clone)]
        struct State;

        async fn handler_static(
            req: Request<Body>,
            _response: Option<Response<Body>>,
            _params: Params,
            _app: App<State, NoState>,
            _state: NoState,
        ) -> HTTPResult<NoState> {
            return Ok((
                req,
                Some(
                    Response::builder()
                        .status(400)
                        .body(Body::from("hello, world".as_bytes()))?,
                ),
                NoState {},
            ));
        }

        let route = Route::new(
            Method::GET,
            "/a/b/c".to_string(),
            Handler::new(
                |req, resp, params, app, state| {
                    Box::pin(handler_static(req, resp, params, app, state))
                },
                None,
            ),
        );

        assert!(route
            .dispatch("/a".to_string(), Request::default(), App::new(), NoState {})
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
                NoState {},
            )
            .await
            .is_err());

        assert!(route
            .dispatch(
                "/a/b/c".to_string(),
                Request::default(),
                App::new(),
                NoState {}
            )
            .await
            .is_ok());

        let body = hyper::body::to_bytes(
            route
                .dispatch(
                    "/a/b/c".to_string(),
                    Request::default(),
                    App::new(),
                    NoState {},
                )
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
            .dispatch(
                "/a/b/c".to_string(),
                Request::default(),
                App::new(),
                NoState {},
            )
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
            app::App, compose_handler, handler::Handler, HTTPResult, Params, TransientState,
        };
        use http::{Method, Request, Response};
        use hyper::Body;

        #[derive(Clone)]
        struct HelloState {
            name: Option<String>,
        }

        impl TransientState for HelloState {
            fn initial() -> Self {
                Self { name: None }
            }
        }

        #[derive(Clone)]
        struct State;

        async fn handler_dynamic(
            req: Request<Body>,
            _response: Option<Response<Body>>,
            params: Params,
            _app: App<State, HelloState>,
            mut state: HelloState,
        ) -> HTTPResult<HelloState> {
            let name = params.get("name").unwrap().clone();
            state.name = Some(name.clone());

            return Ok((
                req,
                Some(
                    Response::builder()
                        .status(200)
                        .body(Body::from(format!(
                            "hello, {}",
                            state.clone().name.unwrap()
                        )))
                        .unwrap(),
                ),
                state,
            ));
        }

        async fn handler_continued(
            req: Request<Body>,
            _response: Option<Response<Body>>,
            _params: Params,
            _app: App<State, HelloState>,
            state: HelloState,
        ) -> HTTPResult<HelloState> {
            return Ok((
                req,
                Some(
                    Response::builder()
                        .status(200)
                        .body(Body::from(format!(
                            "hello, {}",
                            state.clone().name.unwrap()
                        )))
                        .unwrap(),
                ),
                state,
            ));
        }

        async fn handler_static(
            req: Request<Body>,
            _response: Option<Response<Body>>,
            _params: Params,
            _app: App<State, HelloState>,
            _state: HelloState,
        ) -> HTTPResult<HelloState> {
            return Ok((
                req,
                Some(
                    Response::builder()
                        .status(400)
                        .body(Body::from("hello, world".as_bytes()))?,
                ),
                HelloState::initial(),
            ));
        }

        let mut router = Router::new();

        router.add(
            Method::GET,
            "/a/b/c".to_string(),
            Handler::new(
                |req, resp, params, app, state| {
                    Box::pin(handler_static(req, resp, params, app, state))
                },
                None,
            ),
        );

        router.add(
            Method::GET,
            "/c/b/a/:name".to_string(),
            Handler::new(
                |req, resp, params, app, state| {
                    Box::pin(handler_dynamic(req, resp, params, app, state))
                },
                None,
            ),
        );

        router.add(
            Method::GET,
            "/with_state/:name".to_string(),
            compose_handler!(handler_dynamic, handler_continued),
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

            let response = router
                .dispatch(
                    Request::builder()
                        .uri(&format!("/with_state/{}", name))
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
