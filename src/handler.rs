use std::{future::Future, sync::Arc};

use futures_util::future::BoxFuture;
use http::{Request, Response};

use crate::RouteContext;

type Func<Body, Data, Error> = dyn Fn(Request<Body>, RouteContext<Data>) -> BoxFuture<'static, Result<Response<Body>, Error>>
    + Sync
    + Send
    + 'static;

#[derive(Clone)]
pub struct AsyncHandler<Body, Data, Error>(pub Arc<Func<Body, Data, Error>>);

impl<Body, Data, Error, HandlerFn, Fut> From<HandlerFn> for AsyncHandler<Body, Data, Error>
where
    HandlerFn: Fn(Request<Body>, RouteContext<Data>) -> Fut,
    HandlerFn: Sync + Send + 'static,
    Fut: Future<Output = Result<Response<Body>, Error>> + Send + Sync + 'static,
{
    fn from(value: HandlerFn) -> Self {
        Self(Arc::new(move |req, data| Box::pin(value(req, data))))
    }
}
