use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use futures::future::BoxFuture;
use futures::FutureExt;
use http_body::Frame;
use jsonrpsee::core::client::ClientT;
use jsonrpsee::rpc_params;

use super::firebolt_gateway::FireboltGatewayCommand;
use super::firebolt_ws::ClientIdentity;
use crate::firebolt::handlers::provider_registrar::ProviderRegistrar;
use crate::state::platform_state;
use crate::{
    service::apps::delegated_launcher_handler::AppManagerState,
    state::{
        cap::permitted_state::PermissionHandler, platform_state::PlatformState,
        session_state::Session,
    },
};
use futures_util::{Future, TryFutureExt};
use jsonrpsee::server::middleware::rpc::{RpcServiceBuilder, RpcServiceT};
use jsonrpsee::server::{HttpRequest, MethodResponse, RpcModule, Server, TowerServiceBuilder};
use jsonrpsee::types::Request;
use jsonrpsee::ws_client::WsClientBuilder;
use jsonrpsee_core::BoxError;
use jsonrpsee_types::{ErrorCode, ErrorObject};
use ripple_sdk::log::{error, info};
use ripple_sdk::utils::error;
use ripple_sdk::uuid::timestamp::UUID_TICKS_BETWEEN_EPOCHS;
use ripple_sdk::uuid::Uuid;
use serde_json::to_writer_pretty;
use tower::Layer;

fn get_query_param<B>(query_string: &str, key: &str) -> Option<String> {
    querystring::querify(query_string)
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| v.to_string())
}
#[derive(Debug, Clone)]
pub struct FireboltSessionAuthenticator<S> {
    inner: S,
    platform_state: PlatformState,
    default_app_id: Option<String>,
}
#[derive(Debug, Clone)]
enum FireboltSessionId {
    AppProvided(String),
    Generated(String),
}
impl std::fmt::Display for FireboltSessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FireboltSessionId::AppProvided(session_id) => write!(f, "{}", session_id),
            FireboltSessionId::Generated(session_id) => write!(f, "{}", session_id),
        }
    }
}
#[derive(Debug, Clone)]
enum FireboltAppId {
    AppProvided(String),
    SessionProvided(String),
    InternalProvided(String),
}

#[derive(Debug, Clone)]
pub struct FireboltSession {
    pub session_id: FireboltSessionId,
    pub app_id: FireboltAppId,
}
impl FireboltSession {
    pub fn new(session_id: FireboltSessionId, app_id: FireboltAppId) -> Self {
        FireboltSession {
            session_id: session_id,
            app_id: app_id,
        }
    }
}
impl<S, B> tower::Service<HttpRequest<B>> for FireboltSessionAuthenticator<S>
where
    S: tower::Service<HttpRequest, Response = jsonrpsee::server::HttpResponse>,
    S::Response: 'static,
    S::Error: Into<tower::BoxError> + 'static,
    S::Future: Send + 'static,
    B: http_body::Body<Data = hyper::body::Bytes> + Send + 'static,
    B::Data: Send,
    B::Error: Into<tower::BoxError>,
{
    type Response = S::Response;
    type Error = BoxError;
    type Future = std::pin::Pin<
        Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, req: HttpRequest<B>) -> Self::Future {
        let firebolt_session_id = match req.uri().query() {
            Some(q) => match get_query_param::<B>(q, "session_id") {
                Some(session_id) => FireboltSessionId::AppProvided(session_id),
                None => FireboltSessionId::Generated(Uuid::new_v4().to_string()),
            },
            None => FireboltSessionId::Generated(Uuid::new_v4().to_string()),
        };
        let firebolt_app_id = match req.uri().query() {
            Some(q) => match get_query_param::<B>(q, "appId") {
                Some(app_id) => FireboltAppId::AppProvided(app_id),
                None => FireboltAppId::InternalProvided("internal".to_string()),
            },
            None => FireboltAppId::InternalProvided("internal".to_string()),
        };

        let firebolt_session = FireboltSession::new(firebolt_session_id, firebolt_app_id);

        let mut req = req.map(jsonrpsee::server::HttpBody::new);

        req.extensions_mut().insert(firebolt_session);
        //self.inner.call(req)
        Box::pin(self.inner.call(req).map_err(Into::into))
    }
}

#[derive(Debug, Clone)]
pub struct FireboltSessionAuthenticatorLayer {
    internal_app_id: Option<String>,
    secure: bool,
    platform_state: PlatformState,
}
impl FireboltSessionAuthenticatorLayer {
    pub fn new(
        internal_app_id: Option<String>,
        secure: bool,
        platform_state: PlatformState,
    ) -> Self {
        FireboltSessionAuthenticatorLayer {
            internal_app_id: internal_app_id,
            secure: secure,
            platform_state: platform_state,
        }
    }
}

