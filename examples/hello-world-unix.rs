use std::path::PathBuf;

use ratpack::prelude::*;

async fn hello(
    req: Request<Body>,
    _resp: Option<Response<Body>>,
    params: Params,
    _app: App<(), NoState>,
    _state: NoState,
) -> HTTPResult<NoState> {
    let name = params.get("name").unwrap();
    let bytes = Body::from(format!("hello, {}!\n", name));

    return Ok((
        req,
        Some(Response::builder().status(200).body(bytes).unwrap()),
        NoState {},
    ));
}

#[tokio::main]
async fn main() -> Result<(), ServerError> {
    let mut app = App::new();
    app.get("/:name", compose_handler!(hello));

    app.serve_unix(PathBuf::from("/tmp/server.sock")).await?;

    Ok(())
}
