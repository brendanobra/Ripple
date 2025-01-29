use std::net::SocketAddr;

use futures::future::BoxFuture;
use futures::FutureExt;
use jsonrpsee::tokio::sync::{mpsc, oneshot};
use jsonrpsee::{Methods, ResponsePayload};
use ripple_sdk::api::gateway::rpc_gateway_api::{
    ApiMessage, CallContext, ClientContext, RpcRequest,
};
use ripple_sdk::utils::channel_utils::oneshot_send_and_log;
use ripple_sdk::uuid::Uuid;
use serde_json::Value;

use super::firebolt_gateway::FireboltGatewayCommand;
use super::firebolt_ws::ClientIdentity;
use crate::firebolt::firebolt_gatekeeper::FireboltGatekeeper;
use crate::firebolt::firebolt_ws::ConnectionCallbackConfig;
use crate::firebolt::handlers::account_rpc::{AccountImpl, AccountServer};
use crate::firebolt::handlers::audio_description_rpc::AudioDescriptionRPCProvider;
use crate::firebolt::handlers::device_rpc::DeviceRPCProvider;
use crate::firebolt::handlers::provider_registrar::ProviderRegistrar;
use crate::state::platform_state;
use crate::utils::rpc_utils::get_base_method;
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
use jsonrpsee_core::{BoxError, JsonRawValue};
use jsonrpsee_types::{ErrorCode, ErrorObject, Id, Params, RequestSer};
use ripple_sdk::log::{error, info};

