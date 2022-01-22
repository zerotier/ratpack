/// compose_handler allows you to combine multiple [crate::handler::HandlerFunc] functions into a
/// single [crate::handler::Handler], so that they cascade through a chain of responsibility. This
/// means that each handler will feed its output into the input of the next. To start, the first
/// [http::Response] is [std::option::Option::None], and the final return Response must be
/// non-None; otherwise a 500 Internal Server Error is returned. Handlers may do anything they wish
/// to the [http::Request] between processing periods, including replacing the request entirely.
#[macro_export]
macro_rules! compose_handler {
    ($( $x:path ),*) => {
        {
            use $crate::handler::{HandlerFunc, Handler};
            {
                let mut funcs: Vec<HandlerFunc<_, _>> = Vec::new();

                $(
                    funcs.push(|req, resp, params, app, state| Box::pin($x(req, resp, params, app, state)));
                )*

                if funcs.len() == 0 {
                    panic!("compose_handler requires at least one handler to be supplied")
                }


                let mut handlers = Vec::new();
                let mut last: Option<Handler<_, _>> = None;
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

        use crate::{app::App, Error, HTTPResult, NoState, Params};

        #[derive(Clone)]
        struct State;

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

        let handler = compose_handler!(one, two);

        let (req, response, _) = handler
            .perform(
                Request::default(),
                None,
                Params::new(),
                App::new(),
                NoState {},
            )
            .await
            .unwrap();

        assert_eq!(req.headers().get("wakka").unwrap(), "wakka wakka");
        assert_eq!(response.unwrap().status(), StatusCode::OK);

        let handler = compose_handler!(one);

        let (req, response, _) = handler
            .perform(
                Request::default(),
                None,
                Params::new(),
                App::new(),
                NoState {},
            )
            .await
            .unwrap();

        assert_eq!(req.headers().get("wakka").unwrap(), "wakka wakka");
        assert!(response.is_none());

        let handler = compose_handler!(two);

        assert!(handler
            .perform(
                Request::default(),
                None,
                Params::new(),
                App::new(),
                NoState {}
            )
            .await
            .is_err());
    }
}
