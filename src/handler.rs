use std::{future::Future, rc::Rc, sync::Arc};

use futures_util::{
    future::{BoxFuture, LocalBoxFuture},
    FutureExt,
};
use http::{Request, Response};

use crate::{unsync, RouteContext};

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

type UnsyncFunc<Body, Data, Error> = dyn Fn(
        Request<Body>,
        unsync::RouteContext<Data>,
    ) -> LocalBoxFuture<'static, Result<Response<Body>, Error>>
    + 'static;

#[derive(Clone)]
pub struct AsyncUnsyncHandler<Body, Data, Error>(pub Rc<UnsyncFunc<Body, Data, Error>>);

impl<Body, Data, Error, HandlerFn, Fut> From<HandlerFn> for AsyncUnsyncHandler<Body, Data, Error>
where
    HandlerFn: Fn(Request<Body>, unsync::RouteContext<Data>) -> Fut,
    HandlerFn: 'static,
    Fut: Future<Output = Result<Response<Body>, Error>> + 'static,
{
    fn from(value: HandlerFn) -> Self {
        Self(Rc::new(move |req, data| value(req, data).boxed_local()))
    }
}
