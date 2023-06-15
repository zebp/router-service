//! An unsynchronized router that can be used as a [`Service`](tower::Service).
use std::future::Future;
use std::sync::RwLock;
use std::{collections::HashMap, sync::Arc};

use http::{Method, Request, Response};
use matchit::Router as MatchRouter;

use crate::handler::*;

pub use crate::service::ResponseFuture;

#[derive(Default)]
struct Route<Body, Data, Error> {
    handlers: HashMap<Method, AsyncUnsyncHandler<Body, Data, Error>>,
    catchall: Option<AsyncUnsyncHandler<Body, Data, Error>>,
}

/// A router that can be used as a [`Service`](tower::Service).
///
/// # Example
/// ```
/// # futures::executor::block_on(async move {
/// use std::convert::Infallible;
///
/// use http::{Request, Response, StatusCode};
/// use tower::Service;
/// use router_service::Router;
///
/// let mut router = Router::new()
///     .get("/", |_, _| async move {
///         Response::builder().body(())
///     });
///
/// let req = Request::get("/").body(()).unwrap();
/// let resp = router.call(req).await.unwrap();
/// assert_eq!(resp.status(), 200);
/// # });
/// ```
#[derive(Default)]
pub struct Router<Body, Data: Clone, Error> {
    inner: Arc<RwLock<MatchRouter<Route<Body, Data, Error>>>>,
    data: Data,
}

impl<Body, Error> Router<Body, (), Error> {
    /// Create a new router that doesn't require any data to be passed to handlers.
    pub fn new() -> Self {
        Self {
            inner: Default::default(),
            data: (),
        }
    }
}

impl<Body, Data, Error> Router<Body, Data, Error>
where
    Body: 'static,
    Data: Clone + 'static,
    Error: 'static,
{
    /// Create a new router that requires data to be passed to handlers.
    ///
    /// # Example
    /// ```
    /// # futures::executor::block_on(async move {
    /// use std::convert::Infallible;
    ///
    /// use http::{Request, Response, StatusCode};
    /// use tower::Service;
    /// use router_service::Router;
    ///
    /// let mut router = Router::with_data(42)
    ///     .get("/", |_, data| async move {
    ///         println!("Got data! {}", data.data);
    ///         Response::builder().body(())
    ///     });
    ///
    /// let req = Request::get("/").body(()).unwrap();
    /// let resp = router.call(req).await.unwrap();
    /// assert_eq!(resp.status(), 200);
    /// # });
    /// ```
    pub fn with_data(data: Data) -> Self {
        Self {
            inner: Default::default(),
            data,
        }
    }

    /// Registers a route requiring the `GET` method.
    pub fn get<HandlerFn, Fut>(self, path: impl AsRef<str>, handler: HandlerFn) -> Self
    where
        HandlerFn: Fn(Request<Body>, RouteContext<Data>) -> Fut,
        HandlerFn: 'static,
        Fut: Future<Output = Result<Response<Body>, Error>> + 'static,
    {
        self.insert_handler(path, Method::GET, handler)
    }

    /// Registers a route requiring the `POST` method.
    pub fn post<HandlerFn, Fut>(self, path: impl AsRef<str>, handler: HandlerFn) -> Self
    where
        HandlerFn: Fn(Request<Body>, RouteContext<Data>) -> Fut,
        HandlerFn: 'static,
        Fut: Future<Output = Result<Response<Body>, Error>> + 'static,
    {
        self.insert_handler(path, Method::POST, handler)
    }

    /// Registers a route requiring the `PUT` method.
    pub fn put<HandlerFn, Fut>(self, path: impl AsRef<str>, handler: HandlerFn) -> Self
    where
        HandlerFn: Fn(Request<Body>, RouteContext<Data>) -> Fut,
        HandlerFn: 'static,
        Fut: Future<Output = Result<Response<Body>, Error>> + 'static,
    {
        self.insert_handler(path, Method::PUT, handler)
    }

    /// Registers a route requiring the `DELETE` method.
    pub fn delete<HandlerFn, Fut>(self, path: impl AsRef<str>, handler: HandlerFn) -> Self
    where
        HandlerFn: Fn(Request<Body>, RouteContext<Data>) -> Fut,
        HandlerFn: 'static,
        Fut: Future<Output = Result<Response<Body>, Error>> + 'static,
    {
        self.insert_handler(path, Method::DELETE, handler)
    }

    /// Registers a route requiring the `HEAD` method.
    pub fn head<HandlerFn, Fut>(self, path: impl AsRef<str>, handler: HandlerFn) -> Self
    where
        HandlerFn: Fn(Request<Body>, RouteContext<Data>) -> Fut,
        HandlerFn: 'static,
        Fut: Future<Output = Result<Response<Body>, Error>> + 'static,
    {
        self.insert_handler(path, Method::HEAD, handler)
    }

    /// Registers a route requiring the `OPTIONS` method.
    pub fn options<HandlerFn, Fut>(self, path: impl AsRef<str>, handler: HandlerFn) -> Self
    where
        HandlerFn: Fn(Request<Body>, RouteContext<Data>) -> Fut,
        HandlerFn: 'static,
        Fut: Future<Output = Result<Response<Body>, Error>> + 'static,
    {
        self.insert_handler(path, Method::DELETE, handler)
    }

    /// Registers a route requiring the `PATCH` method.
    pub fn patch<HandlerFn, Fut>(self, path: impl AsRef<str>, handler: HandlerFn) -> Self
    where
        HandlerFn: Fn(Request<Body>, RouteContext<Data>) -> Fut,
        HandlerFn: 'static,
        Fut: Future<Output = Result<Response<Body>, Error>> + 'static,
    {
        self.insert_handler(path, Method::PATCH, handler)
    }

    /// Registers a route matching any method.
    pub fn any<HandlerFn, Fut>(self, path: impl AsRef<str>, handler: HandlerFn) -> Self
    where
        HandlerFn: Fn(Request<Body>, RouteContext<Data>) -> Fut,
        HandlerFn: 'static,
        Fut: Future<Output = Result<Response<Body>, Error>> + 'static,
    {
        let mut inner = self.inner.write().unwrap();

        if let Ok(existing) = inner.at_mut(path.as_ref()) {
            existing.value.catchall = Some(handler.into());
        } else {
            inner
                .insert(
                    path.as_ref(),
                    Route {
                        handlers: HashMap::new(),
                        catchall: Some(handler.into()),
                    },
                )
                .expect("unable to add route to router");
        }

        drop(inner);

        self
    }

    fn insert_handler<H>(self, path: impl AsRef<str>, method: Method, handler: H) -> Self
    where
        H: Into<AsyncUnsyncHandler<Body, Data, Error>>,
    {
        let mut inner = self.inner.write().unwrap();
        if let Ok(existing) = inner.at_mut(path.as_ref()) {
            existing.value.handlers.insert(method, handler.into());
        } else {
            let mut handlers: HashMap<Method, AsyncUnsyncHandler<Body, Data, Error>> =
                HashMap::new();
            handlers.insert(method, handler.into());

            inner
                .insert(
                    path.as_ref(),
                    Route {
                        handlers,
                        catchall: None,
                    },
                )
                .expect("unable to add route to router");
        }

        drop(inner);

        self
    }
}

