use http::{Request, Response, StatusCode};
use hyper::Body;
use ratpack::{app::App, compose_handler, handler::Params, Error, HTTPResult, ServerError};

const DEFAULT_AUTHTOKEN: &str = "867-5309";
const AUTHTOKEN_FILENAME: &str = "authtoken.secret";

async fn validate_authtoken(
    req: Request<Body>,
    resp: Option<Response<Body>>,
    _params: Params,
) -> HTTPResult {
    let token = req.headers().get("X-AuthToken");
    if token.is_none() {
        return Err(Error::StatusCode(StatusCode::UNAUTHORIZED));
    }

    let token = token.unwrap();

    let matches = match std::fs::metadata(AUTHTOKEN_FILENAME) {
        Ok(_) => {
            let s = std::fs::read_to_string(AUTHTOKEN_FILENAME)?;
            s.as_str() == token
        }
        Err(_) => DEFAULT_AUTHTOKEN == token,
    };

    if !matches {
        return Err(Error::StatusCode(StatusCode::UNAUTHORIZED));
    }

    return Ok((req, resp));
}

async fn hello(req: Request<Body>, _resp: Option<Response<Body>>, params: Params) -> HTTPResult {
    let name = params.get("name").unwrap();
    let bytes = Body::from(format!("hello, {}!\n", name));

    return Ok((
        req,
        Some(Response::builder().status(200).body(bytes).unwrap()),
    ));
}

#[tokio::main]
async fn main() -> Result<(), ServerError> {
    let mut app = App::new();
    app.get("/auth/:name", compose_handler!(validate_authtoken, hello));
    app.get("/:name", compose_handler!(hello));

    app.serve("127.0.0.1:3000").await?;

    Ok(())
}
