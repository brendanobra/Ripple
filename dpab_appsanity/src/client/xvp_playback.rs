use std::collections::HashSet;

use dpab_core::model::discovery::{MediaEventsAccountLinkRequestParams, ProgressUnit};

use hyper::body::{self};
use hyper::http;
use hyper::{header::CONTENT_TYPE, http::HeaderValue, Body, Client, Method, Request};
use hyper_tls::HttpsConnector;
use serde::{Deserialize, Serialize};
use tower::Service;
use tower::ServiceBuilder;
use tower::ServiceExt;

use tower_http::{
    classify::StatusInRangeAsFailures,
    decompression::DecompressionLayer,
    set_header::SetRequestHeaderLayer,
    trace::{DefaultOnResponse, TraceLayer},
};
use tracing::{debug, error, trace};
use url::{ParseError, Url};

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ResumePointBody {
    durable_app_id: String,
    progress: i64,
    progress_units: ProgressUnit,
    completed: bool,
    cet: HashSet<String>,
    owner_reference: String,
}

pub struct XvpError {}

impl From<ParseError> for XvpError {
    fn from(_: ParseError) -> Self {
        XvpError {}
    }
}

impl From<http::Error> for XvpError {
    fn from(_: http::Error) -> Self {
        XvpError {}
    }
}

#[derive(Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ResumePointResponse {
    pub message_id: Option<String>,
    pub sns_status_code: Option<i16>,
    pub sns_status_text: Option<String>,
    pub aws_request_id: Option<String>,
}

pub struct XvpPlayback {}

impl XvpPlayback {
    pub async fn put_resume_point(
        base_url: String,
        params: MediaEventsAccountLinkRequestParams,
    ) -> Result<ResumePointResponse, XvpError> {
        let mut url = Url::parse(&base_url)?;

        url.path_segments_mut()
            .map_err(|_| ParseError::SetHostOnCannotBeABaseUrl)?
            .push("partners")
            .push(&params.dist_session.id)
            .push("accounts")
            .push(&params.dist_session.account_id)
            .push("devices")
            .push(&params.dist_session.device_id)
            .push("resumePoints")
            .push("ott")
            .push(&params.content_partner_id)
            .push(&params.media_event.content_id);

        url.query_pairs_mut().append_pair("clientId", "ripple");

        debug!("Watched data tagged as {:?}", params.data_tags);

        let body = ResumePointBody {
            durable_app_id: params.media_event.app_id,
            progress: params.media_event.progress as i64,
            progress_units: params.media_event.progress_unit,
            completed: params.media_event.completed,
            cet: params.data_tags,
            owner_reference: format!("xrn:subscriber:device:{}", params.dist_session.device_id),
        };
        let body_json = serde_json::to_string(&body);
        if let Err(_) = body_json {
            return Err(XvpError {});
        }
        let resp_body = XvpPlayback::xvp_request(
            Method::PUT,
            url.into_string(),
            body_json.unwrap(),
            params.dist_session.token,
        )
        .await?;
        let resp: Result<ResumePointResponse, serde_json::Error> = serde_json::from_str(&resp_body);
        if resp.is_err() {
            return Err(XvpError {});
        }
        Ok(resp.unwrap())
    }

    async fn xvp_request(
        method: Method,
        uri: String,
        body: String,
        auth: String,
    ) -> Result<String, XvpError> {
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
            .layer(DecompressionLayer::new())
            .layer(TraceLayer::new_for_http())
            .service(hyper_client);

        let request = Request::builder()
            .uri(uri)
            .method(method)
            .header("Authorization", format!("Bearer {}", auth))
            .body(Body::from(body))?;

        let resp_res = client.ready().await.unwrap().call(request).await;
        if let Err(_) = resp_res {
            return Err(XvpError {});
        }
        let response = resp_res.unwrap();

        let status = response.status();
        trace!("status = {}", status.as_str());
        let bytes_res = body::to_bytes(response.into_body()).await;
        if let Err(_) = bytes_res {
            return Err(XvpError {});
        }
        let body_json =
            String::from_utf8(bytes_res.unwrap().to_vec()).expect("response was not valid utf-8");

        if !status.is_success() {
            error!(
                "XVP returned a non-succesful status: ({:?}), body: {}",
                status.as_str(),
                body_json
            );
            return Err(XvpError {});
        }

        Ok(body_json)
    }
}
