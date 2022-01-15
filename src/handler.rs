use crate::Params;

use async_trait::async_trait;
use hyper::{Error, Request, Response};

pub type HandlerFunc<T, R> =
    dyn Fn(&Request<T>, &Params, Option<&Response<R>>) -> Result<Response<R>, Error>;

#[async_trait]
pub trait Handler<R>
where
    Self: Send + Sync + 'static,
{
    async fn perform(&self, response: Option<&Response<R>>) -> Result<Response<R>, Error>;
}

pub struct BasicHandler<T, R>
where
    T: Send + Sync + 'static,
    R: Send + Sync + 'static,
{
    req: Request<T>,
    params: Params,
    next: Option<&'static BasicHandler<T, R>>,
    func: &'static HandlerFunc<T, R>,
}

impl<T, R> BasicHandler<T, R>
where
    T: Send + Sync + 'static,
    R: Send + Sync + 'static,
{
}

#[async_trait]
impl<T, R> Handler<R> for BasicHandler<T, R>
where
    Self: Send + Sync,
    R: Copy + Send + Sync + Sized + 'static,
    T: Copy + Send + Sync + Sized + 'static,
{
    async fn perform(&self, response: Option<&Response<R>>) -> Result<Response<R>, Error> {
        let response = (*self.func)(&self.req, &self.params, response)?;
        if self.next.is_some() {
            return Ok(self.next.unwrap().perform(Some(&response)).await?);
        }

        Ok(response)
    }
}
