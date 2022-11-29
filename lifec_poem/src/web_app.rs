use std::time::Duration;

use lifec::prelude::*;

use poem::{
    listener::{Acceptor, Listener, RustlsCertificate, RustlsConfig, TcpListener},
    Route, Server,
};
use tokio::sync::oneshot::Receiver;

/// Trait that enables a web app to be hosted via AppHost plugin,
///
pub trait WebApp {
    /// update context and returns a new instance of self
    fn create(context: &mut ThunkContext) -> Self;

    /// update self an returns routes for the host
    fn routes(&mut self) -> Route;
}

/// Wrapper struct over a function to create a new web app,
///
#[derive(Component)]
#[storage(DenseVecStorage)]
pub struct AppHost<A>(Option<A>)
where
    A: WebApp + Send + Sync + 'static;

impl<A> AppHost<A>
where
    A: WebApp + Send + Sync + 'static,
{
    /// Starts the server,
    ///
    pub async fn start_server<L, B>(
        &mut self,
        tc: &mut ThunkContext,
        cancel_source: Receiver<()>,
        server: Server<L, B>,
    ) where
        L: Listener + 'static,
        B: Acceptor + 'static,
    {
        if let AppHost(Some(app)) = self {
            let server = server.run_with_graceful_shutdown(
                app.routes(),
                async {
                    match cancel_source.await {
                        Ok(_) => tc.status("Cancelling server").await,
                        Err(err) => {
                            tc.status(format!("Error cancelling server, {err}")).await;
                        }
                    }
                },
                tc.state()
                    .find_int("shutdown_timeout_ms")
                    .and_then(|f| Some(Duration::from_millis(f as u64))),
            );

            match server.await {
                Ok(_) => {
                    tc.status("Server is exiting").await;
                }
                Err(err) => {
                    event!(Level::ERROR, "Server host error, {err}");

                    tc.status(format!("Server error exit {err}")).await;
                    tc.error(|e| {
                        e.with_text("err", format!("app host error: {err}"));
                    });
                }
            }
        }
    }
}

impl<A> Plugin for AppHost<A>
where
    A: WebApp + Send + Sync,
{
    fn symbol() -> &'static str {
        "app_host"
    }

    fn description() -> &'static str {
        r#"
    Creates an app host with `address`, w/ routes provided by some type `A` which implements WebApp.
    If a `tls_key` and `tld_crt` are loaded, the app will start with tls enabled.
    "#
    }

    fn call(context: &mut ThunkContext) -> Option<AsyncContext> {
        context.clone().task(|cancel_source| {
            let mut tc = context.clone();
            async {
                if let Some(address) = tc.state().find_symbol("app_host") {
                    eprintln!("Starting {address}");

                    let mut app_host = AppHost(Some(A::create(&mut tc)));

                    let mut tcp_conn = Some(TcpListener::bind(address));
                    let mut tls_tcp_conn = None;

                    // Enable TLS
                    if let (Some(key), Some(cert)) = (
                        tc.state().find_binary("tls_key"),
                        tc.state().find_binary("tls_crt"),
                    ) {
                        if let Some(conn) = tcp_conn.take() {
                            tls_tcp_conn = Some(
                                conn.rustls(
                                    RustlsConfig::new()
                                        .fallback(RustlsCertificate::new().key(key).cert(cert)),
                                ),
                            );
                        }
                    }

                    if let Some(tls_conn) = tls_tcp_conn {
                        let server = Server::new(tls_conn);
                        app_host.start_server(&mut tc, cancel_source, server).await;
                    } else if let Some(tcp_conn) = tcp_conn {
                        let server = Server::new(tcp_conn);
                        app_host.start_server(&mut tc, cancel_source, server).await;
                    }
                }

                Some(tc)
            }
        })
    }
}

impl<A> BlockObject for AppHost<A>
where
    A: WebApp + Send + Sync,
{
    fn query(&self) -> BlockProperties {
        BlockProperties::default()
            .require("app_host")
            .optional("shutdown_timeout_ms")
            .optional("tls_key")
            .optional("tls_crt")
    }

    fn parser(&self) -> Option<CustomAttribute> {
        Some(Self::as_custom_attr())
    }
}

impl<A> Default for AppHost<A>
where
    A: WebApp + Send + Sync,
{
    fn default() -> Self {
        Self(None)
    }
}
