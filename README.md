# ratpack: a simpleton's HTTP framework (for rust-lang)

`ratpack` is idealized in the simplicity of the [sinatra](http://sinatrarb.com/) (ruby) framework in its goal, and attempts to be an alternative to other async HTTP frameworks such as [tower](https://github.com/tower-rs/tower), [warp](https://github.com/seanmonstar/warp), [axum](https://github.com/tokio-rs/axum), and [tide](https://github.com/http-rs/tide).

`ratpack` tries to deliver a promise that _any handler can also be middleware_, by implementing a "chain of responsibility" pattern that crosses handler boundaries. In summary, what you return from the first handler is fed to the second, which returns to the third, until all handlers are processed, or an error is received. Errors can return valid status codes or plain text errors in the form of a HTTP 500 (Internal Service Error).

## What ratpack is not

- Complicated: `ratpack` is not especially designed for services with a large web of routes or complicated interactions with the HTTP protocol, such as SSE or Websockets (at this time, at least). `ratpack` is very focused on somewhat typical request/response cycles.
- Verbose: `ratpack` tries very hard to make both its internals and your interaction with it _the simplest thing that could possibly work_. This means that your request handlers are functions you pass to a macro called `compose_handler!` which you pass to routing calls, and that likely, you won't be spending your time implementing complicated, extremely verbose traits or even need complicated understandings of how futures and `async` work.
- Focused on one platform: while at this time we only directly support `tokio`, nothing is keeping us from moving into `smol` and `async-std`'s territory. The majority of `ratpack`'s use of `async` are futures that `tokio` ends up leveraging from a very high level.

## Example

Here is an example which carries global _application state_ as an authentication token validator middleware handler, which then passes forward to a greeting handler. The greeting handler can also be re-used without authentication at a different endpoint, which is also demonstrated.

**Note:** this is available at [examples/auth-with-state.rs](examples/auth-with-state.rs). It can also be run with cargo: `cargo run --example auth-with-state`.

```rust
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
    authstate.authed = Some(state.clone().lock().await.authtoken == token);

    return Ok((req, resp, authstate));
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
    if authstate.authed.is_some() && !authstate.authed.unwrap() {
        return Err(Error::StatusCode(StatusCode::UNAUTHORIZED));
    }

    let name = params.get("name").unwrap();
    let bytes = Body::from(format!("hello, {}!\n", name));

    return Ok((
        req,
        Some(Response::builder().status(200).body(bytes).unwrap()),
        authstate,
    ));
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
```

Hitting this service with `curl` gives the result you'd expect:

```
% curl localhost:3000/erik
hello, erik!

% curl -D- localhost:3000/auth/erik
HTTP/1.1 401 Unauthorized
content-length: 0
date: Fri, 21 Jan 2022 18:29:03 GMT

% curl -D- -H "X-AuthToken: 867-5309" localhost:3000/auth/erik
HTTP/1.1 200 OK
content-length: 13
date: Fri, 21 Jan 2022 18:29:19 GMT

hello, erik!
```

## More information & documentation

For more information, see the [docs](https://docs.rs/ratpack/latest/ratpack/).

## Author

Erik Hollensbe <erik.hollensbe@zerotier.com>

## License

BSD 3-Clause
