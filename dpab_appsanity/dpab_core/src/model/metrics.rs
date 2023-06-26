use std::collections::{HashMap, HashSet};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
//https://developer.comcast.com/firebolt/core/sdk/latest/api/metrics

trait AnalyticsIdentifier {
    fn to_string(self) -> String;
}
use serde_json::Value;

use crate::message::DistributorSession;
pub struct AppId {
    val: String,
}
impl From<String> for AppId {
    fn from(id: String) -> Self {
        AppId { val: id }
    }
}
impl AppId {
    pub fn new(app_id: String) -> AppId {
        AppId { val: app_id }
    }
    pub fn to_string(&self) -> String {
        self.val.clone()
    }
}
impl From<AppBehavioralMetric> for AppId {
    fn from(metric: AppBehavioralMetric) -> Self {
        let app_context: AppContext = metric.into();
        AppId::new(app_context.app_id)
    }
}

impl AnalyticsIdentifier for AppId {
    fn to_string(self) -> String {
        self.val
    }
}
impl From<AppBehavioralMetric> for AppContext {
    fn from(metric: AppBehavioralMetric) -> Self {
        match metric {
            AppBehavioralMetric::Ready(a) => a.context,
            AppBehavioralMetric::SignIn(a) => a.context,
            AppBehavioralMetric::SignOut(a) => a.context,
            AppBehavioralMetric::StartContent(a) => a.context,
            AppBehavioralMetric::StopContent(a) => a.context,
            AppBehavioralMetric::Page(a) => a.context,
            AppBehavioralMetric::Action(a) => a.context,
            AppBehavioralMetric::Error(a) => a.context,
            AppBehavioralMetric::MediaLoadStart(a) => a.context,
            AppBehavioralMetric::MediaPlay(a) => a.context,
            AppBehavioralMetric::MediaPlaying(a) => a.context,
            AppBehavioralMetric::MediaPause(a) => a.context,
            AppBehavioralMetric::MediaWaiting(a) => a.context,
            AppBehavioralMetric::MediaProgress(a) => a.context,
            AppBehavioralMetric::MediaSeeking(a) => a.context,
            AppBehavioralMetric::MediaSeeked(a) => a.context,
            AppBehavioralMetric::MediaRateChanged(a) => a.context,
            AppBehavioralMetric::MediaRenditionChanged(a) => a.context,
            AppBehavioralMetric::MediaEnded(a) => a.context,
            AppBehavioralMetric::AppStateChange(a) => a.context,
        }
    }
}
#[derive(Debug, Serialize, Deserialize, Clone, Default)]

