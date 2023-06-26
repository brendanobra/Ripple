use dpab_core::gateway::DpabDelegate;
use dpab_core::message::DistributorSession;
use dpab_core::message::DpabRequest;
use dpab_core::message::DpabRequestPayload;
use dpab_core::message::DpabRequestPayload::AppMetric;
use dpab_core::message::DpabRequestPayload::BadgerMetric as BadgerMetricPayload;

use dpab_core::model::metrics::AppError;
use dpab_core::model::metrics::AppLifecycleState;
use dpab_core::model::metrics::CategoryType;
use dpab_core::model::metrics::ContextualMetricsService;
use dpab_core::model::metrics::LoggingBehavioralMetricsManager;
use dpab_core::model::metrics::MediaPositionType;
use dpab_core::model::metrics::Param;
use dpab_core::model::metrics::{
    Action, AppBehavioralMetric, AppContext, BadgerAppAction, BadgerDismissLoadingScreen,
    BadgerError, BadgerLaunchCompleted, BadgerMetric, BadgerMetrics, BadgerMetricsService,
    BadgerPageView, BadgerUserAction, BadgerUserError, BehavioralMetricsService,
    DeviceMetricsContext, ErrorType, MediaEnded, MediaLoadStart, MediaPause, MediaPlay,
    MediaPlaying, MediaProgress, MediaRateChanged, MediaRenditionChanged, MediaSeeked,
    MediaSeeking, MediaWaiting, Page, Ready, SignIn, SignOut, StartContent, StopContent,
};
use env_logger::Logger;
use hyper::body::HttpBody;
use hyper::header;
use hyper::header::{HeaderName, CONTENT_TYPE};
use hyper::http::{HeaderValue, Request};
use hyper::{Body, Client, Method};
use hyper_tls::HttpsConnector;
use queues::*;
use serde::Serializer;
use serde::{Deserialize, Serialize};

use serde_json::Value;
use tokio::sync::Mutex as TokioMutex;
use tokio::sync::RwLock;
use tracing::debug;
use tracing::log::Log;
use tracing::trace;

use std::collections::HashSet;
use std::convert::From;
use std::sync::Arc;

use std::time::{SystemTime, UNIX_EPOCH};
use std::vec;

use tonic::async_trait;
use tower::{Service, ServiceBuilder, ServiceExt};
use tower_http::trace::DefaultOnResponse;
use tower_http::{
    classify::StatusInRangeAsFailures, decompression::DecompressionLayer,
    set_header::SetRequestHeaderLayer, trace::TraceLayer,
};

use tracing::{error, info};
use uuid::Uuid;

