use http::StatusCode;
use hyper::{Body, Response, Server};
use router_service::Router;
use tower::make::Shared;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let router = Router::new()
        .get("/", |_, _| async move {
            Response::builder().body(Body::from("Hello, World!"))
        })
        .any("/*catchall", |_, ctx| async move {
            let body = format!("no endpoint \"/{}\"", ctx.param("catchall").unwrap());
            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from(body))
        });

    let addr = ([127, 0, 0, 1], 3030).into();
    Server::bind(&addr).serve(Shared::new(router)).await?;

    Ok(())
}
