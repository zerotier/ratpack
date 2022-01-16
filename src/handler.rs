use std::{collections::BTreeMap, sync::Arc};

use crate::HTTPResult;

use async_trait::async_trait;
use http::{Request, Response};

pub(crate) type Params = BTreeMap<&'static str, &'static str>;

pub type HandlerFunc =
    dyn Fn(Request<hyper::Body>, Option<Response<hyper::Body>>, Params) -> HTTPResult + Sync;

#[async_trait]
pub trait Handler
where
    Self: Sync + Sized,
{
    async fn perform(
        &self,
        req: Request<hyper::Body>,
        response: Option<Response<hyper::Body>>,
        params: Params,
    ) -> HTTPResult;
}

#[derive(Clone)]
pub struct BasicHandler
where
    Self: Sync + Sized,
{
    next: Option<Arc<BasicHandler>>,
    func: &'static HandlerFunc,
}

impl BasicHandler
where
    Self: Sync + Sized,
{
    pub fn new(next: Option<Arc<BasicHandler>>, func: &'static HandlerFunc) -> Self {
        Self { next, func }
    }
}

#[async_trait]
impl Handler for BasicHandler
where
    Self: Sync + Sized,
{
    async fn perform(
        &self,
        req: Request<hyper::Body>,
        response: Option<Response<hyper::Body>>,
        params: Params,
    ) -> HTTPResult {
        let (req, response) = (*self.func)(req, response, params.clone())?;
        if self.next.is_some() {
            return Ok(self
                .next
                .clone()
                .unwrap()
                .perform(req, response, params)
                .await?);
        }

        Ok((req, response))
    }
}

mod tests {
    use crate::{Error, HTTPResult};
    use http::{HeaderValue, Request, Response, StatusCode};
    use hyper::Body;

    use super::Params;

    // this method adds a header:
    // wakka: wakka wakka
    // to the request. that's it!
    #[allow(dead_code)]
    fn one(
        mut req: Request<Body>,
        _response: Option<Response<Body>>,
        _params: Params,
    ) -> HTTPResult {
        let headers = req.headers_mut();
        headers.insert("wakka", HeaderValue::from_str("wakka wakka").unwrap());
        Ok((req, None))
    }

    // this method returns an OK status when the wakka header exists.
    #[allow(dead_code)]
    fn two(
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

    // orchestration!!!!
    #[tokio::test]
    async fn test_handler_basic() {
        use super::Handler;
        use std::sync::Arc;

        // single stage handler that never yields a response
        let bh = super::BasicHandler::new(None, &one);
        let req = Request::default();
        let (req, response) = bh.perform(req, None, Params::new()).await.unwrap();
        if !req.headers().get("wakka").is_some() {
            panic!("no wakkas")
        }

        if response.is_some() {
            panic!("response should be none at this point")
        }

        // two-stage handler; yields a response if the first one was good.
        let bh_two = super::BasicHandler::new(None, &two);
        let bh = super::BasicHandler::new(Some(Arc::new(bh_two.clone())), &one);
        let (_, response) = bh.perform(req, None, Params::new()).await.unwrap();

        if !(response.is_some() && response.unwrap().status() == StatusCode::OK) {
            panic!("response not ok")
        }

        if !bh_two
            .perform(Request::default(), None, Params::new())
            .await
            .is_err()
        {
            panic!("no error")
        }
    }
}
