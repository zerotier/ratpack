use std::future::Future;

use crate::{app::App, HTTPResult, PinBox, TransientState};
use async_recursion::async_recursion;

use http::{Request, Response};
use hyper::Body;

/// HandlerFunc is the type signature of all handlers. All handlers must conform to this pattern to
/// be used with [crate::compose_handler!].
///
/// Example:
///
/// ```
/// use ratpack::prelude::*;
///
/// async fn hello(
///     req: Request<Body>,
///     _resp: Option<Response<Body>>,
///     params: Params,
///     _app: App<(), NoState>,
///     _state: NoState,
/// ) -> HTTPResult<NoState> {
///     let name = params.get("name").unwrap();
///     let bytes = Body::from(format!("hello, {}!\n", name));
///
///     return Ok((
///         req,
///         Some(Response::builder().status(200).body(bytes).unwrap()),
///         NoState{},
///     ));
/// }
/// ```
///
pub type HandlerFunc<S, T> = fn(
    req: Request<Body>,
    response: Option<Response<Body>>,
    params: crate::Params,
    app: App<S, T>,
    state: T,
) -> PinBox<dyn Future<Output = HTTPResult<T>> + Send>;

/// Handler is the structure of the handler. Typically, you will not use this directly, and instead
/// interact with the [crate::compose_handler!] macro. That said, if you wanted to define your own
/// macros or otherwise compose more complicated structures for your handlers, this is available to
/// you.
#[derive(Clone)]
pub struct Handler<S: Clone + Send, T: TransientState + 'static> {
    handler: HandlerFunc<S, T>,
    next: Box<Option<Handler<S, T>>>,
}

impl<S: Clone + Send, T: TransientState> Handler<S, T>
where
    Self: Send,
    S: Clone + Send,
{
    /// Construct a new handler composed of a HandlerFunc with state, and an optional next handler
    /// in the chain.
    pub fn new(handler: HandlerFunc<S, T>, next: Option<Handler<S, T>>) -> Self {
        Self {
            handler,
            next: Box::new(next),
        }
    }

    /// Perform the function, this will recursively execute all handlers in the chain.
    #[async_recursion]
    pub async fn perform(
        &self,
        req: Request<hyper::Body>,
        response: Option<Response<hyper::Body>>,
        params: crate::Params,
        app: App<S, T>,
        state: T,
    ) -> HTTPResult<T> {
        let (req, response, state) =
            (self.handler)(req, response, params.clone(), app.clone(), state).await?;
        if self.next.is_some() {
            return Ok((*self.clone().next)
                .unwrap()
                .perform(req, response, params, app, state)
                .await?);
        }

        Ok((req, response, state))
    }
}

mod tests {
    #[tokio::test]
    async fn test_handler_basic() {
        use crate::{app::App, Error, HTTPResult, NoState, Params};
        use http::{HeaderValue, Request, Response, StatusCode};
        use hyper::Body;

        #[derive(Clone)]
        struct State;

        #[derive(Clone)]
        struct TransientState;

        // this method adds a header:
        // wakka: wakka wakka
        // to the request. that's it!
        async fn one(
            mut req: Request<Body>,
            _response: Option<Response<Body>>,
            _params: Params,
            _app: App<State, NoState>,
            _state: NoState,
        ) -> HTTPResult<NoState> {
            let headers = req.headers_mut();
            headers.insert("wakka", HeaderValue::from_str("wakka wakka").unwrap());
            Ok((req, None, NoState {}))
        }

        // this method returns an OK status when the wakka header exists.
        async fn two(
            req: Request<Body>,
            mut response: Option<Response<Body>>,
            _params: Params,
            _app: App<State, NoState>,
            _state: NoState,
        ) -> HTTPResult<NoState> {
            if let Some(header) = req.headers().get("wakka") {
                if header != "wakka wakka" {
                    return Err(Error::new("invalid header value"));
                }

                if response.is_some() {
                    return Ok((req, response, NoState {}));
                } else {
                    let resp = Response::builder()
                        .status(StatusCode::OK)
                        .body(Body::default())?;
                    response.replace(resp);

                    return Ok((req, response, NoState {}));
                }
            }

            Err(Error::default())
        }

        // single stage handler that never yields a response
        let bh = super::Handler::new(
            |req, resp, params, app, state| Box::pin(one(req, resp, params, app, state)),
            None,
        );
        let req = Request::default();
        let (req, response, _) = bh
            .perform(req, None, Params::new(), App::new(), NoState {})
            .await
            .unwrap();

        assert!(req.headers().get("wakka").is_some());
        assert!(response.is_none());

        // two-stage handler; yields a response if the first one was good.
        let bh_two = super::Handler::new(
            |req, resp, params, app, state| Box::pin(two(req, resp, params, app, state)),
            None,
        );
        let bh = super::Handler::new(
            |req, resp, params, app, state| Box::pin(one(req, resp, params, app, state)),
            Some(bh_two.clone()),
        );
        let (_, response, _) = bh
            .perform(req, None, Params::new(), App::new(), NoState {})
            .await
            .unwrap();

        assert!(response.is_some() && response.unwrap().status() == StatusCode::OK);

        assert!(bh_two
            .perform(
                Request::default(),
                None,
                Params::new(),
                App::new(),
                NoState {}
            )
            .await
            .is_err());

        drop(bh)
    }
}
