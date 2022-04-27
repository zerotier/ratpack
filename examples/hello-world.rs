use ratpack::prelude::*;
#[cfg(feature = "unix")]
use std::path::PathBuf;

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

    #[cfg(feature = "unix")]
    {
        std::fs::remove_file("/tmp/server.sock").unwrap_or_default();
        eprintln!("Serving over /tmp/server.sock");
        app.serve_unix(PathBuf::from("/tmp/server.sock")).await?;
    }
    #[cfg(not(feature = "unix"))]
    {
        eprintln!("Serving over 127.0.0.1:3000");
        app.serve("127.0.0.1:3000").await?;
    }

    Ok(())
}
