#[macro_export]
macro_rules! compose_handler {
    ($( $x:path ),*) => {
        {
            use crate::handler::{HandlerFunc, Handler};
            {
                let mut funcs: Vec<HandlerFunc> = Vec::new();

                $(
                    funcs.push(|req, resp, params| Box::pin($x(req, resp, params)));
                )*

                if funcs.len() == 0 {
                    panic!("compose_handler requires at least one handler to be supplied")
                }


                let mut handlers = Vec::new();
                let mut last: Option<Handler> = None;
                funcs.reverse();

                for func in funcs {
                    last = Some(Handler::new(func, last.clone()));
                    handlers.push(last.clone());
                }

                last.unwrap()
            }
        }
    };
}

mod tests {
    #[tokio::test]
    async fn test_handler_macro() {
        use http::{HeaderValue, Request, Response, StatusCode};
        use hyper::Body;

        use crate::{handler::Params, Error, HTTPResult};

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

        let handler = compose_handler!(one, two);

        let (req, response) = handler
            .perform(Request::default(), None, Params::new())
            .await
            .unwrap();

        assert_eq!(req.headers().get("wakka").unwrap(), "wakka wakka");
        assert_eq!(response.unwrap().status(), StatusCode::OK);

        let handler = compose_handler!(one);

        let (req, response) = handler
            .perform(Request::default(), None, Params::new())
            .await
            .unwrap();

        assert_eq!(req.headers().get("wakka").unwrap(), "wakka wakka");
        assert!(response.is_none());
    }
}