impl<Body, Data, Error> Clone for Router<Body, Data, Error>
where
    Data: Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            data: self.data.clone(),
        }
    }
}

/// The context of a matched route.
#[derive(Debug)]
pub struct RouteContext<T> {
    /// Arbitrary data associated with the router that is available to all handlers.
    pub data: T,
    params: HashMap<String, String>,
}

impl<T> RouteContext<T> {
    /// Returns a parameter value from the path by name.
    pub fn param(&self, name: impl AsRef<str>) -> Option<&str> {
        self.params.get(name.as_ref()).map(|s| s.as_str())
    }
}

#[cfg(test)]
mod tests {
    use std::{
        convert::Infallible,
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        },
    };

    use http::{Method, Request, Response};
    use tower::Service;

    use crate::Router;

    #[test]
    fn not_found() {
        futures::executor::block_on(async move {
            let mut router: Router<(), (), Infallible> = Router::new();
            let req = Request::builder()
                .uri("/not-found")
                .method(Method::GET)
                .body(())
                .unwrap();

            let resp = router.call(req).await;
            let resp = resp.unwrap();

            assert_eq!(resp.status(), 404);
        });
    }

    #[test]
    fn receives_data() {
        futures::executor::block_on(async move {
            let data = Arc::new(AtomicBool::new(false));
            let mut router: Router<(), Arc<AtomicBool>, Infallible> =
                Router::with_data(data.clone()).get("/example", |_req, ctx| async move {
                    ctx.data.store(true, Ordering::SeqCst);
                    Ok(Response::builder().body(()).unwrap())
                });

            let req = Request::builder()
                .uri("/example")
                .method(Method::GET)
                .body(())
                .unwrap();

            let resp = router.call(req).await.unwrap();
            assert_eq!(resp.status(), 200);
            assert!(data.load(Ordering::SeqCst));
        });
    }

    #[test]
    fn receives_data_async() {
        futures::executor::block_on(async move {
            let data = Arc::new(AtomicBool::new(false));
            let mut router: Router<_, _, Infallible> =
                Router::with_data(data.clone()).get("/example", |_req, ctx| async move {
                    ctx.data.store(true, Ordering::SeqCst);
                    Ok(Response::builder().body(()).unwrap())
                });

            let req = Request::builder()
                .uri("/example")
                .method(Method::GET)
                .body(())
                .unwrap();

            let resp = router.call(req).await.unwrap();
            assert_eq!(resp.status(), 200);
            assert!(data.load(Ordering::SeqCst));
        });
    }
}
