use http::{Request, Response};
use hyper::Body;
use log::LevelFilter;
use ratpack::{app::App, compose_handler, handler::Params, HTTPResult, ServerError};

async fn log(req: Request<Body>, resp: Option<Response<Body>>, _params: Params) -> HTTPResult {
    log::trace!("New request: {}", req.uri().path());
    Ok((req, resp))
}

async fn hello(req: Request<Body>, _resp: Option<Response<Body>>, params: Params) -> HTTPResult {
    let name = params.get("name").unwrap();
    log::info!("Saying hello to {}", name);
    let bytes = Body::from(format!("hello, {}!\n", name));

    return Ok((
        req,
        Some(Response::builder().status(200).body(bytes).unwrap()),
    ));
}

#[tokio::main]
async fn main() -> Result<(), ServerError> {
    let mut app = App::new();
    env_logger::builder()
        .target(env_logger::Target::Stderr)
        .filter(None, LevelFilter::Trace)
        .init();
    app.get("/:name", compose_handler!(log, hello));

    app.serve("127.0.0.1:3000").await?;

    Ok(())
}
