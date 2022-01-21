use http::{Request, Response, StatusCode};
use hyper::Body;
use ratpack::{app::App, compose_handler, handler::Params, Error, HTTPResult, ServerError};

async fn validate_authtoken(
    req: Request<Body>,
    resp: Option<Response<Body>>,
    _params: Params,
    app: App<State>,
) -> HTTPResult {
    let token = req.headers().get("X-AuthToken");
    if token.is_none() {
        return Err(Error::StatusCode(StatusCode::UNAUTHORIZED));
    }

    let token = token.unwrap();

    let state = app.state().await;
    if state.is_none() {
        return Err(Error::StatusCode(StatusCode::UNAUTHORIZED));
    }

    let state = state.unwrap();

    if !(state.clone().lock().await.authtoken == token) {
        return Err(Error::StatusCode(StatusCode::UNAUTHORIZED));
    }

    return Ok((req, resp));
}

async fn hello(
    req: Request<Body>,
    _resp: Option<Response<Body>>,
    params: Params,
    _app: App<State>,
) -> HTTPResult {
    let name = params.get("name").unwrap();
    let bytes = Body::from(format!("hello, {}!\n", name));

    return Ok((
        req,
        Some(Response::builder().status(200).body(bytes).unwrap()),
    ));
}

#[derive(Clone)]
struct State {
    authtoken: &'static str,
}

#[tokio::main]
async fn main() -> Result<(), ServerError> {
    let mut app = App::with_state(State {
        authtoken: "867-5309",
    });
    app.get("/auth/:name", compose_handler!(validate_authtoken, hello));
    app.get("/:name", compose_handler!(hello));

    app.serve("127.0.0.1:3000").await?;

    Ok(())
}
