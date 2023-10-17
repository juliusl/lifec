use hyper::{client::HttpConnector};
use lifec_engine::{operation::Operation, plugin::{CallAsync, ThunkContext}};

use poem::{
    get,
    http::{Extensions, HeaderMap, HeaderName, HeaderValue, StatusCode, Uri, Version},
    web::Data,
    Body, EndpointExt, Response, ResponseParts, Server, async_trait, RequestBuilder,
};
use reality::{borrow_mut, derive::BlockObjectType, ResourceKey, StorageTarget};
use tracing::{error, info};

/// Maps http request into transient storage before executing an engine operation,
///
#[poem::handler]
async fn run_operation(
    request: &poem::Request,
    body: Body,
    operation: Data<&Operation>,
) -> Response {
    let mut op = operation.clone();
    if let Some(op) = op.context_mut() {
        op.reset();
        let transient = op.transient();
        let mut storage = transient.storage.write().await;

        let headers = request.headers().clone();
        storage.put_resource(headers, None);
        let uri = request.uri().clone();
        storage.put_resource(uri, None);
        storage.put_resource(body, None);
        storage.put_resource(
            ResponseParts {
                status: StatusCode::OK,
                version: Version::HTTP_11,
                headers: HeaderMap::new(),
                extensions: Extensions::new(),
            },
            None,
        );
        storage.put_resource(Body::empty(), Some(ResourceKey::with_hash("response")));
        storage.put_resource(request.method().clone(), None);
    }

    if let Ok(op) = op.execute().await {
        let transient = op.transient();
        let mut storage = transient.storage.write().await;

        if let Some(response) = storage.take_resource::<Response>(None) {
            return *response;
        } else if let (Some(parts), Some(body)) = (
            storage.take_resource::<ResponseParts>(None),
            storage.take_resource::<Body>(Some(ResourceKey::with_hash("response"))),
        ) {
            return Response::from_parts(*parts, *body);
        }
    }

    Response::builder().status(StatusCode::BAD_REQUEST).finish()
}

/// Host an engine as an http server,
///
pub async fn host_engine<L: poem::listener::Listener + 'static, const UUID: u128>(
    listener: L,
    engine: lifec_engine::engine::Engine<UUID>,
) {
    let engine = engine.compile().await;

    let routes = engine
        .iter_operations()
        .fold(poem::Route::new(), |routes, (address, op)| {
            info!("Setting route {address}");
            routes.at(
                address,
                get(run_operation.data(op.clone())).post(run_operation.data(op.clone())),
            )
        });

    // -- TODO: Engine server protocol -- can have a "list_operations"
    // -- Can also parse comments as documentation
    // Then, can have something like this:
    // + .operation
    // <application/lifec.engine.server> localhost:7575
    // <..connect>
    // : .run ''
    // : .run ''
    // let routes = Route::new().nest("/operation", operations);

    let cancel_engine = engine.cancellation.clone();
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;

        info!("Received cancel signal, closing server");
        cancel_engine.cancel();
    });

    let server = Server::new(listener);
    server
        .run_with_graceful_shutdown(routes, engine.cancellation.cancelled(), None)
        .await
        .unwrap();
}

/// Provides helper functions for accessing poem request resources,
///
pub trait PoemExt {
    /// Take the request body from storage,
    ///
    fn take_body(&mut self) -> Option<poem::Body>;

    /// Set the status code on the response,
    ///
    fn set_status_code(&mut self, code: StatusCode);

    /// Sets a header on the response,
    ///
    fn set_header(
        &mut self,
        header: impl Into<HeaderName> + Send + Sync + 'static,
        value: impl Into<HeaderValue> + Send + Sync + 'static,
    );

    /// Sets the body on the response,
    ///
    fn set_response_body(&mut self, body: Body);
}

impl PoemExt for lifec_engine::plugin::ThunkContext {
    fn take_body(&mut self) -> Option<poem::Body> {
        let transient = self.transient();
        transient
            .storage
            .try_write()
            .ok()
            .and_then(|mut s| s.take_resource::<Body>(None).map(|b| *b))
    }

    fn set_status_code(&mut self, code: StatusCode) {
        let transient = self.transient().storage;
        let transient = transient.try_write();

        if let Ok(mut transient) = transient {
            use std::ops::DerefMut;

            borrow_mut!(transient, ResponseParts, |parts| => {
                parts.status = code;
            });
        } else {
            error!("Could not write to transient storage. Existing read-lock.");
        }
    }

    fn set_header(
        &mut self,
        header: impl Into<HeaderName> + Send + Sync + 'static,
        value: impl Into<HeaderValue> + Send + Sync + 'static,
    ) {
        let transient = self.transient().storage;
        let transient = transient.try_write();

        if let Ok(mut transient) = transient {
            use std::ops::DerefMut;

            borrow_mut!(transient, ResponseParts, |parts| => {
                parts.headers.insert(header.into(), value.into());
            });
        } else {
            error!("Could not write to transient storage. Existing read-lock.");
        }
    }

