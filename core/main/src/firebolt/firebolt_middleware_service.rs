use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use futures::future::BoxFuture;
use futures::FutureExt;
use http::request;
use http_body::Frame;
use jsonrpsee::core::client::ClientT;
use jsonrpsee::rpc_params;
use jsonrpsee::tokio::sync::mpsc;
use ripple_sdk::api::gateway::rpc_gateway_api::{ApiMessage, ClientContext};
use ripple_sdk::uuid::Uuid;

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
use jsonrpsee_core::BoxError;
use jsonrpsee_types::{ErrorCode, ErrorObject};
use ripple_sdk::log::{error, info};

use tower::Layer;

#[derive(Debug, Clone)]
pub struct FireboltSessionValidator<S> {
    inner: S,
    platform_state: PlatformState,
    default_app_id: Option<String>,
    internal_app_id: Option<String>,
    secure: bool,
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
    Secure,
    AppProvided(String),
    InternalProvided(String),
}
impl std::fmt::Display for FireboltAppId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FireboltAppId::Secure => write!(f, "secure"),
            FireboltAppId::AppProvided(app_id) => write!(f, "{}", app_id),
            FireboltAppId::InternalProvided(app_id) => write!(f, "{}", app_id),
        }
    }
}

enum FireboltGatewayError {
    QueryParamMissing(String),
    NoSessionFound,
}
#[derive(Debug, Clone)]
pub struct FireboltSession {
    pub session_id: FireboltSessionId,
    pub app_id: FireboltAppId,
    pub secure: bool,
}
impl From<&FireboltSession> for ClientContext {
    fn from(firebolt_session: &FireboltSession) -> Self {
        ClientContext {
            session_id: firebolt_session.session_id.to_string(),
            app_id: firebolt_session.app_id.to_string(),
            gateway_secure: firebolt_session.secure,
        }
    }
}
impl FireboltSession {
    pub fn new(session_id: FireboltSessionId, app_id: FireboltAppId, secure: bool) -> Self {
        FireboltSession {
            session_id: session_id,
            app_id: app_id,
            secure: secure,
        }
    }
}

fn get_query_param<B>(
    query_string: &str,
    key: &str,
    required: bool,
) -> Result<Option<String>, FireboltGatewayError> {
    let query_item = querystring::querify(query_string)
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| v.to_string());
    if required && query_item.is_none() {
        let err_msg = format!("{} query parameter missing", key);
        error!("{}", err_msg);
        return Err(FireboltGatewayError::QueryParamMissing(err_msg));
    } else {
        return Ok(query_item);
    }
}
fn get_app_id<B>(
    secure: bool,
    query_string: &str,
    internal_app_id: Option<String>,
) -> Result<FireboltAppId, FireboltGatewayError> {
    match secure {
        true => Ok(FireboltAppId::Secure),
        false => match get_query_param::<B>(query_string, "appId", false)? {
            Some(a) => Ok(FireboltAppId::AppProvided(a)),
            None => match internal_app_id {
                Some(a) => Ok(FireboltAppId::InternalProvided(a)),
                None => Ok(FireboltAppId::Secure),
            },
        },
    }
}

fn get_firebolt_session<B>(
    secure: bool,
    query_string: &str,
    internal_app_id: Option<String>,
    platform_state: &PlatformState,
) -> Result<FireboltSession, FireboltGatewayError> {
    let app_id = get_app_id::<B>(secure, query_string, internal_app_id)?;
    let session_id = match app_id.clone() {
        FireboltAppId::Secure => match get_query_param::<B>(query_string, "session", false)? {
            Some(a) => FireboltSessionId::AppProvided(a),
            None => {
                return Err(FireboltGatewayError::QueryParamMissing(
                    "session".to_string(),
                ))
            }
        },
        FireboltAppId::AppProvided(provided) | FireboltAppId::InternalProvided(provided) => {
            match get_query_param::<B>(query_string, "session", true)? {
                Some(a) => FireboltSessionId::AppProvided(provided),
                None => {
                    return Err(FireboltGatewayError::QueryParamMissing(
                        "session".to_string(),
                    ))
                }
            }
        }
    };
    match app_id.clone() {
        FireboltAppId::Secure => {
            //talk to appmanager to get the session
            match platform_state
                .app_manager_state
                .get_app_id_from_session_id(&session_id.to_string())
            {
                Some(retrieved_app_id) => Ok(FireboltSession::new(
                    session_id,
                    FireboltAppId::AppProvided(retrieved_app_id),
                    secure,
                )),
                None => Err(FireboltGatewayError::NoSessionFound),
            }
        }
        FireboltAppId::AppProvided(provided) | FireboltAppId::InternalProvided(provided) => Ok(
            FireboltSession::new(FireboltSessionId::AppProvided(provided), app_id, secure),
        ),
    }
}

