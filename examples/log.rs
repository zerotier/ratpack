#[cfg(all(feature = "logging", not(feature = "trace")))]
use log::LevelFilter;

#[cfg(feature = "trace")]
use tracing::Level;

use ratpack::prelude::*;

async fn log(
    req: Request<Body>,
    resp: Option<Response<Body>>,
    _params: Params,
    _app: App<(), NoState>,
    _state: NoState,
) -> HTTPResult<NoState> {
    #[cfg(all(feature = "logging", not(feature = "trace")))]
    log::trace!("New request: {}", req.uri().path());
    #[cfg(feature = "trace")]
    tracing::trace!("New request: {}", req.uri().path());

    Ok((req, resp, NoState {}))
}

async fn hello(
    req: Request<Body>,
    _resp: Option<Response<Body>>,
    params: Params,
    _app: App<(), NoState>,
    _state: NoState,
) -> HTTPResult<NoState> {
    let name = params.get("name").unwrap();

    #[cfg(all(feature = "logging", not(feature = "trace")))]
    log::info!("Saying hello to {}", name);
    #[cfg(feature = "trace")]
    tracing::info!("Saying hello to {}", name);

    let bytes = Body::from(format!("hello, {}!\n", name));

    return Ok((
        req,
        Some(Response::builder().status(200).body(bytes).unwrap()),
        NoState,
    ));
}

#[tokio::main]
async fn main() -> Result<(), ServerError> {
    #[cfg(all(feature = "logging", not(feature = "trace")))]
    env_logger::builder()
        .target(env_logger::Target::Stderr)
        .filter(None, LevelFilter::Trace)
        .init();

    #[cfg(feature = "trace")]
    {
        let subscriber = tracing_subscriber::FmtSubscriber::builder()
            // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
            // will be written to stdout.
            .with_max_level(Level::TRACE)
            // completes the builder.
            .finish();

        tracing::subscriber::set_global_default(subscriber)
            .expect("setting default subscriber failed");
    }

    let mut app = App::new();
    app.get("/:name", compose_handler!(log, hello));

    app.serve("127.0.0.1:3000").await?;

    Ok(())
}
