use dpab_core::model::discovery::{
    ClearContentSetParams, ContentAccessInfo, ContentAccessListSetParams, SessionParams,
};

use hyper::header::CONTENT_TYPE;
use hyper::http::{self, HeaderValue, Request};
use hyper::{Body, Client, Method};
use hyper_tls::HttpsConnector;

use serde::Deserialize;
use tower::{Service, ServiceBuilder, ServiceExt};
use tower_http::trace::DefaultOnResponse;
use tower_http::{
    auth::AddAuthorizationLayer, classify::StatusInRangeAsFailures,
    decompression::DecompressionLayer, set_header::SetRequestHeaderLayer, trace::TraceLayer,
};
use tracing::{debug, trace};
use url::{ParseError, Url};

pub struct XvpSessionError {}

impl From<ParseError> for XvpSessionError {
    fn from(_: ParseError) -> Self {
        XvpSessionError {}
    }
}

impl From<http::Error> for XvpSessionError {
    fn from(_: http::Error) -> Self {
        XvpSessionError {}
    }
}

pub struct XvpSession {}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct XvpSessioErrorResponse {
    #[serde(rename = "type")]
    _type: Option<String>,
    title: Option<String>,
    status: Option<i32>,
    detail: Option<String>,
    instance: Option<String>,
}

impl XvpSession {
    pub fn build_session_url(
        base_url: String,
        session_params: &SessionParams,
    ) -> Result<String, XvpSessionError> {
        let mut session_url = Url::parse(&base_url)?;

        session_url
            .path_segments_mut()
            .map_err(|_| ParseError::SetHostOnCannotBeABaseUrl)?
            .push("partners")
            .push(&session_params.dist_session.id)
            .push("accounts")
            .push(&session_params.dist_session.account_id)
            .push("appSettings")
            .push(&session_params.app_id);

        session_url
            .query_pairs_mut()
            .append_pair("deviceId", &session_params.dist_session.device_id)
            .append_pair("clientId", "ripple");

        Ok(session_url.to_string())
    }

    pub async fn set_content_access(
        base_url: String,
        params: ContentAccessListSetParams,
    ) -> Result<(), XvpSessionError> {
        let session_url = XvpSession::build_session_url(base_url, &params.session_info)?;
        let body = serde_json::to_string(&params.content_access_info).unwrap();

        let resp = XvpSession::xvp_session_request(
            Method::PUT,
            session_url,
            params.session_info.dist_session.token,
            body,
        )
        .await;

        if resp.is_err() {
            return Err(XvpSessionError {});
        }
        Ok(())
    }

    pub async fn clear_content_access(
        base_url: String,
        params: ClearContentSetParams,
    ) -> Result<(), XvpSessionError> {
        let session_url = XvpSession::build_session_url(base_url, &params.session_info)?;

        // pass empty vectors for clearing the contents
        let content_access_info = ContentAccessInfo {
            availabilities: Some(vec![]),
            entitlements: Some(vec![]),
        };
        let body = serde_json::to_string(&content_access_info).unwrap();
        let resp = XvpSession::xvp_session_request(
            Method::PUT,
            session_url,
            params.session_info.dist_session.token,
            body,
        )
        .await;

        if resp.is_err() {
            return Err(XvpSessionError {});
        }
        Ok(())
    }

    async fn xvp_session_request(
        method: Method,
        uri: String,
        auth_token: String,
        body: String,
    ) -> Result<(), XvpSessionError> {
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
            return Err(XvpSessionError {});
        }

        let request = req.unwrap();

        let response = client.ready().await.unwrap().call(request).await;

        if let Err(_e) = response {
            return Err(XvpSessionError {});
        }

        let response = response.unwrap();

        let status = response.status();
        trace!(
            "StatusCode {} Received for XVP Session Request",
            status.as_str()
        );

        if status.is_success() {
            // StatusCode::NO_CONTENT for xvp session availabilities and entitlements
            // Request was accepted. Nothing to collect.
        } else {
            if let Ok(body_bytes) = hyper::body::to_bytes(response.into_body()).await {
                if let Ok(body_string) = String::from_utf8(body_bytes.to_vec()) {
                    let response: Result<XvpSessioErrorResponse, serde_json::Error> =
                        serde_json::from_str(&body_string);
                    match response {
                        Ok(r) => {
                            debug!("xvp-session Error Response {:#?}", r);
                        }
                        Err(_) => {}
                    }
                }
            }
            return Err(XvpSessionError {});
        }

        Ok(())
    }
}
