use log::LevelFilter;
use ratpack::prelude::*;

async fn log(
    req: Request<Body>,
    resp: Option<Response<Body>>,
    _params: Params,
    _app: App<()>,
) -> HTTPResult {
    log::trace!("New request: {}", req.uri().path());
    Ok((req, resp))
}

async fn hello(
    req: Request<Body>,
    _resp: Option<Response<Body>>,
    params: Params,
    _app: App<()>,
) -> HTTPResult {
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
    env_logger::builder()
        .target(env_logger::Target::Stderr)
        .filter(None, LevelFilter::Trace)
        .init();

    let mut app = App::new();
    app.get("/:name", compose_handler!(log, hello));

    app.serve("127.0.0.1:3000").await?;

    Ok(())
}