impl<S> Layer<S> for FireboltSessionAuthenticatorLayer {
    type Service = FireboltSessionAuthenticator<S>;

    fn layer(&self, inner: S) -> Self::Service {
        FireboltSessionAuthenticator {
            inner: inner,
            platform_state: self.platform_state.clone(),
            default_app_id: self.internal_app_id.clone(),
        }
    }
}

/*ent::ClientT;
use jsonrpsee_types::rpc
 1)
 if secure==true, then app_id is not required
 if secure==false, then app_id is required - get it from query or internal_app_id
 2) if session is not given, then error
 3) if app_id is not given, then get it from session
 4) if app_id is given, then use it
 5) if app_id is not given and session is not found, then error
 6) use the session id and app id to create a client identity
 7) send the client identity to the next step
*/

#[derive(Clone)]
pub struct Logger<S>(S);

impl<'a, S> RpcServiceT<'a> for Logger<S>
where
    S: RpcServiceT<'a> + Send + Sync,
{
    type Future = S::Future;

    fn call(&self, req: Request<'a>) -> Self::Future {
        println!("logger middleware: method `{:?}`", req);
        self.0.call(req)
    }
}

#[derive(Clone)]
pub struct RippleGatewayLayer<S> {
    service: S,
    platform_state: PlatformState,
}

impl<'a, S> RpcServiceT<'a> for RippleGatewayLayer<S>
where
    S: RpcServiceT<'a> + Send + Sync + Clone + 'static,
{
    type Future = BoxFuture<'a, MethodResponse>;

    fn call(&self, req: Request<'a>) -> Self::Future {
        println!("logger middleware: method `{:?}`", req);
        let service = self.service.clone();
        let platform_state = self.platform_state.clone();

        async move {
            match req.extensions.get::<FireboltSession>() {
                Some(firebolt_session) => {
                    match platform_state
                        .app_manager_state
                        .get_app_id_from_session_id(&firebolt_session.session_id.to_string())
                    {
                        Some(app_id) => {
                            let response = service.call(req.clone()).await;
                            service.call(req).await;
                            response
                        }
                        None => {
                            error!(
                                "no app id found in session manager for session id: {}",
                                firebolt_session.session_id
                            );
                            MethodResponse::error(
                                jsonrpsee_types::Id::Number(403),
                                ErrorObject::owned(
                                    ErrorCode::InternalError.code(),
                                    "no session found in app manager",
                                    None::<()>,
                                ),
                            )
                        }
                    }
                }
                None => MethodResponse::error(
                    jsonrpsee_types::Id::Number(403),
                    ErrorObject::owned(
                        ErrorCode::InternalError.code(),
                        "no session found in extensions",
                        None::<()>,
                    ),
                ),
            }
        }
        .boxed()
    }
}

pub async fn start(
    server_addr: &str,
    platform_state: PlatformState,
    secure: bool,
    internal_app_id: Option<String>,
) -> anyhow::Result<SocketAddr> {
    let http_middleware = tower::ServiceBuilder::new().layer(
        FireboltSessionAuthenticatorLayer::new(internal_app_id, secure, platform_state.clone()),
    );

    let rpc_middleware = RpcServiceBuilder::new().layer_fn(move |service| RippleGatewayLayer {
        service,
        platform_state: platform_state.clone(),
    });
    // let http_middleware = HttpSe QueryExtractorLayer::new();
    // let server = Server::builder()
    // .set_http_middleware(http_middleware)
    // .set_rpc_middleware(rpc_middleware).build(server_addr).await?;
    let server = jsonrpsee::server::Server::builder()
        .set_http_middleware(http_middleware)
        .set_rpc_middleware(rpc_middleware)
        .build(server_addr)
        .await?;
    let mut module = RpcModule::new(());
    //ProviderRegistrar::register_method(platform_state, methods)

    /*
    need to:
    register methods
    FireboltWs::start
    FireboltWs::handle_connection
        */
    //module.register_method("firebolt_gateway", FireboltGatewayCommand::new(state.clone()));

    let addr = server.local_addr()?;

    let handle = server.start(module);

    // In this example we don't care about doing shutdown so let's it run forever.
    // You may use the `ServerHandle` to shut it down or manage it yourself.
    info!("Listening on: {} secure={}", server_addr, secure);
    ripple_sdk::tokio::spawn(handle.stopped());

    Ok(addr)
}