use crate::gateway::appsanity_gateway::MetricsSchema;
use crate::gateway::appsanity_gateway::MetricsSchemas;
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum InAppMediaEventType {
    MediaLoadStart,
    MediaPlay,
    MediaPlaying,
    MediaPause,
    MediaWaiting,
    MediaProgress,
    MediaSeeking,
    MediaSeeked,
    MediaRateChange,
    MediaRenditionChange,
    MediaEnded,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InAppMedia {
    media_event_name: InAppMediaEventType,
    src_page_id: String,
    app_session_id: String,
    app_user_session_id: String,
    durable_app_id: String,
    media_pos_pct: Option<i32>,
    media_pos_seconds: Option<i32>,
    playback_rate: Option<i32>,
    playback_bitrate: Option<i32>,
    playback_width: Option<i32>,
    playback_height: Option<i32>,
    playback_profile: Option<String>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InAppContentStart {
    src_entity_id: Option<String>,
    app_session_id: String,
    app_user_session_id: String,
    durable_app_id: String,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InAppContentStop {
    src_entity_id: Option<String>,
    app_session_id: String,
    app_user_session_id: String,
    durable_app_id: String,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InAppPageView {
    src_page_id: String,
    app_session_id: String,
    app_user_session_id: String,
    durable_app_id: String,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ActionCategory {
    #[serde(rename = "user")]
    User,
    #[serde(rename = "app")]
    App,
}
impl From<CategoryType> for ActionCategory {
    fn from(category_type: CategoryType) -> Self {
        match category_type {
            dpab_core::model::metrics::CategoryType::User => ActionCategory::User,
            dpab_core::model::metrics::CategoryType::App => ActionCategory::App,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InAppOtherAction {
    category: ActionCategory,
    #[serde(rename = "type")]
    action_type: String,
    parameters: Option<Vec<Param>>,
    app_session_id: String,
    app_user_session_id: String,
    durable_app_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppLifecycleStateChange {
    app_session_id: String,
    app_user_session_id: Option<String>,
    durable_app_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    previous_life_cycle_state: Option<AppLifecycleState>,
    new_life_cycle_state: AppLifecycleState,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppReady {
    app_session_id: String,
    app_user_session_id: String,
    durable_app_id: String,
    is_cold_launch: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InAppError {
    #[serde(skip_serializing)]
    pub context: AppContext,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_user_session_id: Option<String>,
    pub durable_app_id: String,
    pub third_party_error: bool,
    #[serde(rename = "type")]
    pub error_type: ErrorType,
    pub code: String,
    pub description: String,
    pub visible: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Vec<Param>>,
}
/*
This is basically a projection of FireboltBehavioralMetrics
A type alias will not work because we need to implement Traits (below)
and FireboltBehavioralMetrics is in another crate
*/
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum AXPMetric {
    InAppMedia(InAppMedia),
    InAppContentStart(InAppContentStart),
    InAppContentStop(InAppContentStop),
    InAppPageView(InAppPageView),
    InAppOtherAction(InAppOtherAction),
    AppLifecycleStateChange(AppLifecycleStateChange),
    InAppError(InAppError),
    AppReady(AppReady),
    BadgerAppAction(BadgerAppAction),
    BadgerMetric(BadgerMetric),
    BadgerError(BadgerError),
    BadgerLaunchCompleted(BadgerLaunchCompleted),
    BadgerDismissLoadingScreen(BadgerDismissLoadingScreen),
    BadgerPageView(BadgerPageView),
    BadgerUserAction(BadgerUserAction),
    BadgerUserError(BadgerUserError),
}

impl BehavioralMetric for AXPMetric {
    fn event_name(&self, metrics_schemas: &MetricsSchemas) -> String {
        match self {
            AXPMetric::BadgerAppAction(_) => metrics_schemas.get_event_name_alias("app_action"),
            AXPMetric::BadgerMetric(_) => metrics_schemas.get_event_name_alias("metric"),
            AXPMetric::BadgerError(_) => metrics_schemas.get_event_name_alias("error"),
            AXPMetric::BadgerLaunchCompleted(_) => {
                metrics_schemas.get_event_name_alias("launch_completed")
            }
            AXPMetric::BadgerDismissLoadingScreen(_) => {
                metrics_schemas.get_event_name_alias("dismiss_loading_screen")
            }
            AXPMetric::BadgerPageView(_) => metrics_schemas.get_event_name_alias("page_view"),
            AXPMetric::BadgerUserAction(_) => metrics_schemas.get_event_name_alias("user_action"),
            AXPMetric::BadgerUserError(_) => metrics_schemas.get_event_name_alias("user_error"),
            AXPMetric::AppLifecycleStateChange(_) => {
                metrics_schemas.get_event_name_alias("app_lc_state_change")
            }
            AXPMetric::InAppMedia(_) => metrics_schemas.get_event_name_alias("inapp_media"),
            AXPMetric::InAppError(_) => metrics_schemas.get_event_name_alias("app_error"),
            AXPMetric::InAppContentStart(_) => {
                metrics_schemas.get_event_name_alias("inapp_content_start")
            }
            AXPMetric::InAppContentStop(_) => {
                metrics_schemas.get_event_name_alias("inapp_content_stop")
            }
            AXPMetric::InAppPageView(_) => metrics_schemas.get_event_name_alias("inapp_page_view"),
            AXPMetric::InAppOtherAction(_) => {
                metrics_schemas.get_event_name_alias("inapp_other_action")
            }
            AXPMetric::AppReady(_) => metrics_schemas.get_event_name_alias("app_ready"),
        }
        .to_string()
    }

    fn event_type(&self) -> String {
        if self.is_badger() {
            "firebadger".to_string()
        } else {
            "firebolt".to_string()
        }
    }

    fn event_schema(&self, metrics_schemas: &MetricsSchemas) -> String {
        metrics_schemas.get_event_path(&self.event_name(metrics_schemas))
    }

    fn schema_version(&self) -> String {
        /*for now, not very exciting */
        "3".to_string()
    }
}
impl AXPMetric {
    fn is_badger(&self) -> bool {
        match self {
            AXPMetric::BadgerAppAction(_) => true,
            _ => false,
        }
    }
}

fn extract_context(metric: &AppBehavioralMetric) -> AppContext {
    match metric {
        AppBehavioralMetric::Ready(a) => a.context.clone(),
        AppBehavioralMetric::SignIn(a) => a.context.clone(),
        AppBehavioralMetric::SignOut(a) => a.context.clone(),
        AppBehavioralMetric::StartContent(a) => a.context.clone(),
        AppBehavioralMetric::StopContent(a) => a.context.clone(),
        AppBehavioralMetric::Page(a) => a.context.clone(),
        AppBehavioralMetric::Action(a) => a.context.clone(),
        AppBehavioralMetric::Error(e) => e.context.clone(),
        AppBehavioralMetric::MediaLoadStart(a) => a.context.clone(),
        AppBehavioralMetric::MediaPlay(a) => a.context.clone(),
        AppBehavioralMetric::MediaPlaying(a) => a.context.clone(),
        AppBehavioralMetric::MediaPause(a) => a.context.clone(),
        AppBehavioralMetric::MediaWaiting(a) => a.context.clone(),
        AppBehavioralMetric::MediaProgress(a) => a.context.clone(),
        AppBehavioralMetric::MediaSeeking(a) => a.context.clone(),
        AppBehavioralMetric::MediaSeeked(a) => a.context.clone(),
        AppBehavioralMetric::MediaRateChanged(a) => a.context.clone(),
        AppBehavioralMetric::MediaRenditionChanged(a) => a.context.clone(),
        AppBehavioralMetric::MediaEnded(a) => a.context.clone(),
        AppBehavioralMetric::AppStateChange(a) => a.context.clone(),
    }
}
fn badger_extract_context(metric: &BadgerMetrics) -> AppContext {
    match metric {
        BadgerMetrics::Metric(a) => a.context.clone(),
        BadgerMetrics::AppAction(a) => a.context.clone(),
        BadgerMetrics::Error(a) => a.context.clone(),
        BadgerMetrics::LaunchCompleted(a) => a.context.clone(),
        BadgerMetrics::DismissLoadingScreen(a) => a.context.clone(),
        BadgerMetrics::PageView(a) => a.context.clone(),
        BadgerMetrics::UserAction(a) => a.context.clone(),
        BadgerMetrics::UserError(a) => a.context.clone(),
    }
}

impl From<AppBehavioralMetric> for AXPMetric {
    fn from(metric: AppBehavioralMetric) -> Self {
        fb_2_pabs(&metric)
    }
}

/*PABS specific wrappers */
pub trait BehavioralMetric: Send + Clone + Sized {
    fn event_name(&self, metrics_schemas: &MetricsSchemas) -> String;
    fn event_type(&self) -> String;
    fn event_schema(&self, metrics_schemas: &MetricsSchemas) -> String;
    fn schema_version(&self) -> String;
}

pub trait CustomMetric {
    fn event_schema(&self, metrics_schemas: &MetricsSchemas) -> String;
}

/*
 { method: "badger.logMoneyBadgerLoaded",
 params_json: "[{\"app_id\":\"foo\",\"call_id\":0,
 \"method\":\"badger.logMoneyBadgerLoaded\",
 \"protocol\":\"Badger\",
 \"session_id\":\"ec027353-1c25-446e-8be8-994eac8a32ee\"},
 {\"startTime\":1660321072274,\"version\":\"4.10.0-7e1cc95\"}]",
      ctx: CallContext { session_id: "ec027353-1c25-446e-8be8-994eac8a32ee", app_id: "foo", call_id: 0, protocol: Badger, method: "badger.logMoneyBadgerLoaded" } }}:
*/
fn default_as_true() -> bool {
    true
}
fn option_string_to_pabs(the_string: &Option<String>) -> String {
    /*clone is required due to &*/
    the_string.clone().unwrap_or_else(|| String::from(""))
}
/*
SIFT likes empty strings
*/
fn serialize_pabs_option<S>(x: &Option<String>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match x {
        Some(val) => s.serialize_str(val),
        None => s.serialize_str(""),
    }
}
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct EntOsCetTags {
    durable_app_id: String,
    capabilities_exclusion_tags: HashSet<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct EntOsCustomMetric {
    device_mac: String,
    device_serial: String,
    source_app: String,
    source_app_version: String,
    #[serde(default = "default_as_true")]
    authenticated: bool,
    access_schema: Option<String>,
    access_payload: Option<EntOsCetTags>,
}

impl CustomMetric for EntOsCustomMetric {
    fn event_schema(&self, metrics_schemas: &MetricsSchemas) -> String {
        metrics_schemas.get_event_path("custom")
    }
}

/// serialization wrapper for Comcast Common Schema, aka "CCS"
#[derive(Serialize, Debug)]
pub struct CCS<T: Serialize, C: Serialize> {
    app_name: String,
    app_ver: String,
    device_language: String,
    device_model: String,
    partner_id: String,
    device_id: String,
    account_id: String,
    device_timezone: i32,
    device_name: String,
    platform: String,
    os_ver: String,
    session_id: String,
    event_id: String,
    event_type: String,
    event_name: String,
    timestamp: u64,
    event_schema: String,
    event_payload: T,
    custom_schema: String,
    custom_payload: C,
}
/*
Given a human readable timezone , compute ms offset from UTC
*/
fn get_timezone_offset(_metrics_context: &DeviceMetricsContext) -> i32 {
    use chrono::Local;
    Local::now().offset().local_minus_utc() * 1000
}

impl<T: Serialize + BehavioralMetric, C: Serialize + CustomMetric> CCS<T, C> {
    fn new(
        metrics_context: &DeviceMetricsContext,
        app_name: String,
        app_ver: String,
        partner_id: String,
        event_payload: T,
        custom_payload: C,
        metrics_schemas: &MetricsSchemas,
    ) -> CCS<T, C> {
        let timestamp: u64 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis()
            .try_into()
            .unwrap();

        CCS {
            app_name,
            app_ver: app_ver,
            device_language: metrics_context.device_language.to_string(),
            device_model: metrics_context.device_model.to_string(),
            partner_id: partner_id,
            device_id: metrics_context.device_id.to_string(),
            account_id: metrics_context.account_id.to_string(),
            device_timezone: get_timezone_offset(&metrics_context),
            device_name: metrics_context.device_name.to_string(),
            platform: metrics_context.platform.to_string(),
            os_ver: metrics_context.os_ver.to_string(),
            session_id: metrics_context.device_session_id.to_string(),
            event_id: Uuid::new_v4().to_string(),
            event_type: event_payload.event_type(),
            event_name: event_payload.event_name(metrics_schemas),
            timestamp: timestamp,
            event_schema: event_payload.event_schema(metrics_schemas),
            event_payload: event_payload,
            custom_schema: custom_payload.event_schema(metrics_schemas),
            custom_payload: custom_payload,
        }
    }
}

pub struct SiftService {
    pub sift_endpoint: String,
    pub batch_size: u8,
    pub metrics_schemas: MetricsSchemas,
    metrics_send_queue: Arc<TokioMutex<CircularBuffer<Value>>>,
    metrics_context: Option<DeviceMetricsContext>,
}

#[async_trait]
impl BehavioralMetricsService for SiftService {
    async fn send_metric(
        &mut self,
        payload: AppBehavioralMetric,
        session: DistributorSession,
    ) -> () {
        let _ = self.send_behavioral(payload).await;
    }
}
#[async_trait]
impl BadgerMetricsService for SiftService {
    async fn send_badger_metric(
        &mut self,
        payload: BadgerMetrics,
        session: DistributorSession,
    ) -> () {
        self.send_badger(payload).await;
    }
}
#[async_trait]
impl ContextualMetricsService for SiftService {
    async fn update_metrics_context(&mut self, new_context: Option<DeviceMetricsContext>) -> () {
        self.set_metrics_context(new_context)
    }
}

fn fb_2_pabs(firebolt_metric: &AppBehavioralMetric) -> AXPMetric {
    match firebolt_metric {
        AppBehavioralMetric::Ready(a) => {
            AXPMetric::AppLifecycleStateChange(AppLifecycleStateChange {
                app_session_id: a.context.app_session_id.clone(),
                app_user_session_id: a.context.app_user_session_id.clone(),
                durable_app_id: a.context.durable_app_id.clone(),
                previous_life_cycle_state: None,
                new_life_cycle_state: AppLifecycleState::Foreground,
            })
        }
        AppBehavioralMetric::AppStateChange(a) => {
            AXPMetric::AppLifecycleStateChange(AppLifecycleStateChange {
                app_session_id: a.context.app_session_id.clone(),
                app_user_session_id: a.context.app_user_session_id.clone(),
                durable_app_id: a.context.durable_app_id.clone(),
                previous_life_cycle_state: a.previous_state.clone(),
                new_life_cycle_state: a.new_state.clone(),
            })
        }
        AppBehavioralMetric::Error(a) => AXPMetric::InAppError(InAppError {
            context: a.context.clone(),
            app_session_id: a.app_session_id.clone(),
            app_user_session_id: a.app_user_session_id.clone(),
            durable_app_id: a.durable_app_id.clone(),
            third_party_error: a.third_party_error,
            error_type: a.error_type.clone(),
            code: a.code.clone(),
            description: a.description.clone(),
            visible: a.visible,
            parameters: a.parameters.clone(),
        }),

        AppBehavioralMetric::SignIn(sign_in) => AXPMetric::InAppOtherAction(InAppOtherAction {
            category: ActionCategory::User,
            action_type: String::from("sign_in"),
            parameters: None,
            app_session_id: sign_in.context.app_session_id.clone(),
            app_user_session_id: option_string_to_pabs(
                &sign_in.context.app_user_session_id.clone(),
            ),
            durable_app_id: sign_in.context.durable_app_id.clone(),
        }),
        AppBehavioralMetric::SignOut(sign_out) => AXPMetric::InAppOtherAction(InAppOtherAction {
            category: ActionCategory::User,
            action_type: String::from("sign_out"),
            parameters: None,
            app_session_id: sign_out.context.app_session_id.clone(),
            app_user_session_id: option_string_to_pabs(
                &sign_out.context.app_user_session_id.clone(),
            ),
            durable_app_id: sign_out.context.durable_app_id.clone(),
        }),

        AppBehavioralMetric::StartContent(start_content) => {
            AXPMetric::InAppContentStart(InAppContentStart {
                src_entity_id: start_content.entity_id.clone(),
                app_session_id: start_content.context.app_session_id.clone(),
                app_user_session_id: option_string_to_pabs(
                    &start_content.context.app_user_session_id.clone(),
                ),
                durable_app_id: start_content.context.durable_app_id.clone(),
            })
        }
        AppBehavioralMetric::StopContent(stop_content) => {
            AXPMetric::InAppContentStop(InAppContentStop {
                src_entity_id: stop_content.entity_id.clone(),
                app_session_id: stop_content.context.app_session_id.clone(),
                app_user_session_id: option_string_to_pabs(
                    &stop_content.context.app_user_session_id,
                ),
                durable_app_id: stop_content.context.durable_app_id.clone(),
            })
        }

        AppBehavioralMetric::Page(page) => AXPMetric::InAppPageView(InAppPageView {
            src_page_id: page.page_id.clone(),
            app_session_id: page.context.app_session_id.clone(),
            app_user_session_id: option_string_to_pabs(&page.context.app_user_session_id),
            durable_app_id: page.context.durable_app_id.clone(),
        }),

        AppBehavioralMetric::Action(action) => AXPMetric::InAppOtherAction(InAppOtherAction {
            category: action.category.clone().into(),
            action_type: action._type.clone(),
            parameters: Some(action.parameters.clone()),
            app_session_id: action.context.app_session_id.clone(),
            app_user_session_id: option_string_to_pabs(&action.context.app_user_session_id),
            durable_app_id: action.context.durable_app_id.clone(),
        }),
        AppBehavioralMetric::MediaLoadStart(media_load_start) => {
            AXPMetric::InAppMedia(InAppMedia {
                media_event_name: InAppMediaEventType::MediaLoadStart,
                src_page_id: media_load_start.entity_id.clone(),
                app_session_id: media_load_start.context.app_session_id.clone(),
                app_user_session_id: option_string_to_pabs(
                    &media_load_start.context.app_user_session_id,
                ),
                durable_app_id: media_load_start.context.durable_app_id.clone(),
                media_pos_pct: None,
                media_pos_seconds: None,
                playback_rate: None,
                playback_bitrate: None,
                playback_width: None,
                playback_height: None,
                playback_profile: None,
            })
        }

        AppBehavioralMetric::MediaPlay(media_play) => AXPMetric::InAppMedia(InAppMedia {
            media_event_name: InAppMediaEventType::MediaPlay,
            src_page_id: media_play.entity_id.clone(),
            app_session_id: media_play.context.app_session_id.clone(),
            app_user_session_id: option_string_to_pabs(&media_play.context.app_user_session_id),
            durable_app_id: media_play.context.durable_app_id.clone(),
            media_pos_pct: None,
            media_pos_seconds: None,
            playback_rate: None,
            playback_bitrate: None,
            playback_width: None,
            playback_height: None,
            playback_profile: None,
        }),

        AppBehavioralMetric::MediaPlaying(media_playing) => AXPMetric::InAppMedia(InAppMedia {
            media_event_name: InAppMediaEventType::MediaPlaying,
            src_page_id: media_playing.entity_id.clone(),
            app_session_id: media_playing.context.app_session_id.clone(),
            app_user_session_id: option_string_to_pabs(&media_playing.context.app_user_session_id),
            durable_app_id: media_playing.context.durable_app_id.clone(),
            media_pos_pct: None,
            media_pos_seconds: None,
            playback_rate: None,
            playback_bitrate: None,
            playback_width: None,
            playback_height: None,
            playback_profile: None,
        }),
        AppBehavioralMetric::MediaPause(event) => AXPMetric::InAppMedia(InAppMedia {
            media_event_name: InAppMediaEventType::MediaPlaying,
            src_page_id: event.entity_id.clone(),
            app_session_id: event.context.app_session_id.clone(),
            app_user_session_id: option_string_to_pabs(&event.context.app_user_session_id),
            durable_app_id: event.context.durable_app_id.clone(),
            media_pos_pct: None,
            media_pos_seconds: None,
            playback_rate: None,
            playback_bitrate: None,
            playback_width: None,
            playback_height: None,
            playback_profile: None,
        }),

        AppBehavioralMetric::MediaWaiting(event) => AXPMetric::InAppMedia(InAppMedia {
            media_event_name: InAppMediaEventType::MediaWaiting,
            src_page_id: event.entity_id.clone(),
            app_session_id: event.context.app_session_id.clone(),
            app_user_session_id: option_string_to_pabs(&event.context.app_user_session_id),
            durable_app_id: event.context.durable_app_id.clone(),
            media_pos_pct: None,
            media_pos_seconds: None,
            playback_rate: None,
            playback_bitrate: None,
            playback_width: None,
            playback_height: None,
            playback_profile: None,
        }),

        AppBehavioralMetric::MediaProgress(event) => AXPMetric::InAppMedia(InAppMedia {
            media_event_name: InAppMediaEventType::MediaProgress,
            src_page_id: event.entity_id.clone(),
            app_session_id: event.context.app_session_id.clone(),
            app_user_session_id: option_string_to_pabs(&event.context.app_user_session_id),
            durable_app_id: event.context.durable_app_id.clone(),
            media_pos_pct: event.into(),
            media_pos_seconds: event.into(),
            playback_rate: None,
            playback_bitrate: None,
            playback_width: None,
            playback_height: None,
            playback_profile: None,
        }),

        AppBehavioralMetric::MediaSeeking(event) => AXPMetric::InAppMedia(InAppMedia {
            media_event_name: InAppMediaEventType::MediaSeeking,
            src_page_id: event.entity_id.clone(),
            app_session_id: event.context.app_session_id.clone(),
            app_user_session_id: option_string_to_pabs(&event.context.app_user_session_id),
            durable_app_id: event.context.durable_app_id.clone(),
            media_pos_pct: event.into(),
            media_pos_seconds: event.into(),
            playback_rate: None,
            playback_bitrate: None,
            playback_width: None,
            playback_height: None,
            playback_profile: None,
        }),

        AppBehavioralMetric::MediaSeeked(event) => AXPMetric::InAppMedia(InAppMedia {
            media_event_name: InAppMediaEventType::MediaSeeked,
            src_page_id: event.entity_id.clone(),
            app_session_id: event.context.app_session_id.clone(),
            app_user_session_id: option_string_to_pabs(&event.context.app_user_session_id),
            durable_app_id: event.context.durable_app_id.clone(),
            media_pos_pct: event.into(),
            media_pos_seconds: event.into(),
            playback_rate: None,
            playback_bitrate: None,
            playback_width: None,
            playback_height: None,
            playback_profile: None,
        }),

        AppBehavioralMetric::MediaRateChanged(event) => AXPMetric::InAppMedia(InAppMedia {
            media_event_name: InAppMediaEventType::MediaRateChange,
            src_page_id: event.entity_id.clone(),
            app_session_id: event.context.app_session_id.clone(),
            app_user_session_id: option_string_to_pabs(&event.context.app_user_session_id),
            durable_app_id: event.context.durable_app_id.clone(),
            media_pos_pct: None,
            media_pos_seconds: None,
            playback_rate: Some(event.rate.try_into().unwrap()),
            playback_bitrate: None,
            playback_width: None,
            playback_height: None,
            playback_profile: None,
        }),

        AppBehavioralMetric::MediaRenditionChanged(event) => AXPMetric::InAppMedia(InAppMedia {
            media_event_name: InAppMediaEventType::MediaRenditionChange,
            src_page_id: event.entity_id.clone(),
            app_session_id: event.context.app_session_id.clone(),
            app_user_session_id: option_string_to_pabs(&event.context.app_user_session_id),
            durable_app_id: event.context.durable_app_id.clone(),
            media_pos_pct: None,
            media_pos_seconds: None,
            playback_rate: None,
            playback_bitrate: Some(event.bitrate.try_into().unwrap()),
            playback_width: Some(event.width.try_into().unwrap()),
            playback_height: Some(event.height.try_into().unwrap()),
            playback_profile: event.profile.clone(),
        }),

        AppBehavioralMetric::MediaEnded(event) => AXPMetric::InAppMedia(InAppMedia {
            media_event_name: InAppMediaEventType::MediaEnded,
            src_page_id: event.entity_id.clone(),
            app_session_id: event.context.app_session_id.clone(),
            app_user_session_id: option_string_to_pabs(&event.context.app_user_session_id),
            durable_app_id: event.context.durable_app_id.clone(),
            media_pos_pct: None,
            media_pos_seconds: None,
            playback_rate: None,
            playback_bitrate: None,
            playback_width: None,
            playback_height: None,
            playback_profile: None,
        }),
    }
}

pub fn get_access_schema(app_context: &AppContext) -> Option<String> {
    match app_context.governance_state.clone() {
        Some(_) => Some(String::from("access/tags/0")),
        None => None,
    }
}
impl SiftService {
    pub fn set_metrics_context(&mut self, metrics_context: Option<DeviceMetricsContext>) {
        self.metrics_context = metrics_context;
    }

    fn badger_2_pabs(&self, badger_metric: &BadgerMetrics) -> impl BehavioralMetric + Serialize {
        match badger_metric {
            BadgerMetrics::Metric(a) => AXPMetric::BadgerMetric(BadgerMetric {
                context: a.context.clone(),
                segment: a.segment.clone(),
                args: a.args.clone(),
            }),
            BadgerMetrics::AppAction(a) => AXPMetric::BadgerAppAction(BadgerAppAction {
                context: a.context.clone(),
                action: a.action.clone(),
                args: a.args.clone(),
            }),
            BadgerMetrics::Error(a) => AXPMetric::BadgerError(BadgerError {
                context: a.context.clone(),
                message: a.message.clone(),
                visible: a.visible,
                code: a.code,
                args: a.args.clone(),
            }),
            BadgerMetrics::LaunchCompleted(a) => {
                AXPMetric::BadgerLaunchCompleted(BadgerLaunchCompleted {
                    context: a.context.clone(),
                    args: a.args.clone(),
                })
            }
            BadgerMetrics::DismissLoadingScreen(a) => {
                AXPMetric::BadgerDismissLoadingScreen(BadgerDismissLoadingScreen {
                    context: a.context.clone(),
                    args: a.args.clone(),
                })
            }
            BadgerMetrics::PageView(a) => AXPMetric::BadgerPageView(BadgerPageView {
                context: a.context.clone(),
                page: a.page.clone(),
                args: a.args.clone(),
            }),
            BadgerMetrics::UserAction(a) => AXPMetric::BadgerUserAction(BadgerUserAction {
                context: a.context.clone(),
                args: a.args.clone(),
            }),
            BadgerMetrics::UserError(a) => AXPMetric::BadgerUserError(BadgerUserError {
                context: a.context.clone(),
                message: a.message.clone(),
                visible: a.visible,
                code: a.code,
                args: a.args.clone(),
            }),
        }
    }

    fn extract_app_id(&self, metric: &AppBehavioralMetric) -> String {
        extract_context(metric).app_id.clone()
    }
    fn extract_partner_id(&self, metric: &AppBehavioralMetric) -> String {
        extract_context(metric).partner_id.clone()
    }
    fn extract_app_version(&self, metric: &AppBehavioralMetric) -> String {
        extract_context(metric).app_version.clone()
    }
    fn badger_extract_app_id(&self, metric: &BadgerMetrics) -> String {
        badger_extract_context(metric).app_id.clone()
    }
    fn badger_extract_partner_id(&self, metric: &BadgerMetrics) -> String {
        badger_extract_context(metric).partner_id.clone()
    }
    fn badger_extract_app_version(&self, metric: &BadgerMetrics) -> String {
        badger_extract_context(metric).app_id.clone()
    }

    pub fn new(
        sift_endpoint: String,
        batch_size: u8,
        max_queue_size: u8,
        metrics_schemas: MetricsSchemas,
        metrics_context: Option<DeviceMetricsContext>,
        eos_rendered: Arc<TokioMutex<CircularBuffer<Value>>>,
    ) -> impl BehavioralMetricsService
           + BadgerMetricsService
           + ContextualMetricsService
           + DpabDelegate
           + Sync
           + Send {
        let result = SiftService {
            sift_endpoint: sift_endpoint,
            batch_size: batch_size,
            metrics_schemas,
            metrics_context: metrics_context,
            metrics_send_queue: eos_rendered,
        };

        result
    }

    pub async fn send_behavioral(&mut self, firebolt_metric: AppBehavioralMetric) {
        if self.metrics_context.is_none() {
            error!("be metrics context is not set");
            return;
        };
        /*
        This is the app name in the TOP LEVEL CCS schema
        */
        let app_name = String::from("entos");
        let partner_id = self.extract_partner_id(&firebolt_metric);
        let app_ver = self.extract_app_version(&firebolt_metric);
        let context = self.metrics_context.as_ref().unwrap();
        let app_context = extract_context(&firebolt_metric);
        let (schema, tags) = if let Some(cet_tags) = app_context.clone().governance_state {
            if cet_tags.data_tags_to_apply.len() > 0 {
                (
                    get_access_schema(&app_context.clone()),
                    Some(EntOsCetTags {
                        durable_app_id: app_context.durable_app_id.clone(),
                        capabilities_exclusion_tags: cet_tags.data_tags_to_apply,
                    }),
                )
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        let custom_payload = EntOsCustomMetric {
            device_mac: context.mac_address.to_string(),
            device_serial: context.serial_number.to_string(),
            source_app: self.extract_app_id(&firebolt_metric),
            source_app_version: self.extract_app_version(&firebolt_metric),
            authenticated: true,
            access_schema: schema,
            access_payload: tags,
        };

        let payload = CCS::new(
            self.metrics_context.as_ref().unwrap(),
            app_name,
            app_ver,
            partner_id,
            fb_2_pabs(&firebolt_metric),
            custom_payload,
            &self.metrics_schemas,
        );
        let mut batch = self.metrics_send_queue.lock().await;
        debug!("behavioral={}", serde_json::to_string(&payload).unwrap());
        batch.add(serde_json::to_value(payload).unwrap());
    }

    pub async fn send_badger(&mut self, badger_metric: BadgerMetrics) {
        if self.metrics_context.is_none() {
            println!("ba metrics context is not set");
            return;
        };

        let b = self.badger_2_pabs(&badger_metric);
        let app_name = String::from("entos");
        let partner_id = self.badger_extract_partner_id(&badger_metric);
        let app_ver = self.badger_extract_app_version(&badger_metric);

        let context = self.metrics_context.as_ref().unwrap();
        let app_context = badger_extract_context(&badger_metric);
        let (schema, tags) = if let Some(cet_tags) = app_context.clone().governance_state {
            if cet_tags.data_tags_to_apply.len() > 0 {
                (
                    get_access_schema(&app_context.clone()),
                    Some(EntOsCetTags {
                        durable_app_id: app_context.durable_app_id.clone(),
                        capabilities_exclusion_tags: cet_tags.data_tags_to_apply,
                    }),
                )
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        let custom_payload = EntOsCustomMetric {
            device_mac: context.mac_address.to_string(),
            device_serial: context.serial_number.to_string(),
            source_app: self.badger_extract_app_id(&badger_metric),
            source_app_version: self.badger_extract_app_version(&badger_metric),
            authenticated: true,
            access_schema: schema,
            access_payload: tags,
        };

        let payload = CCS::new(
            self.metrics_context.as_ref().unwrap(),
            app_name,
            app_ver,
            partner_id,
            b,
            custom_payload,
            &self.metrics_schemas,
        );
        let mut batch = self.metrics_send_queue.lock().await;
        debug!("badger  = {}", serde_json::to_string(&payload).unwrap());
        batch.add(serde_json::to_value(&payload).unwrap());
    }
}

pub async fn send_metrics(
    metrics_queue: Arc<TokioMutex<CircularBuffer<Value>>>,
    uri: String,
    token: String,
    batch_size: u16,
) {
    /*
    render the union of batches, and send
    */
    let mut batch: Vec<Value> = vec![];
    {
        let rendered_metrics = &mut *metrics_queue.lock().await;
        while rendered_metrics.size() > 0 {
            let mut metric = rendered_metrics.remove().unwrap();
            batch.push(metric);
        }
    }
    let f: usize = batch_size.into();
    let batch_num = match batch.len() / f {
        0 => 1,
        num => num,
    };

    let batches: Vec<&[Value]> = batch.chunks(batch_num).collect();

    for chunk in batches.iter() {
        let payloads = serde_json::to_string(chunk).unwrap();
        debug!("sending batch of {} BI metrics", batch.len());
        match send_to_axp(payloads, uri.clone(), token.clone()).await {
            Ok(okie) => {
                debug!("SIFT metrics send success={:?}", okie);
            }
            Err(uhoh) => {
                error!(
                    "error sending bi metrics={:?}, will requeue and re-attempt later",
                    uhoh
                );
                let rendered_metrics = &mut *metrics_queue.lock().await;
                for metric in batch.iter() {
                    rendered_metrics.add(metric.clone());
                }
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum SiftError {
    ClientError,
    TranportError,
    Non200ResponseError(u16),
}
pub async fn send_to_axp(
    payloads: String,
    endpoint_uri: String,
    auth: String,
) -> Result<(), SiftError> {
    let hyper_client = Client::builder().build::<_, hyper::Body>(HttpsConnector::new());

    let mut client = ServiceBuilder::new()
        // Add tracing and consider server errors and client
        // errors as failures.
        .layer(TraceLayer::new(
            StatusInRangeAsFailures::new(400..=599).into_make_classifier(),
        ))
        .layer(
            TraceLayer::new_for_http()
                .on_response(DefaultOnResponse::new().level(tracing::Level::DEBUG)),
        )
        // Set x-api-key.
        .layer(SetRequestHeaderLayer::overriding(
            CONTENT_TYPE,
            HeaderValue::from_static("application/json; charset=UTF-8"),
        ))
        .layer(SetRequestHeaderLayer::overriding(
            header::AUTHORIZATION,
            HeaderValue::from_str(auth.as_str()).unwrap(),
        ))
        // Decompress response bodies
        .layer(DecompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        // Wrap a `hyper::Client` in our middleware stack.
        // This is possible because `hyper::Client` implements
        // `tower::Service`.
        .service(hyper_client);

    // Make a request
    if let Ok(request) = Request::builder()
        .uri(endpoint_uri)
        .method(Method::POST)
        .body(Body::from(payloads.clone()))
    {
        /*
        TODO wrap this is fire and forget tokio
        */

        let response = client.ready().await.unwrap().call(request).await;
        match response {
            Ok(mut ok) => {
                trace!("status = {}", ok.status().as_str());
                if ok.status() != 200 {
                    error!(
                        "SIFT returned a  non 200 ({:?}) status: when sending: {:?} ",
                        ok.status().as_str(),
                        payloads.clone()
                    );
                    while let Some(chunk) = ok.body_mut().data().await {
                        error!("{:?}", &chunk);
                    }
                    Err(SiftError::Non200ResponseError(ok.status().as_u16()))
                } else {
                    Ok(())
                }
            }
            Err(err) => {
                error!("error sending to SIFT={:?}", err);
                Err(SiftError::ClientError)
            }
        }
    } else {
        error!("could not compose a PABS request, will not send metrics");
        Err(SiftError::ClientError)
    }
}

#[cfg(any(feature = "local_metrics_logging"))]
pub fn config_testing_logging() {
    use std::env;

    use log4rs::{
        append::{
            file::FileAppender,
            rolling_file::{
                policy::compound::{
                    roll::fixed_window::FixedWindowRoller, trigger::size::SizeTrigger,
                    CompoundPolicy,
                },
                RollingFileAppender, RollingFileAppenderBuilder,
            },
        },
        config::{Appender, Root},
        encode::json::JsonEncoder,
        Config,
    };
    use tracing::log::LevelFilter;
    #[cfg(not(feature = "local_metrics_logging"))]
    let file = match env::var("TEST_LOG_FILE") {
        Ok(file) => file,
        Err(e) => String::from("./ripple_metrics.json"),
    };

    #[cfg(feature = "local_metrics_logging")]
    let file = match env::var("TEST_LOG_FILE") {
        Ok(file) => file,
        Err(e) => String::from("/opt/logs/ripple_metrics.json"),
    };
    info!(
        "local metrics logging is setup, logs will be written to: {}",
        file
    );

    let window_size = 3; // log0, log1, log2
    let fixed_window_roller = FixedWindowRoller::builder()
        .build("log{}", window_size)
        .unwrap();
    let size_limit = 50 * 1024; // 5KB as max log file size to roll
    let size_trigger = SizeTrigger::new(size_limit);

    let compound_policy =
        CompoundPolicy::new(Box::new(size_trigger), Box::new(fixed_window_roller));

    let logfile = RollingFileAppender::builder()
        .encoder(Box::new(JsonEncoder::new()))
        .build(&file, Box::new(compound_policy))
        .unwrap();

    let config = Config::builder()
        .appender(Appender::builder().build("logfile", Box::new(logfile)))
        .build(Root::builder().appender("logfile").build(LevelFilter::Info))
        .unwrap();
    log4rs::init_config(config).unwrap();
}
#[cfg(any(feature = "local_metrics_logging", feature = "local_dev"))]
pub fn log(message: &Value) {
    log::info!("{}", message);
}
#[cfg(any(feature = "local_metrics_logging", feature = "local_dev"))]
async fn local_behavioral_logging(
    payload: DpabRequestPayload,
    sift_endpoint: String,
    metrics_send_queue: Arc<TokioMutex<CircularBuffer<Value>>>,
    batch_size: u16,
) {
    if let AppMetric(context, metric, session) = payload {
        tokio::spawn(async move {
            LoggingBehavioralMetricsManager::new(context, log)
                .send_metric(metric)
                .await;
        });
        send_metrics(
            metrics_send_queue.clone(),
            sift_endpoint.clone(),
            session.token,
            batch_size.into(),
        )
        .await;
    }
}
#[cfg(any(feature = "local_metrics_logging", feature = "local_dev"))]
async fn local_badger_logging(
    payload: DpabRequestPayload,
    sift_endpoint: String,
    metrics_send_queue: Arc<TokioMutex<CircularBuffer<Value>>>,
    batch_size: u16,
) {
    if let BadgerMetricPayload(context, metric, session) = payload {
        tokio::spawn(async move {
            LoggingBehavioralMetricsManager::new(context, log)
                .send_badger_metric(metric)
                .await;
        });
        send_metrics(
            metrics_send_queue.clone(),
            sift_endpoint.clone(),
            session.token,
            batch_size.into(),
        )
        .await;
    }
}
#[async_trait]
impl DpabDelegate for SiftService {
    async fn handle(&mut self, request: DpabRequest) {
        match &request.payload {
            AppMetric(context, metric, session) => {
                self.set_metrics_context(context.clone());
                self.send_metric(metric.clone(), session.clone()).await;
                #[cfg(any(feature = "local_metrics_logging", feature = "local_dev"))]
                local_behavioral_logging(
                    request.payload.clone(),
                    self.sift_endpoint.clone(),
                    self.metrics_send_queue.clone(),
                    self.batch_size.into(),
                )
                .await;
            }
            BadgerMetricPayload(context, badger_metric, session) => {
                self.set_metrics_context(context.clone());
                self.send_badger_metric(badger_metric.clone(), session.clone())
                    .await;

                #[cfg(any(feature = "local_metrics_logging", feature = "local_dev"))]
                local_badger_logging(
                    request.payload.clone(),
                    self.sift_endpoint.clone(),
                    self.metrics_send_queue.clone(),
                    self.batch_size.into(),
                )
                .await;
            }
            _ => {}
        }
    }
}
///
/// All test currently (simply) check that metrics will not interfere with UX in any - for instance,
/// auth or connection failure should just log the failure , but not push an errors
/// back to caller who is await-ing.
///
#[cfg(test)]
pub mod tests {
    use std::sync::Arc;

    use crate::gateway::appsanity_gateway::MetricsSchemas;

    use super::DeviceMetricsContext;

    use super::BehavioralMetricsService;
    use super::ContextualMetricsService;
    use super::EntOsCustomMetric;
    use super::SiftService;

    use dpab_core::message::DistributorSession;
    use dpab_core::model::metrics::AppBehavioralMetric;
    use dpab_core::model::metrics::BadgerAppAction;
    use dpab_core::model::metrics::BadgerMetrics;
    use dpab_core::model::metrics::BadgerMetricsService;
    use dpab_core::model::metrics::CategoryType;
    use dpab_core::model::metrics::{Action, ActionType, AppContext, Param, Ready};
    use queues::CircularBuffer;
    use tokio::sync::Mutex;
    use tokio::sync::RwLock;
    use uuid::Uuid;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};
    fn metrics_context() -> DeviceMetricsContext {
        DeviceMetricsContext {
            device_language: "en-US".to_string(),
            device_model: "eniac".to_string(),
            device_id: "0123456789".to_string(),
            account_id: "0123456789".to_string(),
            device_timezone: String::from("PDT"),
            device_name: "basement".to_string(),
            platform: "web".to_string(),
            os_ver: "0.0.1".to_string(),
            device_session_id: Uuid::new_v4().to_string(),
            mac_address: "DEAD-BEEF-DEAD".to_string(),
            serial_number: "0123456789".to_string(),
            distribution_tenant_id: "comcast".to_string(),
        }
    }
    fn metrics_schemas() -> MetricsSchemas {
        MetricsSchemas {
            default_metrics_namespace: "namespace".to_string(),
            default_metrics_schema_version: "1.0".to_string(),
            metrics_schemas: vec![],
        }
    }
    fn behavioral_context() -> AppContext {
        AppContext {
            app_id: "foo".to_string(),
            app_version: "1.2.3".to_string(),
            partner_id: "foocast".to_string(),
            app_session_id: "foo".to_string(),
            app_user_session_id: Some("foo".to_string()),
            durable_app_id: "foo".to_string(),
            governance_state: None,
        }
    }
    fn custom_payload() -> EntOsCustomMetric {
        EntOsCustomMetric {
            device_mac: "BEEF".to_string(),
            device_serial: "cereal".to_string(),
            source_app: String::from("app"),
            source_app_version: String::from("1.2.3"),
            authenticated: true,
            access_schema: None,
            access_payload: None,
        }
    }
    fn distributor_session() -> DistributorSession {
        DistributorSession {
            id: String::from("id"),
            token: String::from("token"),
            account_id: String::from("account"),
            device_id: String::from("device"),
        }
    }

    #[tokio::test]
    pub async fn test_happy_badger() {
        let listener = std::net::TcpListener::bind("127.0.0.1:9998").unwrap();
        let mock_server = MockServer::builder().listener(listener).start().await;

        Mock::given(method("POST"))
            .and(path("/platco/dev"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let metrics_context = metrics_context();

        let _app_name = "foo".to_string();
        let _app_ver = "1.0.0".to_string();
        let _partner_id = "charter".to_string();
        let _custom_payload = custom_payload();

        let mut service = SiftService::new(
            "http://localhost:9998/platco/dev".to_string(),
            1,
            20,
            metrics_schemas(),
            None,
            Arc::new(Mutex::new(CircularBuffer::new(2))),
        );

        let context = behavioral_context();

        service
            .send_metric(
                AppBehavioralMetric::Ready(Ready {
                    context: context.clone(),
                    ttmu_ms: 30000,
                }),
                distributor_session(),
            )
            .await;

        let p = Param {
            name: "asdf".to_string(),
            value: "fda".to_string(),
        };
        let params = vec![p];

        service.update_metrics_context(Some(metrics_context)).await;

        let a = BadgerMetrics::AppAction(BadgerAppAction {
            context: context.clone(),
            action: "boom".to_string(),
            args: params.clone(),
        });
        let b = BadgerMetrics::AppAction(BadgerAppAction {
            context: context.clone(),
            action: "bam".to_string(),
            args: params.clone(),
        });
        let c = BadgerMetrics::AppAction(BadgerAppAction {
            context: context.clone(),
            action: "whamo".to_string(),
            args: params.clone(),
        });
        service.send_badger_metric(a, distributor_session()).await;
        service.send_badger_metric(b, distributor_session()).await;
        service.send_badger_metric(c, distributor_session()).await;

        assert_eq!(true, true);
    }
    #[tokio::test]
    pub async fn test_sad_badger() {
        let listener = std::net::TcpListener::bind("127.0.0.1:9999").unwrap();
        let mock_server = MockServer::builder().listener(listener).start().await;

        Mock::given(method("POST"))
            .and(path("/platco/dev"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&mock_server)
            .await;

        let metrics_context = metrics_context();

        let _app_name = "foo".to_string();
        let _app_ver = "1.0.0".to_string();
        let _partner_id = "charter".to_string();
        let _custom_payload = custom_payload();

        let mut service = SiftService::new(
            "http://localhost:9999/platco/dev".to_string(),
            1,
            20,
            metrics_schemas(),
            None,
            Arc::new(Mutex::new(CircularBuffer::new(2))),
        );

        let context = behavioral_context();

        service
            .send_metric(
                AppBehavioralMetric::Ready(Ready {
                    context: context.clone(),
                    ttmu_ms: 30000,
                }),
                distributor_session(),
            )
            .await;

        let p = Param {
            name: "asdf".to_string(),
            value: "fda".to_string(),
        };
        let params = vec![p];

        service.update_metrics_context(Some(metrics_context)).await;

        let a = BadgerMetrics::AppAction(BadgerAppAction {
            context: context.clone(),
            action: "boom".to_string(),
            args: params.clone(),
        });
        let b = BadgerMetrics::AppAction(BadgerAppAction {
            context: context.clone(),
            action: "bam".to_string(),
            args: params.clone(),
        });
        let c = BadgerMetrics::AppAction(BadgerAppAction {
            context: context.clone(),
            action: "whamo".to_string(),
            args: params.clone(),
        });
        service.send_badger_metric(a, distributor_session()).await;
        service.send_badger_metric(b, distributor_session()).await;
        service.send_badger_metric(c, distributor_session()).await;

        assert_eq!(true, true);
    }
    #[tokio::test]
    pub async fn test_happy_firebolt() {
        let listener = std::net::TcpListener::bind("127.0.0.1:6666").unwrap();
        let mock_server = MockServer::builder().listener(listener).start().await;

        Mock::given(method("POST"))
            .and(path("/platco/dev"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let metrics_context = metrics_context();

        let _app_name = "foo".to_string();
        let _app_ver = "1.0.0".to_string();
        let _partner_id = "charter".to_string();
        let _custom_payload = custom_payload();

        let mut service = SiftService::new(
            "http://localhost:6666/entos/dev".to_string(),
            1,
            20,
            metrics_schemas(),
            None,
            Arc::new(tokio::sync::Mutex::new(CircularBuffer::new(2))),
        );

        let context = behavioral_context();

        service
            .send_metric(
                AppBehavioralMetric::Ready(Ready {
                    context: context.clone(),
                    ttmu_ms: 30000,
                }),
                distributor_session(),
            )
            .await;

        let p = Param {
            name: "asdf".to_string(),
            value: "fda".to_string(),
        };
        let params = vec![p];

        service.update_metrics_context(Some(metrics_context)).await;

        service
            .send_metric(
                AppBehavioralMetric::Action(Action {
                    context: context.clone(),
                    category: CategoryType::User,
                    parameters: params.clone(),
                    _type: "user".to_string(),
                }),
                distributor_session(),
            )
            .await;
    }
    #[tokio::test]
    pub async fn test_sad_firebolt() {
        let listener = std::net::TcpListener::bind("127.0.0.1:6667").unwrap();
        let mock_server = MockServer::builder().listener(listener).start().await;

        Mock::given(method("POST"))
            .and(path("/platco/dev"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&mock_server)
            .await;

        let metrics_context = metrics_context();

        let _app_name = "foo".to_string();
        let _app_ver = "1.0.0".to_string();
        let _partner_id = "charter".to_string();
        let _custom_payload = custom_payload();

        let mut service = SiftService::new(
            "http://localhost:6667/platco/dev".to_string(),
            1,
            20,
            metrics_schemas(),
            None,
            Arc::new(Mutex::new(CircularBuffer::new(2))),
        );

        let context = behavioral_context();

        service
            .send_metric(
                AppBehavioralMetric::Ready(Ready {
                    context: context.clone(),
                    ttmu_ms: 30000,
                }),
                distributor_session(),
            )
            .await;

        let p = Param {
            name: "asdf".to_string(),
            value: "fda".to_string(),
        };
        let _params = vec![p];

        service.update_metrics_context(Some(metrics_context)).await;

        assert_eq!(true, true);
    }
}