impl<S, B> tower::Service<HttpRequest<B>> for FireboltSessionValidator<S>
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
        let platform_state = self.platform_state.clone();
        let session = get_firebolt_session::<B>(
            self.secure,
            req.uri().query().unwrap_or_default(),
            self.internal_app_id.clone(),
            &platform_state,
        );

        let mut req = req.map(jsonrpsee::server::HttpBody::new);
        match session {
            Ok(firebolt_session) => {
                req.extensions_mut().insert(firebolt_session);
                Box::pin(self.inner.call(req).map_err(Into::into))
            }
            Err(e) => match e {
                FireboltGatewayError::QueryParamMissing(msg) => {
                    Box::pin(self.inner.call(req).map_err(Into::into))
                }
                FireboltGatewayError::NoSessionFound => {
                    Box::pin(self.inner.call(req).map_err(Into::into))
                }
            },
        }
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
    type Service = FireboltSessionValidator<S>;

    fn layer(&self, inner: S) -> Self::Service {
        FireboltSessionValidator {
            inner: inner,
            platform_state: self.platform_state.clone(),
            default_app_id: self.internal_app_id.clone(),
            secure: self.secure,
            internal_app_id: self.internal_app_id.clone(),
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
pub struct FireboltAuthLayer<S> {
    service: S,
    platform_state: PlatformState,
}

impl<'a, S> RpcServiceT<'a> for FireboltAuthLayer<S>
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
                        Some(_) => service.call(req.clone()).await,
                        None => {
                            error!(
                                "no app id found in session manager for session id: {}",
                                firebolt_session.session_id
                            );
                            MethodResponse::error(
                                jsonrpsee_types::Id::Number(403),
                                ErrorObject::owned(
                                    ErrorCode::InternalError.code(),
                                    "invalid session",
                                    None::<()>,
                                ),
                            )
                        }
                    }
                }
                None => MethodResponse::error(
                    jsonrpsee_types::Id::Number(403),
                    ErrorObject::owned(ErrorCode::InternalError.code(), "no session", None::<()>),
                ),
            }
        }
        .boxed()
    }
}

pub struct FireboltDispatchLayer<S> {
    service: S,
    platform_state: PlatformState,
}
impl<'a, S> RpcServiceT<'a> for FireboltDispatchLayer<S>
where
    S: RpcServiceT<'a> + Send + Sync + Clone + 'static,
{
    type Future = BoxFuture<'a, MethodResponse>;

    fn call(&self, req: Request<'a>) -> Self::Future {
        let service = self.service.clone();
        let platform_state = self.platform_state.clone();
        let ripple_client = platform_state.get_client();

        async move {
            match req.extensions.get::<FireboltSession>() {
                Some(firebolt_session) => {
                    let client_context: ClientContext = firebolt_session.into();

                    let (session_tx, mut session_rx) = mpsc::channel::<ApiMessage>(32);
                    let app_id = client_context.app_id.clone();
                    let session_id = client_context.session_id.clone();
                    let gateway_secure = firebolt_session.secure;
                    let session = Session::new(
                        client_context.app_id.clone(),
                        Some(session_tx.clone()),
                        ripple_sdk::api::apps::EffectiveTransport::Websocket,
                    );

                    let app_id_c = app_id.clone();
                    let session_id_c = session_id.clone();

                    let connection_id = Uuid::new_v4().to_string();
                    info!(
                        "Creating new connection_id={} app_id={} session_id={}, gateway_secure={}",
                        connection_id, app_id_c, session_id_c, gateway_secure
                    );

                    let connection_id_c = connection_id.clone();

                    let msg = FireboltGatewayCommand::RegisterSession {
                        session_id: connection_id.clone(),
                        session,
                    };
                    if let Err(e) = ripple_client.send_gateway_command(msg) {
                        error!("Error registering the connection {:?}", e);
                        return MethodResponse::error(
                            jsonrpsee_types::Id::Number(403),
                            ErrorObject::owned(
                                ErrorCode::InternalError.code(),
                                "no session",
                                None::<()>,
                            ),
                        );
                    }
                    if !gateway_secure
                        && PermissionHandler::fetch_and_store(&platform_state, &app_id, false)
                            .await
                            .is_err()
                    {
                        error!("Couldnt pre cache permissions");
                    }

                    // let (mut sender, mut receiver) = ws_stream.split();

                    service.call(req.clone()).await
                }
                None => MethodResponse::error(
                    jsonrpsee_types::Id::Number(403),
                    ErrorObject::owned(ErrorCode::InternalError.code(), "no session", None::<()>),
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
    let http_middleware =
        tower::ServiceBuilder::new().layer(FireboltSessionAuthenticatorLayer::new(
            internal_app_id.clone(),
            secure,
            platform_state.clone(),
        ));

    let rpc_middleware = RpcServiceBuilder::new().layer_fn(move |service| FireboltAuthLayer {
        service,
        platform_state: platform_state.clone(),
    });

    let server = jsonrpsee::server::Server::builder()
        .set_http_middleware(http_middleware)
        .set_rpc_middleware(rpc_middleware)
        .build(server_addr)
        .await?;
    /*
    TODO: bolt up to registrar
    */
    let mut module = RpcModule::new(());

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
    info!(
        "Listening on: {} secure={} with internal_app_id={:?}",
        server_addr, secure, internal_app_id
    );
    ripple_sdk::tokio::spawn(handle.stopped());

    Ok(addr)
}
