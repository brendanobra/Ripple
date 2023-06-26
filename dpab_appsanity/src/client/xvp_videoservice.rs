use dpab_core::model::discovery::{SessionParams, SignInRequestParams};

use hyper::header::CONTENT_TYPE;
use hyper::http::{self, HeaderValue, Request};
use hyper::{Body, Client, Method, StatusCode};
use hyper_tls::HttpsConnector;

use serde::{Deserialize, Serialize};
use tower::{Service, ServiceBuilder, ServiceExt};
use tower_http::trace::DefaultOnResponse;
use tower_http::{
    auth::AddAuthorizationLayer, classify::StatusInRangeAsFailures,
    decompression::DecompressionLayer, set_header::SetRequestHeaderLayer, trace::TraceLayer,
};
use tracing::{debug, trace};
use url::{ParseError, Url};

pub struct XvpClientError {}

impl From<ParseError> for XvpClientError {
    fn from(_: ParseError) -> Self {
        XvpClientError {}
    }
}

impl From<http::Error> for XvpClientError {
    fn from(_: http::Error) -> Self {
        XvpClientError {}
    }
}

pub struct XvpVideoService {}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct XvpVideoServiceErrResponse {
    #[serde(rename = "type")]
    pub _type: Option<String>,
    pub title: Option<String>,
    pub status: Option<i32>,
    pub detail: Option<String>,
    pub instance: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct XvpVideoServiceResponse {
    pub partner_id: Option<String>,
    pub account_id: Option<String>,
    pub owner_reference: Option<String>,
    pub entity_urn: Option<String>,
    pub entity_id: Option<String>,
    pub entity_type: Option<String>,
    pub durable_app_id: Option<String>,
    pub event_type: Option<String>,
    pub is_signed_in: Option<bool>,
    pub added: Option<String>,
    pub updated: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VideoServiceSignInInfo {
    pub event_type: String,
    pub is_signed_in: bool,
}

impl XvpVideoService {
    pub fn build_session_url(
        base_url: &str,
        scope: &str,
        session_params: &SessionParams,
    ) -> Result<String, XvpClientError> {
        let mut session_url = Url::parse(base_url)?;

        // create owner_reference based on sign_in_state scope.
        let (subscriber_field_name, subscriber_value) = match scope {
            "device" => ("device", session_params.dist_session.device_id.clone()),
            _ => ("account", session_params.dist_session.account_id.clone()),
        };

        let owner_reference = format!(
            "{}{}{}{}",
            "xrn:xcal:subscriber:", subscriber_field_name, ":", subscriber_value,
        );
        let entity_urn = format!(
            "{}{}",
            "xrn:xvp:application:",
            session_params.app_id.clone()
        );

        session_url
            .path_segments_mut()
            .map_err(|_| ParseError::SetHostOnCannotBeABaseUrl)?
            .push("partners")
            .push(&session_params.dist_session.id)
            .push("accounts")
            .push(&session_params.dist_session.account_id)
            .push("videoServices")
            .push(&entity_urn)
            .push("engaged");

        session_url
            .query_pairs_mut()
            .append_pair("ownerReference", &owner_reference)
            .append_pair("clientId", "ripple");

        Ok(session_url.to_string())
    }

    pub async fn sign_in(
        base_url: &str,
        scope: &str,
        params: SignInRequestParams,
    ) -> Result<XvpVideoServiceResponse, XvpClientError> {
        let session_url =
            XvpVideoService::build_session_url(base_url, scope, &params.session_info)?;
        let sign_in_info = VideoServiceSignInInfo {
            event_type: "signIn".to_owned(),
            is_signed_in: params.is_signed_in,
        };
        let body = serde_json::to_string(&sign_in_info).unwrap();

        let resp = XvpVideoService::xvp_session_request(
            Method::PUT,
            session_url,
            params.session_info.dist_session.token,
            body,
        )
        .await;

        resp
    }

    async fn xvp_session_request(
        method: Method,
        uri: String,
        auth_token: String,
        body: String,
    ) -> Result<XvpVideoServiceResponse, XvpClientError> {
        let hyper_client = Client::builder().build::<_, hyper::Body>(HttpsConnector::new());

        let mut client = ServiceBuilder::new()
            .layer(TraceLayer::new(
                StatusInRangeAsFailures::new(400..=599).into_make_classifier(),
            ))
            .layer(
                TraceLayer::new_for_http()
                    .on_response(DefaultOnResponse::new().level(tracing::Level::INFO)),
            )
            .layer(SetRequestHeaderLayer::overriding(
                CONTENT_TYPE,
                HeaderValue::from_static("application/json; charset=UTF-8"),
            ))
            .layer(AddAuthorizationLayer::bearer(&auth_token))
            .layer(DecompressionLayer::new())
            .layer(TraceLayer::new_for_http())
            .service(hyper_client);

        let req = Request::builder()
            .uri(uri.clone())
            .method(method.clone())
            .body(Body::from(body.clone()));

        debug!(
            "xvp_session_request: req={:?}, uri={}, method={}, body={}, token={}",
            req,
            uri,
            method.as_str(),
            body,
            auth_token
        );

        if let Err(_) = req {
            return Err(XvpClientError {});
        }

        let request = req.unwrap();

        let response = client.ready().await.unwrap().call(request).await;

        if let Err(_e) = response {
            return Err(XvpClientError {});
        }

        let response = response.unwrap();

        let status = response.status();
        trace!(
            "StatusCode {} Received for XVP Video Service Request",
            status.as_str()
        );

        if status.is_success() {
            if status == StatusCode::OK {
                if let Ok(body_bytes) = hyper::body::to_bytes(response.into_body()).await {
                    if let Ok(body_string) = String::from_utf8(body_bytes.to_vec()) {
                        let response: Result<XvpVideoServiceResponse, serde_json::Error> =
                            serde_json::from_str(&body_string);
                        match response {
                            Ok(r) => {
                                debug!("xvp-video service Response {:#?}", r);
                                return Ok(r);
                            }
                            Err(_) => {}
                        }
                    }
                }
            }
        } else {
            if let Ok(body_bytes) = hyper::body::to_bytes(response.into_body()).await {
                if let Ok(body_string) = String::from_utf8(body_bytes.to_vec()) {
                    let response: Result<XvpVideoServiceErrResponse, serde_json::Error> =
                        serde_json::from_str(&body_string);
                    match response {
                        Ok(r) => {
                            debug!("xvp-session Error Response {:#?}", r);
                        }
                        Err(_) => {}
                    }
                }
            }

            return Err(XvpClientError {});
        }

        Err(XvpClientError {})
    }
}