use tower_layer::Layer;

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
    /*
    if secure = false, app_id = None
    if secure = true:
       get from query string
       if not in query string, use internal_app_id

    */
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
fn get_firebolt_session_unauthenticated<B>(
    query_string: &str,
    internal_app_id: Option<String>,
    platform_state: &PlatformState,
) -> Result<FireboltSession, FireboltGatewayError> {
    let app_id = get_app_id::<B>(false, query_string, internal_app_id)?;
    Ok(FireboltSession::new(
        FireboltSessionId::Generated(Uuid::new_v4().to_string()),
        app_id,
        false,
    ))
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
        FireboltAppId::AppProvided(_) | FireboltAppId::InternalProvided(_) => {
            match get_query_param::<B>(query_string, "session", true)? {
                Some(session) => FireboltSessionId::AppProvided(session),
                None => FireboltSessionId::Generated(Uuid::new_v4().to_string()),
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
        FireboltAppId::AppProvided(_) | FireboltAppId::InternalProvided(_) => {
            Ok(FireboltSession::new(session_id, app_id, secure))
        }
    }
}

impl<S, B> tower_service::Service<HttpRequest<B>> for FireboltSessionValidator<S>
where
    S: tower_service::Service<HttpRequest, Response = jsonrpsee::server::HttpResponse>,
    S::Response: 'static,
    S::Error: Into<jsonrpsee_core::BoxError> + 'static,
    S::Future: Send + 'static,
    B: http_body::Body<Data = hyper::body::Bytes> + Send + 'static,
    B::Data: Send,
    B::Error: Into<jsonrpsee_core::BoxError>,
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
        let session = if self.secure {
            get_firebolt_session::<B>(
                self.secure,
                req.uri().query().unwrap_or_default(),
                self.internal_app_id.clone(),
                &platform_state,
            )
        } else {
            get_firebolt_session_unauthenticated::<B>(
                req.uri().query().unwrap_or_default(),
                self.internal_app_id.clone(),
                &platform_state,
            )
        };

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
fn enrich_request_params(params: &Params, call_context: &CallContext) -> Box<JsonRawValue> {
    let params: serde_json::Map<String, Value> = params.parse().unwrap();

    let ctx: Value = serde_json::to_value(&call_context.clone()).unwrap();
    let p = serde_json::json!({
        "ctx": ctx,
        "params": params,
        "request" : params,
    });

    println!("sending {} to next layer", p);
    serde_json::value::to_raw_value(&p).unwrap()
}
/*
UnauthenticatedFireboltLayer:
Tower layer to handle unauthenticated requests , using default_app_id (which is provided by configuration)
*/
pub struct UnauthenticatedFireboltLayer<S> {
    service: S,
    platform_state: PlatformState,
    default_app_id: Option<String>,
}
impl<S> UnauthenticatedFireboltLayer<S> {
    pub fn get_method_response(method_name: &str, methods: Methods) -> MethodResponse {
        MethodResponse::error(
            jsonrpsee_types::Id::Number(403),
            ErrorObject::owned(
                ErrorCode::InternalError.code(),
                format!("method {} not found", method_name),
                None::<()>,
            ),
        )
    }
}

impl<'a, S> RpcServiceT<'a> for UnauthenticatedFireboltLayer<S>
where
    S: RpcServiceT<'a> + Send + Sync + Clone + 'static,
{
    type Future = BoxFuture<'a, MethodResponse>;

    fn call(&self, request: Request<'a>) -> Self::Future {
        let service = self.service.clone();
        let platform_state = self.platform_state.clone();
        let default_app_id = self.default_app_id.clone();
        println!("UnauthenticatedFireboltLayer: method `{:?}`", request);

        async move {
            let ripple_client = platform_state.get_client();
            let app_state = platform_state.app_manager_state.clone();
            let (connect_tx, connect_rx) = oneshot::channel::<ClientIdentity>();
            let cfg = ConnectionCallbackConfig {
                next: connect_tx,
                app_state: app_state.clone(),
                secure: false,
                internal_app_id: default_app_id.clone(),
            };
            let (session_tx, mut session_rx) = mpsc::channel::<ApiMessage>(32);

            /*
            need to:
            1) determine if rule or not
            2) if rule, then call the broker
            3) if not rule, then "let" the regular rpc method call happen
            */
            match request.extensions.get::<FireboltSession>() {
                Some(firebolt_session) => {
                    println!("session found: {:?}", firebolt_session);
                    let method_name = request.method_name();
                    let app_id = firebolt_session.app_id.clone().to_string();
                    let session_id = firebolt_session.session_id.clone().to_string();
                    let request_id = Uuid::new_v4().to_string();
                    let connection_id = Uuid::new_v4().to_string();
                    let cid = ClientIdentity {
                        session_id: session_id.clone(),
                        app_id: app_id.clone(),
                    };
                    oneshot_send_and_log(cfg.next, cid, "ResolveClientIdentity");
                    let session = Session::new(
                        app_id.clone(),
                        Some(session_tx.clone()),
                        ripple_sdk::api::apps::EffectiveTransport::Websocket,
                    );
                    let request_downstream = request.clone();

                    let json = serde_json::to_string(&RequestSer::owned(
                        request_downstream.id,
                        request_downstream.method,
                        request_downstream.params.map(|p| p.into_owned()),
                    ))
                    .unwrap();

                    let rpc_request = RpcRequest::parse(
                        json,
                        app_id.clone(),
                        session_id.clone(),
                        request_id.clone(),
                        Some(connection_id.clone()),
                        false,
                    )
                    .unwrap();

                    if !platform_state.rule_engine().has_rule_with_name(method_name) {
                        /*defer to next layer/auto impls */
                        let mut legacy_request = request.clone();
                        //legacy_request.method = std::borrow::Cow::Owned(method_name.to_lowercase());

                        // let params: serde_json::Map<String, Value> =
                        //     request.params().parse().unwrap();
                        // let ctx: Value = serde_json::to_value(&rpc_request.ctx.clone()).unwrap();
                        // let p = serde_json::json!({
                        //     "ctx": ctx,
                        //     "params": params,
                        //     "request" : params,
                        // });

                        // println!("sending {} to next layer", p);
                        // let p = serde_json::value::to_raw_value(&p).unwrap();
                        legacy_request.params = Some(std::borrow::Cow::Owned(
                            enrich_request_params(&legacy_request.params(), &rpc_request.ctx),
                        ));
                        //return Self::get_method_response(method_name);
                        let g =
                            FireboltGatekeeper::gate(platform_state.clone(), rpc_request.clone())
                                .await;
                        match g {
                            Ok(_) => {
                                println!("gatekeeper passed");
                                return service.call(legacy_request).await;
                            }
                            Err(deny_reason) => {
                                return MethodResponse::error(
                                    jsonrpsee_types::Id::Number(403),
                                    ErrorObject::owned(
                                        ErrorCode::InvalidRequest.code(),
                                        "Access Denied",
                                        Some(deny_reason),
                                    ),
                                );
                            }
                        }
                        // let f = service.call(legacy_request).await;

                        // println!(
                        //     "response from next layer: {:?} for {:?} ",
                        //     f,
                        //     request.clone()
                        // );
                        // return f;
                    }
                    println!("method_name: {:?} is rule based", method_name);

                    println!(
                        "Creating new connection_id={} app_id={} session_id={}",
                        connection_id, app_id, session_id
                    );
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
                                "could not register the sesion",
                                None::<()>,
                            ),
                        );
                    }
                    let _ =
                        PermissionHandler::fetch_and_store(&platform_state, &app_id, false).await;

                    let msg = FireboltGatewayCommand::HandleRpc {
                        request: rpc_request,
                    };
                    if let Err(e) = ripple_client.clone().send_gateway_command(msg) {
                        error!("failed to send request {:?}", e);
                    }
                    let api_message = session_rx.recv().await.unwrap();

                    let response =
                        match serde_json::from_str::<serde_json::Value>(&api_message.jsonrpc_msg) {
                            Ok(r) => {
                                let rp = ResponsePayload::success(r);
                                MethodResponse::response(request.id(), rp, 1024 * 2)
                            }
                            Err(e) => {
                                error!("failed to parse response {:?}", e);
                                MethodResponse::error(
                                    jsonrpsee_types::Id::Number(403),
                                    ErrorObject::owned(
                                        ErrorCode::InternalError.code(),
                                        "failed to parse response",
                                        None::<()>,
                                    ),
                                )
                            }
                        };
                    service.call(request.clone()).await;
                    return response;
                }
                None => {
                    error!("no app id found in session manager for session",);
                    service.call(request.clone()).await;
                    return MethodResponse::error(
                        jsonrpsee_types::Id::Number(403),
                        ErrorObject::owned(
                            ErrorCode::InternalError.code(),
                            "invalid session",
                            None::<()>,
                        ),
                    );
                }
            }
        }
        .boxed()
    }
}

