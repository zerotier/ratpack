use std::{convert::Infallible, net::SocketAddr, sync::Arc};

use http::{Method, Request, Response, StatusCode};
use hyper::{server::conn::Http, service::service_fn, Body};
use tokio::{net::TcpListener, sync::Mutex};

use crate::{handler::Handler, router::Router, Error, ServerError};

/// App is used to define application-level functionality and initialize the server. Routes are
/// typically programmed here.
///
/// ```ignore
///   async fn item(
///         req: Request<Body>,
///         resp: Option<Response<Body>>,
///         params: Params,
///         app: App<()>
///   ) -> HTTPResult {
///     Ok((
///        req,
///        Response::builder().
///             status(StatusCode::OK).
///             body(Body::default()).
///             unwrap()
///     ))
///   }
///
///   #[tokio::main]
///   async fn main() -> Result<(), ServerError> {
///     let app = App::new();
///     app.get("/:item", compose_handler!(item));
///     app.serve("localhost:0").await
///   }
/// ```
///
/// Note that App here has _no state_. It will have a type signature of `App<()>`. To carry state,
/// look at the `with_state` method which will change the type signature of the `item` call (and
/// other handlers).
///
#[derive(Clone)]
pub struct App<S: Clone + Send> {
    router: Router<S>,
    global_state: Option<Arc<Mutex<S>>>,
}

impl<S: 'static + Clone + Send> App<S> {
    /// Construct a new App with no state; it will be passed to handlers as `App<()>`.
    pub fn new() -> Self {
        Self {
            router: Router::new(),
            global_state: None,
        }
    }

    /// Construct an App with state.
    ///
    /// This has the type `App<S>` where S is `+ 'static + Clone + Send` and will be passed to
    /// handlers with the appropriate concrete type.
    ///
    pub fn with_state(state: S) -> Self {
        Self {
            router: Router::new(),
            global_state: Some(Arc::new(Mutex::new(state))),
        }
    }

    // FIXME Currently you must await this, seems pointless.
    /// Return the state of the App. This is returned as `Arc<Mutex<S>>` and must be acquired under
    /// lock. In situations where there is no state, [std::option::Option::None] is returned.
    pub async fn state(&self) -> Option<Arc<Mutex<S>>> {
        self.global_state.clone()
    }

    /// Create a route for a GET request. See [crate::path::Path] and [crate::handler::Handler] for
    /// more information.
    pub fn get(&mut self, path: &str, ch: Handler<S>) {
        self.router.add(Method::GET, path.to_string(), ch);
    }

    /// Create a route for a POST request. See [crate::path::Path] and [crate::handler::Handler] for
    /// more information.
    pub fn post(&mut self, path: &str, ch: Handler<S>) {
        self.router.add(Method::POST, path.to_string(), ch);
    }

    /// Create a route for a DELETE request. See [crate::path::Path] and [crate::handler::Handler] for
    /// more information.
    pub fn delete(&mut self, path: &str, ch: Handler<S>) {
        self.router.add(Method::DELETE, path.to_string(), ch);
    }

    /// Create a route for a PUT request. See [crate::path::Path] and [crate::handler::Handler] for
    /// more information.
    pub fn put(&mut self, path: &str, ch: Handler<S>) {
        self.router.add(Method::PUT, path.to_string(), ch);
    }

    /// Create a route for an OPTIONS request. See [crate::path::Path] and
    /// [crate::handler::Handler] for more information.
    pub fn options(&mut self, path: &str, ch: Handler<S>) {
        self.router.add(Method::OPTIONS, path.to_string(), ch);
    }

    /// Create a route for a PATCH request. See [crate::path::Path] and
    /// [crate::handler::Handler] for more information.
    pub fn patch(&mut self, path: &str, ch: Handler<S>) {
        self.router.add(Method::PATCH, path.to_string(), ch);
    }

    /// Create a route for a HEAD request. See [crate::path::Path] and
    /// [crate::handler::Handler] for more information.
    pub fn head(&mut self, path: &str, ch: Handler<S>) {
        self.router.add(Method::HEAD, path.to_string(), ch);
    }

    /// Create a route for a CONNECT request. See [crate::path::Path] and
    /// [crate::handler::Handler] for more information.
    pub fn connect(&mut self, path: &str, ch: Handler<S>) {
        self.router.add(Method::CONNECT, path.to_string(), ch);
    }

    /// Create a route for a TRACE request. See [crate::path::Path] and
    /// [crate::handler::Handler] for more information.
    pub fn trace(&mut self, path: &str, ch: Handler<S>) {
        self.router.add(Method::TRACE, path.to_string(), ch);
    }

    /// Dispatch a route based on the request. Returns a response based on the error status of the
    /// handler chain following the normal chain of responsibility rules described elsewhere. Only
    /// needed by server implementors.
    pub async fn dispatch(&self, req: Request<Body>) -> Result<Response<Body>, Infallible> {
        match self.router.dispatch(req, self.clone()).await {
            Ok(resp) => Ok(resp),
            Err(e) => match e {
                Error::StatusCode(sc) => Ok(Response::builder()
                    .status(sc)
                    .body(Body::default())
                    .unwrap()),
                Error::InternalServerError(_) => Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::default())
                    .unwrap()),
            },
        }
    }

    /// Start a TCP/HTTP server with tokio. Performs dispatch on an as-needed basis. This is a more
    /// common path for users to start a server.
    pub async fn serve(self, addr: &str) -> Result<(), ServerError> {
        let socketaddr: SocketAddr = addr.parse()?;

        let tcp_listener = TcpListener::bind(socketaddr).await?;
        loop {
            let s = self.clone();
            let sfn = service_fn(move |req: Request<Body>| {
                let s = s.clone();
                async move { s.clone().dispatch(req).await }
            });
            let (tcp_stream, _) = tcp_listener.accept().await?;
            tokio::task::spawn(async move {
                if let Err(http_err) = Http::new()
                    .http1_keep_alive(true)
                    .serve_connection(tcp_stream, sfn)
                    .await
                {
                    eprintln!("Error while serving HTTP connection: {}", http_err);
                }
            });
        }
    }
}
