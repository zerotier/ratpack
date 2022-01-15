## http API idea

```rust
async fn root(req: Request) -> Result<Response, Error> {
    Ok(Response::builder()
        .status(StatusCode::OK)
        .body("hello, world!")
        .build()?)
}

async fn with_param(req: Request, params: Params) -> Result<Response, Error> {
    let param = params.get("param")?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(&format!("hello, {}!", param))
        .build()?)
}

async fn with_param_post(req: Request, params: Params) -> Result<Response, Error> {
    let param = params.get("param")?;
    let body = req.body().await?; // should also support iter() probably for large requests
    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(&format!("hello, {}!\n{}\n", param, body))
        .build()?)
}

#[derive(Clone, Debug)]
struct ToggleMiddleware {
    state: AtomicBool,
}

#[async_trait]
impl Middleware for ToggleMiddleware {
    async fn perform(
        &mut self,
        req: Request,
        params: Params,
        next: Next,
    ) -> Result<Response, Error> {
        self.state.swap(self.state.get(), Ordering::Relaxed);
        let response = next.call(req).await?;
        let headers = response.headers_mut();
        headers.insert("state", &format!("{:?}", self.state.get()));
        Ok(response)
    }
}

#[tokio::main]
async fn main() {
    let mut app = Server::new();
    app.with(ToggleMiddleware::new());
    app.get("/", root);
    app.get("/:param", with_param);
    app.post("/:param", with_param_post);
    app.listen("127.0.0.1:3000");
}
```