pub struct AppDataGovernanceState {
    pub data_tags_to_apply: HashSet<String>,
}
impl AppDataGovernanceState {
    pub fn new(tags: HashSet<String>) -> AppDataGovernanceState {
        AppDataGovernanceState {
            data_tags_to_apply: tags,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]

pub struct AppContext {
    pub app_id: String,
    pub app_version: String,
    pub partner_id: String,
    pub app_session_id: String,
    pub app_user_session_id: Option<String>,
    pub durable_app_id: String,
    pub governance_state: Option<AppDataGovernanceState>,
}
#[async_trait]
pub trait MetricsContextProvider: core::fmt::Debug {
    async fn provide_context(&mut self) -> Option<DeviceMetricsContext>;
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Ready {
    #[serde(skip_serializing)]
    pub context: AppContext,
    pub ttmu_ms: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum MediaPositionType {
    None,
    PercentageProgress(f32),
    AbsolutePosition(i32),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SignIn {
    #[serde(skip_serializing)]
    pub context: AppContext,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SignOut {
    #[serde(skip_serializing)]
    pub context: AppContext,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StartContent {
    #[serde(skip_serializing)]
    pub context: AppContext,
    pub entity_id: Option<String>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StopContent {
    #[serde(skip_serializing)]
    pub context: AppContext,
    pub entity_id: Option<String>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Page {
    #[serde(skip_serializing)]
    pub context: AppContext,
    pub page_id: String,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ActionType {
    user,
    app,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum CategoryType {
    User,
    App,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Param {
    pub name: String,
    pub value: String,
}
pub fn hashmap_to_param_vec(the_map: Option<HashMap<String, String>>) -> Vec<Param> {
    let mut result = Vec::new();
    if the_map.is_none() {
        return vec![];
    };

    let params_map = the_map.unwrap();

    for (key, value) in params_map {
        result.push(Param { name: key, value });
    }
    result
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Action {
    #[serde(skip_serializing)]
    pub context: AppContext,
    pub category: CategoryType,
    #[serde(rename = "type")]
    pub _type: String,
    pub parameters: Vec<Param>,
}
#[allow(non_camel_case_types)]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ErrorType {
    network,
    media,
    restriction,
    entitlement,
    other,
}

#[derive(Deserialize, Debug, Clone, Serialize)]
pub struct SemanticVersion {
    pub version: Version,
}

#[derive(Deserialize, Debug, Clone, Serialize, PartialEq)]
pub struct Version {
    pub major: i8,
    pub minor: i8,
    pub patch: i8,
    pub readable: String,
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "foo")
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppError {
    #[serde(skip_serializing)]
    pub context: AppContext,
    pub app_session_id: Option<String>,
    pub app_user_session_id: Option<String>,
    pub durable_app_id: String,
    pub third_party_error: bool,
    #[serde(rename = "type")]
    pub error_type: ErrorType,
    pub code: String,
    pub description: String,
    pub visible: bool,
    pub parameters: Option<Vec<Param>>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MediaLoadStart {
    #[serde(skip_serializing)]
    pub context: AppContext,
    pub entity_id: String,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MediaPlay {
    #[serde(skip_serializing)]
    pub context: AppContext,
    pub entity_id: String,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MediaPlaying {
    #[serde(skip_serializing)]
    pub context: AppContext,
    pub entity_id: String,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MediaPause {
    #[serde(skip_serializing)]
    pub context: AppContext,
    pub entity_id: String,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MediaWaiting {
    #[serde(skip_serializing)]
    pub context: AppContext,
    pub entity_id: String,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MediaProgress {
    #[serde(skip_serializing)]
    pub context: AppContext,
    pub entity_id: String,
    pub progress: Option<MediaPositionType>,
}
impl From<&MediaProgress> for Option<i32> {
    fn from(progress: &MediaProgress) -> Self {
        match progress.progress.clone() {
            Some(prog) => prog.as_absolute(),
            None => None,
        }
    }
}

impl From<&MediaProgress> for Option<f32> {
    fn from(progress: &MediaProgress) -> Self {
        match progress.progress.clone() {
            Some(prog) => prog.as_percentage(),
            None => None,
        }
    }
}

impl MediaPositionType {
    fn as_absolute(self) -> Option<i32> {
        match self {
            MediaPositionType::None => None,
            MediaPositionType::PercentageProgress(_) => None,
            MediaPositionType::AbsolutePosition(absolute) => Some(absolute),
        }
    }
    fn as_percentage(self) -> Option<f32> {
        match self {
            MediaPositionType::None => None,
            MediaPositionType::PercentageProgress(percentage) => Some(percentage),
            MediaPositionType::AbsolutePosition(_) => None,
        }
    }
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MediaSeeking {
    #[serde(skip_serializing)]
    pub context: AppContext,
    pub entity_id: String,
    pub target: Option<MediaPositionType>,
}

impl From<&MediaSeeking> for Option<i32> {
    fn from(progress: &MediaSeeking) -> Self {
        match progress.target.clone() {
            Some(prog) => prog.as_absolute(),
            None => None,
        }
    }
}

impl From<&MediaSeeking> for Option<f32> {
    fn from(progress: &MediaSeeking) -> Self {
        match progress.target.clone() {
            Some(prog) => prog.as_percentage(),
            None => None,
        }
    }
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MediaSeeked {
    #[serde(skip_serializing)]
    pub context: AppContext,
    pub entity_id: String,
    pub position: Option<MediaPositionType>,
}

impl From<&MediaSeeked> for Option<i32> {
    fn from(progress: &MediaSeeked) -> Self {
        match progress.position.clone() {
            Some(prog) => prog.as_absolute(),
            None => None,
        }
    }
}

impl From<&MediaSeeked> for Option<f32> {
    fn from(progress: &MediaSeeked) -> Self {
        match progress.position.clone() {
            Some(prog) => prog.as_percentage(),
            None => None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MediaRateChanged {
    #[serde(skip_serializing)]
    pub context: AppContext,
    pub entity_id: String,
    pub rate: u32,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MediaRenditionChanged {
    #[serde(skip_serializing)]
    pub context: AppContext,
    pub entity_id: String,
    pub bitrate: u32,
    pub width: u32,
    pub height: u32,
    pub profile: Option<String>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MediaEnded {
    #[serde(skip_serializing)]
    pub context: AppContext,
    pub entity_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AppBehavioralMetric {
    Ready(Ready),
    SignIn(SignIn),
    SignOut(SignOut),
    StartContent(StartContent),
    StopContent(StopContent),
    Page(Page),
    Action(Action),
    Error(AppError),
    MediaLoadStart(MediaLoadStart),
    MediaPlay(MediaPlay),
    MediaPlaying(MediaPlaying),
    MediaPause(MediaPause),
    MediaWaiting(MediaWaiting),
    MediaProgress(MediaProgress),
    MediaSeeking(MediaSeeking),
    MediaSeeked(MediaSeeked),
    MediaRateChanged(MediaRateChanged),
    MediaRenditionChanged(MediaRenditionChanged),
    MediaEnded(MediaEnded),
    AppStateChange(AppLifecycleStateChange),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AppLifecycleState {
    #[serde(rename = "launching")]
    Launching,
    #[serde(rename = "foreground")]
    Foreground,
    #[serde(rename = "background")]
    Background,
    #[serde(rename = "inactive")]
    Inactive,
    #[serde(rename = "suspended")]
    Suspended,
    #[serde(rename = "not_running")]
    NotRunning,
    #[serde(rename = "initializing")]
    Initializing,
    #[serde(rename = "ready")]
    Ready,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppLifecycleStateChange {
    pub context: AppContext,
    pub previous_state: Option<AppLifecycleState>,
    pub new_state: AppLifecycleState,
}

/// all the things that are provided by platform that need to
/// be updated, and eventually in/outjected into/out of a payload
/// These items may (or may not) be available when the ripple
/// process starts, so this service may need a way to wait for the values
/// to become available
/// This design assumes that all of the items will be available at the same times
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct DeviceMetricsContext {
    pub device_language: String,
    pub device_model: String,
    pub device_id: String,
    pub account_id: String,
    pub device_timezone: String,
    pub device_name: String,
    pub platform: String,
    pub os_ver: String,
    pub distribution_tenant_id: String,
    pub device_session_id: String,
    pub mac_address: String,
    pub serial_number: String,
}

#[allow(non_camel_case_types)]
pub enum DeviceMetricsContextField {
    device_language,
    device_model,
    device_id,
    account_id,
    device_timezone,
    device_name,
    platform,
    os_ver,
    device_session_id,
    distribution_tenant_id,
    mac_address,
    serial_number,
}
impl DeviceMetricsContext {
    pub fn new() -> DeviceMetricsContext {
        DeviceMetricsContext {
            device_language: String::from(""),
            device_model: String::from(""),
            device_id: String::from(""),
            device_timezone: String::from("GMT"),
            device_name: String::from(""),
            mac_address: String::from(""),
            serial_number: String::from(""),
            account_id: String::from(""),
            platform: String::from(""),
            os_ver: String::from(""),
            device_session_id: String::from(""),
            distribution_tenant_id: String::from(""),
        }
    }
    pub fn set(&mut self, field: DeviceMetricsContextField, value: String) {
        match field {
            DeviceMetricsContextField::device_language => self.device_language = value,
            DeviceMetricsContextField::device_model => self.device_model = value,
            DeviceMetricsContextField::device_id => self.device_id = value,
            DeviceMetricsContextField::account_id => self.account_id = value,
            DeviceMetricsContextField::device_timezone => self.device_timezone = value,
            DeviceMetricsContextField::platform => self.platform = value,
            DeviceMetricsContextField::os_ver => self.os_ver = value,
            DeviceMetricsContextField::device_session_id => self.device_session_id = value,
            DeviceMetricsContextField::mac_address => self.mac_address = value,
            DeviceMetricsContextField::serial_number => self.serial_number = value,
            DeviceMetricsContextField::device_name => self.device_name = value,
            DeviceMetricsContextField::distribution_tenant_id => {
                self.distribution_tenant_id = value
            }
        };
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BadgerMetric {
    #[serde(skip_serializing)]
    pub context: AppContext,
    pub segment: Option<String>,
    pub args: Option<Vec<Param>>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BadgerAppAction {
    #[serde(skip_serializing)]
    pub context: AppContext,
    pub action: String,
    pub args: Vec<Param>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BadgerError {
    #[serde(skip_serializing)]
    pub context: AppContext,
    pub message: String,
    pub visible: bool,
    pub code: u16,
    pub args: Option<Vec<Param>>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BadgerLaunchCompleted {
    pub context: AppContext,
    pub args: Option<Vec<Param>>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BadgerDismissLoadingScreen {
    pub context: AppContext,
    pub args: Option<Vec<Param>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BadgerPageView {
    pub context: AppContext,
    pub page: String,
    pub args: Option<Vec<Param>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BadgerUserAction {
    pub context: AppContext,
    pub args: Option<Vec<Param>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BadgerUserError {
    #[serde(skip_serializing)]
    pub context: AppContext,
    pub message: String,
    pub visible: bool,
    pub code: u16,
    pub args: Option<Vec<Param>>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum BadgerMetrics {
    Metric(BadgerMetric),
    AppAction(BadgerAppAction),
    Error(BadgerError),
    LaunchCompleted(BadgerLaunchCompleted),
    DismissLoadingScreen(BadgerDismissLoadingScreen),
    PageView(BadgerPageView),
    UserAction(BadgerUserAction),
    UserError(BadgerUserError),
}

#[async_trait]
pub trait BehavioralMetricsService {
    async fn send_metric(
        &mut self,
        metrics: AppBehavioralMetric,
        session: DistributorSession,
    ) -> ();
}
#[async_trait]
pub trait BadgerMetricsService {
    async fn send_badger_metric(
        &mut self,
        metrics: BadgerMetrics,
        session: DistributorSession,
    ) -> ();
}
#[async_trait]
pub trait ContextualMetricsService {
    async fn update_metrics_context(&mut self, new_context: Option<DeviceMetricsContext>) -> ();
}
#[async_trait]
pub trait MetricsManager: Send + Sync {
    async fn send_metric(&mut self, metrics: AppBehavioralMetric) -> ();
    async fn send_badger_metric(&mut self, metrics: BadgerMetrics) -> ();
}
pub struct LoggingBehavioralMetricsManager {
    pub metrics_context: Option<DeviceMetricsContext>,
    pub log_fn: LogFn,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
struct LoggableBehavioralMetric {
    context: Option<DeviceMetricsContext>,
    payload: AppBehavioralMetric,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
struct LoggableBadgerMetric {
    context: Option<DeviceMetricsContext>,
    payload: BadgerMetrics,
}
type LogFn = fn(&Value) -> ();
impl LoggingBehavioralMetricsManager {
    pub fn new(
        metrics_context: Option<DeviceMetricsContext>,
        log_fn: LogFn,
    ) -> LoggingBehavioralMetricsManager {
        LoggingBehavioralMetricsManager {
            metrics_context,
            log_fn,
        }
    }
}
impl LoggingBehavioralMetricsManager {
    pub async fn send_metric(&mut self, metrics: AppBehavioralMetric) {
        (self.log_fn)(
            &serde_json::to_value(&LoggableBehavioralMetric {
                context: self.metrics_context.clone(),
                payload: metrics,
            })
            .unwrap(),
        );
    }
    pub async fn send_badger_metric(&mut self, metrics: BadgerMetrics) {
        (self.log_fn)(
            &serde_json::to_value(&LoggableBadgerMetric {
                context: self.metrics_context.clone(),
                payload: metrics,
            })
            .unwrap(),
        );
    }
}