#[derive(Clone)]
pub struct FireboltAuthLayer<S> {
    service: S,
    platform_state: PlatformState,
}
pub struct AuthenticatedFireboltLayer<S> {
    service: S,
    platform_state: PlatformState,
}

impl<'a, S> RpcServiceT<'a> for AuthenticatedFireboltLayer<S>
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
                                "no app id found in session manager for session id: {:?}",
                                firebolt_session
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

                    let firebolt_gateway_command = FireboltGatewayCommand::RegisterSession {
                        session_id: connection_id.clone(),
                        session,
                    };
                    if let Err(e) = ripple_client.send_gateway_command(firebolt_gateway_command) {
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
    let mut module = RpcModule::new(());
    let mut methods = Methods::new();
    ProviderRegistrar::register_methods(&platform_state, &mut methods);
    use crate::firebolt::rpc::*;
    // module
    // module.register_methods(methods);
    //module.register_method("firebolt.test", |_, _, _| "lo")?;
    let http_middleware =
        tower::ServiceBuilder::new().layer(FireboltSessionAuthenticatorLayer::new(
            internal_app_id.clone(),
            secure,
            platform_state.clone(),
        ));

    let addr = if secure {
        let closed_platform_state = platform_state.clone();
        let rpc_middleware =
            RpcServiceBuilder::new().layer_fn(move |service| AuthenticatedFireboltLayer {
                service,
                platform_state: closed_platform_state.clone(),
            });

        let server = jsonrpsee::server::Server::builder()
            .set_http_middleware(http_middleware)
            .set_rpc_middleware(rpc_middleware)
            .build(server_addr)
            .await?;
        let addr = server.local_addr()?;

        use crate::firebolt::handlers::account_rpc::*;
        let a = AccountRPCProvider::provide_with_alias(platform_state.clone());
        println!("a: {:?}", a);

        let handle = server.start(a);
        info!(
            "Listening on: {} secure={} with internal_app_id={:?}",
            server_addr, secure, internal_app_id
        );
        ripple_sdk::tokio::spawn(handle.stopped());
        addr
    } else {
        let default_app_id = internal_app_id.clone();
        let closed_platform_state = platform_state.clone();
        let rpc_middleware =
            RpcServiceBuilder::new().layer_fn(move |service| UnauthenticatedFireboltLayer {
                service,
                platform_state: closed_platform_state.clone(),
                default_app_id: internal_app_id.clone(),
            });
        let server = jsonrpsee::server::Server::builder()
            .set_http_middleware(http_middleware)
            .set_rpc_middleware(rpc_middleware)
            .build(server_addr)
            .await?;
        let addr = server.local_addr()?;
        use crate::firebolt::handlers::account_rpc::*;

        //let mut a = AccountRPCProvider::provide_with_alias(platform_state.clone()
        module.merge(AccountRPCProvider::provide_with_alias(
            platform_state.clone(),
        ))?;
        module.merge(DeviceRPCProvider::provide_with_alias(
            platform_state.clone(),
        ))?;
        module.merge(AudioDescriptionRPCProvider::provide_with_alias(
            platform_state.clone(),
        ))?;

        RpcModule::new(());
        //let a = Methods::new();
        //println!("a: {:?}", a);
        module.register_method("say_hello", |_, _, _| "lo")?;
        module.register_method("account.sesh", |_, _, _| "whatup?")?;

        let handle = server.start(module);
        //let handle = server.start(module);
        info!(
            "Listening on: {} secure={} with internal_app_id={:?}",
            server_addr, secure, default_app_id
        );
        ripple_sdk::tokio::spawn(handle.stopped());
        addr
    };
    /*
    TODO: bolt up to registrar
    */

    /*
    need to:
    register methods
    FireboltWs::start
    FireboltWs::handle_connection
        */
    //module.register_method("firebolt_gateway", FireboltGatewayCommand::new(state.clone()));

    // let addr = server.local_addr()?;

    // let handle = server.start(module);

    // // In this example we don't care about doing shutdown so let's it run forever.
    // // You may use the `ServerHandle` to shut it down or manage it yourself.
    // info!(

    //     "Listening on: {} secure={} with internal_app_id={:?}",
    //     server_addr, secure, internal_app_id
    // );
    // ripple_sdk::tokio::spawn(handle.stopped());
    Ok(addr)
}
