use std::{convert::Infallible, net::SocketAddr};

use http::{Method, Request, Response, StatusCode};
use hyper::{server::conn::Http, service::service_fn, Body};
use tokio::net::TcpListener;

use crate::{handler::Handler, router::Router, Error, ServerError};

pub struct App {
    router: Router,
}

impl App {
    pub fn new() -> Self {
        Self {
            router: Router::new(),
        }
    }

    pub fn get(&mut self, path: &str, ch: Handler) {
        self.router.add(Method::GET, path.to_string(), ch);
    }

    pub fn post(&mut self, path: &str, ch: Handler) {
        self.router.add(Method::POST, path.to_string(), ch);
    }

    pub fn delete(&mut self, path: &str, ch: Handler) {
        self.router.add(Method::DELETE, path.to_string(), ch);
    }

    pub fn put(&mut self, path: &str, ch: Handler) {
        self.router.add(Method::PUT, path.to_string(), ch);
    }

    pub fn options(&mut self, path: &str, ch: Handler) {
        self.router.add(Method::OPTIONS, path.to_string(), ch);
    }

    pub fn patch(&mut self, path: &str, ch: Handler) {
        self.router.add(Method::PATCH, path.to_string(), ch);
    }

    pub fn head(&mut self, path: &str, ch: Handler) {
        self.router.add(Method::HEAD, path.to_string(), ch);
    }

    pub fn connect(&mut self, path: &str, ch: Handler) {
        self.router.add(Method::CONNECT, path.to_string(), ch);
    }

    pub fn trace(&mut self, path: &str, ch: Handler) {
        self.router.add(Method::TRACE, path.to_string(), ch);
    }

    pub async fn dispatch(&self, req: Request<Body>) -> Result<Response<Body>, Infallible> {
        match self.router.dispatch(req).await {
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

    pub async fn serve(&'static self, addr: String) -> Result<(), ServerError> {
        let socketaddr: SocketAddr = addr.parse()?;

        let s = self.clone();
        let sfn = service_fn(move |req: Request<Body>| s.dispatch(req));

        let tcp_listener = TcpListener::bind(socketaddr).await?;
        loop {
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