    fn set_response_body(&mut self, body: Body) {
        let transient = self.transient().storage;
        let transient = transient.try_write();

        if let Ok(mut transient) = transient {
            transient.put_resource(body, Some(ResourceKey::with_hash("response")))
        } else {
            error!("Could not write to transient storage. Existing read-lock.");
        }
    }
}

use lifec_engine::prelude::*;

pub type SecureClient = hyper::Client<hyper_tls::HttpsConnector<HttpConnector>>;

/// Extensions for working w/ a hyper client,
/// 
#[async_trait]
pub trait HyperExt {
    /// Makes an https request and returns the response,
    /// 
    async fn request(&mut self, request: hyper::Request<hyper::Body>) -> anyhow::Result<hyper::Response<hyper::Body>>;
}

#[async_trait]
impl HyperExt for ThunkContext {
    async fn request(&mut self, request: hyper::Request<hyper::Body>) -> anyhow::Result<hyper::Response<hyper::Body>> {
        let source = self.source().await;

        let client = source.resource::<SecureClient>(None);
        if let Some(client) = client {
            let response  = client.request(request).await?;
            Ok(response)
        } else {
            Err(anyhow::anyhow!("Secure http client is not enabled"))
        }
    }
}

#[derive(Default, Clone, BlockObjectType)]
pub struct RemotePlugin {
    address: String,
    host: poem::http::Uri,
}

impl std::str::FromStr for RemotePlugin {
    type Err = anyhow::Error;

    fn from_str(_: &str) -> Result<Self, Self::Err> {
        Ok(RemotePlugin {
            address: String::new(),
            host: Uri::from_static("localhost:7575"),
        })
    }
}

#[async_trait]
impl CallAsync for RemotePlugin {
    async fn call(context: &mut ThunkContext) -> anyhow::Result<()> {
        let init = context.initialized::<RemotePlugin>().await;

        let request = hyper::Request::get(init.host).body(hyper::Body::empty())?;
        let response = context.request(request).await?;

        let (parts, body) = response.into_parts();

        context.set_status_code(parts.status);
        context.set_response_body(body.into());

        Ok(())
    }
}

#[allow(unused_imports)]
mod tests {
    use super::PoemExt;
    use std::convert::Infallible;
    use std::str::FromStr;

    use lifec_engine::plugin::{CallAsync, ThunkContext};
    use lifec_engine::{engine, prelude::*};
    use poem::http::{HeaderMap, HeaderName, StatusCode};
    use poem::{async_trait, Body};

    use crate::host_engine;

    #[derive(Debug, Clone, Default, BlockObjectType)]
    #[reality(rename = "app/test")]
    pub struct Test {
        name: String,
    }

    impl FromStr for Test {
        type Err = Infallible;

        fn from_str(_: &str) -> Result<Self, Self::Err> {
            Ok(Self {
                name: String::new(),
            })
        }
    }

    #[async_trait]
    impl CallAsync for Test {
        async fn call(context: &mut ThunkContext) -> anyhow::Result<()> {
            let init = context.initialized::<Test>().await;

            let transient = context.transient();

            if let Some(body) = context.take_body() {
                let body = body.into_vec().await.unwrap();
                println!("{:?}", String::from_utf8(body).unwrap());
            }

            let transient = transient.storage.read().await;

            if let Some(map) = transient.resource::<HeaderMap>(None) {
                println!("{:?}", map);
            }

            // Test logging an error
            context.set_header(
                poem::http::HeaderName::from_static("x-ms-test-header"),
                poem::http::HeaderValue::from_static("test"),
            );
            drop(transient);

            context.set_header(
                poem::http::HeaderName::from_static("x-ms-test-header"),
                poem::http::HeaderValue::from_static("test"),
            );
            context.set_status_code(StatusCode::ACCEPTED);
            context.set_response_body(Body::from_string("hello-world".to_string()));

            println!("{:?}", init);
            Ok(())
        }
    }

    #[ignore = "This test is to test an engine server locally, requires Ctrl+C to shutdown"]
    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_engine_server() {
        let mut builder = engine::Engine::builder();
        builder.register::<Test>();

        let mut engine = builder.build_primary();
        engine
            .load_source(
                r#"
```runmd
+ .operation test/operation
<app/test> cargo
: .name hello-world-2
```
"#,
            )
            .await;

        host_engine(poem::listener::TcpListener::bind("localhost:7575"), engine).await;

        // Tests graceful shutdown
        assert!(logs_contain("Received cancel signal, closing server"));
        ()
    }
}
