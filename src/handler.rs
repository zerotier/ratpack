use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{HTTPResult, Params};

use async_trait::async_trait;
use http::{Request, Response};

pub type HandlerFunc<'a, T, R> =
    dyn Fn(&'a Request<T>, Params, Option<&'a Response<R>>) -> HTTPResult<'a, T, R>;

#[async_trait]
pub trait Handler<'a, 'b, T, R>
where
    Self: Send + Sync + 'b,
{
    async fn perform(&'b self, response: Option<&'a Response<R>>) -> HTTPResult<'a, T, R>;
}

pub struct BasicHandler<'a, T, R>
where
    T: Send + Sync + 'a,
    R: Send + Sync + 'a,
{
    req: Arc<Mutex<Request<T>>>,
    params: Params,
    next: Option<&'a BasicHandler<'a, T, R>>,
    func: &'a HandlerFunc<'a, T, R>,
}

impl<'a, 'b, T, R> BasicHandler<'b, T, R>
where
    T: Send + Sync + 'b,
    R: Send + Sync + 'b,
{
    pub fn new(
        req: Request<T>,
        params: Params,
        next: Option<&'b BasicHandler<'b, T, R>>,
        func: &'static HandlerFunc<'b, T, R>,
    ) -> Self {
        Self {
            req: Arc::new(Mutex::new(req)),
            params,
            next,
            func,
        }
    }
}

#[async_trait]
impl<'a, 'b, T, R> Handler<'a, 'b, T, R> for BasicHandler<'b, T, R>
where
    Self: Send + Sync + 'b,
    R: Copy + Send + Sync + Sized + 'static,
    T: Copy + Send + Sync + Sized + 'static,
{
    async fn perform(&'b self, response: Option<&'a Response<R>>) -> HTTPResult<'a, T, R> {
        let mut req = self.req.lock().await;

        let (req, response) = (*self.func)(&mut req, self.params, response)?;
        if self.next.is_some() {
            return Ok(self.next.unwrap().perform(response).await?);
        }

        Ok((req, response))
    }
}

mod tests {
    use crate::{Error, HTTPResult, Params};
    use http::{HeaderValue, Request, Response, StatusCode};
    use hyper::Body;

    fn one<'a>(
        mut req: &'a Request<Body>,
        _params: Params,
        _response: Option<&'a Response<Body>>,
    ) -> HTTPResult<'a, Body, Body> {
        let headers = req.headers_mut();
        headers.insert("wakka", HeaderValue::from_str("wakka wakka").unwrap());
        Ok((&req, None))
    }

    fn two<'a>(
        mut req: &'a Request<Body>,
        _params: Params,
        response: Option<&'a Response<Body>>,
    ) -> HTTPResult<'a, Body, Body> {
        if let Some(header) = req.headers().get("wakka") {
            if header != "wakka wakka" {
                return Err(Error::new("invalid header value"));
            }

            if response.is_some() {
                return Ok((&req, response));
            } else {
                response.replace(
                    &Response::builder()
                        .status(StatusCode::OK)
                        .body(Body::default())?,
                );

                return Ok((&req, response));
            }
        }

        Err(Error::default())
    }

    #[test]
    fn test_handler_basic() {
        let bh = super::BasicHandler::new(Request::default(), Params::default(), None, &one);
    }
}
