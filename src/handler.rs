use std::{collections::BTreeMap, future::Future};

use crate::{HTTPResult, PinBox};
use async_recursion::async_recursion;

use http::{Request, Response};
use hyper::Body;

pub type Params = BTreeMap<String, String>;

pub type HandlerFunc = fn(
    req: Request<Body>,
    response: Option<Response<Body>>,
    params: Params,
) -> PinBox<dyn Future<Output = HTTPResult> + Send>;

#[derive(Clone)]
pub struct Handler {
    handler: HandlerFunc,
    next: Box<Option<Handler>>,
}

impl Handler
where
    Self: Send,
{
    pub fn new(handler: HandlerFunc, next: Option<Handler>) -> Self {
        Self {
            handler,
            next: Box::new(next),
        }
    }

    #[async_recursion]
    pub async fn perform(
        &self,
        req: Request<hyper::Body>,
        response: Option<Response<hyper::Body>>,
        params: Params,
    ) -> HTTPResult {
        let (req, response) = (self.handler)(req, response, params.clone()).await?;
        if self.next.is_some() {
            return Ok((*self.clone().next)
                .unwrap()
                .perform(req, response, params)
                .await?);
        }

        Ok((req, response))
    }
}

mod tests {
    #[tokio::test]
    async fn test_handler_basic() {
        use crate::{Error, HTTPResult};
        use http::{HeaderValue, Request, Response, StatusCode};
        use hyper::Body;

        use super::Params;

        // this method adds a header:
        // wakka: wakka wakka
        // to the request. that's it!
        async fn one(
            mut req: Request<Body>,
            _response: Option<Response<Body>>,
            _params: Params,
        ) -> HTTPResult {
            let headers = req.headers_mut();
            headers.insert("wakka", HeaderValue::from_str("wakka wakka").unwrap());
            Ok((req, None))
        }

        // this method returns an OK status when the wakka header exists.
        async fn two(
            req: Request<Body>,
            mut response: Option<Response<Body>>,
            _params: Params,
        ) -> HTTPResult {
            if let Some(header) = req.headers().get("wakka") {
                if header != "wakka wakka" {
                    return Err(Error::new("invalid header value"));
                }

                if response.is_some() {
                    return Ok((req, response));
                } else {
                    let resp = Response::builder()
                        .status(StatusCode::OK)
                        .body(Body::default())?;
                    response.replace(resp);

                    return Ok((req, response));
                }
            }

            Err(Error::default())
        }

        // single stage handler that never yields a response
        let bh = super::Handler::new(|req, resp, params| Box::pin(one(req, resp, params)), None);
        let req = Request::default();
        let (req, response) = bh.perform(req, None, Params::new()).await.unwrap();

        assert!(req.headers().get("wakka").is_some());
        assert!(response.is_none());

        // two-stage handler; yields a response if the first one was good.
        let bh_two =
            super::Handler::new(|req, resp, params| Box::pin(two(req, resp, params)), None);
        let bh = super::Handler::new(
            |req, resp, params| Box::pin(one(req, resp, params)),
            Some(bh_two.clone()),
        );
        let (_, response) = bh.perform(req, None, Params::new()).await.unwrap();

        assert!(response.is_some() && response.unwrap().status() == StatusCode::OK);

        assert!(bh_two
            .perform(Request::default(), None, Params::new())
            .await
            .is_err());

        drop(bh)
    }
}
