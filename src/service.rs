use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll}, collections::HashMap,
};

use futures_util::{future::{BoxFuture}, FutureExt};
use http::{Request, Response, StatusCode};
use tower::Service;

use crate::{Router, RouteContext};

impl<Body, Data, Error> Service<Request<Body>> for Router<Body, Data, Error>
where
    Body: Default,
    Data: Clone,
{
    type Response = Response<Body>;

    type Error = Error;

    type Future = ResponseFuture<Body, Error>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let uri = req.uri();

        let inner = self.inner.read().unwrap();

        if let Ok(node) = inner.at(uri.path()) {
            let route = node.value;
            let ctx = RouteContext {
                params: {
                    let mut params = HashMap::with_capacity(node.params.len());
                    for (name, value) in node.params.iter() {
                        params.insert(name.into(), value.into());
                    }
                    params
                },
                data: self.data.clone(),
            };

            if let Some(handler) = route.handlers.get(req.method()) {
                return ResponseFuture((handler.0)(req, ctx));
            }

            if let Some(handler) = &route.catchall {
                return ResponseFuture((handler.0)(req, ctx));
            }
        }

        ResponseFuture(Box::pin(async move {
            Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::default())
                .unwrap())
        }))
    }
}

/// A [`Future`] that resolves to a [`Response`](http::Response).
pub struct ResponseFuture<Body, Error>(BoxFuture<'static, Result<Response<Body>, Error>>);

impl<Body, Error> Future for ResponseFuture<Body, Error> {
    type Output = Result<Response<Body>, Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.0.poll_unpin(cx)
    }
}
