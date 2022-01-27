use ratpack::prelude::*;

// We'll use authstate to (optionally) capture information about the token
// being correct. if it is Some(true), the user was authed, if None, there was no
// authentication performed.
#[derive(Clone)]
struct AuthedState {
    authed: Option<bool>,
}

// All transient state structs must have an initial state, which will be
// initialized internally in the router.
impl TransientState for AuthedState {
    fn initial() -> Self {
        Self { authed: None }
    }
}

// our authtoken validator, this queries the app state and the header
// `X-AuthToken` and compares the two. If there are any discrepancies, it
// returns `401 Unauthorized`.
//
// every handler & middleware takes and returns the same params and has the
// same prototype.
//
async fn validate_authtoken(
    req: Request<Body>,
    resp: Option<Response<Body>>,
    _params: Params,
    app: App<State, AuthedState>,
    mut authstate: AuthedState,
) -> HTTPResult<AuthedState> {
    if let (Some(token), Some(state)) = (req.headers().get("X-AuthToken"), app.state().await) {
        authstate.authed = Some(state.clone().lock().await.authtoken == token);
        Ok((req, resp, authstate))
    } else {
        Err(Error::StatusCode(
            StatusCode::UNAUTHORIZED,
            String::default(),
        ))
    }
}

// our `hello` responder; it simply echoes the `name` parameter provided in the
// route.
async fn hello(
    req: Request<Body>,
    _resp: Option<Response<Body>>,
    params: Params,
    _app: App<State, AuthedState>,
    authstate: AuthedState,
) -> HTTPResult<AuthedState> {
    let name = params.get("name").unwrap();
    let bytes = Body::from(format!("hello, {}!\n", name));

    if let Some(authed) = authstate.authed {
        if authed {
            return Ok((
                req,
                Some(Response::builder().status(200).body(bytes).unwrap()),
                authstate,
            ));
        }
    } else if authstate.authed.is_none() {
        return Ok((
            req,
            Some(Response::builder().status(200).body(bytes).unwrap()),
            authstate,
        ));
    }

    Err(Error::StatusCode(
        StatusCode::UNAUTHORIZED,
        String::default(),
    ))
}

// Our global application state; must be `Clone`.
#[derive(Clone)]
struct State {
    authtoken: &'static str,
}

// ServerError is a catch-all for errors returned by serving content through
// ratpack.
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
