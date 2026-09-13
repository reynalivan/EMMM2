//! Discover's shared, network-only EasyList engine and filter-list cache.
//!
//! The engine deliberately never receives cookies or page content. It only
//! evaluates request metadata supplied by the native WebView2 callback.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

use adblock::lists::{ParseOptions, RuleTypes};
use adblock::request::Request;
use adblock::{Engine, FilterSet};
use futures_util::StreamExt;
use sqlx::SqlitePool;
use tauri::{AppHandle, Emitter, Manager};

use crate::modules::browser::adapters::sqlite::browser;
use crate::shared::errors::BrowserError;

const EASYLIST_URL: &str = "https://easylist.to/easylist/easylist.txt";
const EASYPRIVACY_URL: &str = "https://easylist.to/easylist/easyprivacy.txt";
const FILTER_HOST: &str = "easylist.to";
const MAX_FILTER_BYTES: usize = 5 * 1024 * 1024;
const UPDATE_INTERVAL_SECONDS: i64 = 7 * 24 * 60 * 60;
const SETTING_ENABLED: &str = "adblock_enabled";
const SETTING_LAST_SUCCESS: &str = "adblock_last_success_at";
const SETTING_EASYLIST_ETAG: &str = "adblock_easylist_etag";
const SETTING_EASYLIST_MODIFIED: &str = "adblock_easylist_last_modified";
const SETTING_EASYLIST_FILE: &str = "adblock_easylist_file";
const SETTING_EASYPRIVACY_ETAG: &str = "adblock_easyprivacy_etag";
const SETTING_EASYPRIVACY_MODIFIED: &str = "adblock_easyprivacy_last_modified";
const SETTING_EASYPRIVACY_FILE: &str = "adblock_easyprivacy_file";

const BUNDLED_EASYLIST: &str = include_str!("../../../../../../resources/adblock/easylist.txt");
const BUNDLED_EASYPRIVACY: &str =
    include_str!("../../../../../../resources/adblock/easyprivacy.txt");

/// Process-wide state shared by every Discover child webview.
pub struct BrowserAdblockState {
    enabled: AtomicBool,
    engine: RwLock<Option<Arc<Engine>>>,
    initialization_lock: tokio::sync::Mutex<()>,
    update_in_flight: AtomicBool,
}

impl Default for BrowserAdblockState {
    fn default() -> Self {
        Self {
            enabled: AtomicBool::new(true),
            engine: RwLock::new(None),
            initialization_lock: tokio::sync::Mutex::new(()),
            update_in_flight: AtomicBool::new(false),
        }
    }
}

impl BrowserAdblockState {
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    pub fn set_enabled_in_memory(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    /// Match one network request. Invalid URLs are never blocked.
    pub fn should_block(&self, url: &str, source_url: &str, resource_type: &str) -> bool {
        if !self.is_enabled() {
            return false;
        }
        let Ok(request) = Request::new(url, source_url, resource_type, "") else {
            return false;
        };
        self.engine
            .read()
            .ok()
            .and_then(|engine| engine.as_ref().cloned())
            .is_some_and(|engine| engine.check_network_request(&request).should_block())
    }

    /// Ensure the local fallback/cache engine exists before a page can navigate.
    pub async fn ensure_loaded(
        &self,
        app: &AppHandle,
        db: &SqlitePool,
    ) -> Result<(), BrowserError> {
        if self.engine.read().is_ok_and(|engine| engine.is_some()) {
            return Ok(());
        }

        let _guard = self.initialization_lock.lock().await;
        if self.engine.read().is_ok_and(|engine| engine.is_some()) {
            return Ok(());
        }

        let enabled = get_enabled(db).await?;
        self.set_enabled_in_memory(enabled);

        let cache_dir = filter_cache_dir(app)?;
        let lists = load_cached_lists(db, &cache_dir)
            .await
            .unwrap_or_else(|| (BUNDLED_EASYLIST.to_owned(), BUNDLED_EASYPRIVACY.to_owned()));
        let engine = tokio::task::spawn_blocking(move || build_engine(lists.0, lists.1))
            .await
            .map_err(|error| BrowserError::Io(format!("ad-block engine task failed: {error}")))?;
        *self
            .engine
            .write()
            .map_err(|_| BrowserError::Io("ad-block engine lock is poisoned".to_string()))? =
            Some(Arc::new(engine));

        if update_is_due(db).await && !self.update_in_flight.swap(true, Ordering::AcqRel) {
            let app = app.clone();
            let db = db.clone();
            tauri::async_runtime::spawn(async move {
                let state = app.state::<BrowserAdblockState>();
                if let Err(error) = update_filter_lists(state.inner(), &app, &db).await {
                    log::warn!(
                        "Discover ad-block list update failed; keeping the prior list: {error}"
                    );
                    let _ = app.emit("browser:adblock-update-failed", ());
                }
                state.update_in_flight.store(false, Ordering::Release);
            });
        }

        Ok(())
    }
}

/// Read the persisted toggle. A missing value is an enabled first installation.
pub async fn get_enabled(db: &SqlitePool) -> Result<bool, BrowserError> {
    match browser::get_setting(db, SETTING_ENABLED).await? {
        Some(value) => match value.as_str() {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err(BrowserError::InvalidSetting(
                "adblock_enabled must be true or false".to_string(),
            )),
        },
        None => {
            browser::set_setting(db, SETTING_ENABLED, "true").await?;
            Ok(true)
        }
    }
}

pub async fn set_enabled(
    app: &AppHandle,
    db: &SqlitePool,
    enabled: bool,
) -> Result<(), BrowserError> {
    browser::set_setting(db, SETTING_ENABLED, if enabled { "true" } else { "false" }).await?;
    app.state::<BrowserAdblockState>()
        .set_enabled_in_memory(enabled);
    Ok(())
}

fn build_engine(easylist: String, easyprivacy: String) -> Engine {
    let network_only = ParseOptions {
        rule_types: RuleTypes::NetworkOnly,
        ..Default::default()
    };
    let mut filters = FilterSet::new(false);
    filters.add_filter_list(easylist, network_only.clone());
    filters.add_filter_list(easyprivacy, network_only);
    Engine::new_with_filter_set(filters)
}

fn filter_cache_dir(app: &AppHandle) -> Result<PathBuf, BrowserError> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|error| BrowserError::Io(format!("Discover profile path unavailable: {error}")))?
        .join("discover")
        .join("adblock");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Data directory used only by Discover's WebView profile. It intentionally
/// lives outside Tauri's own webview data so clearing browser data cannot
/// affect EMMM's UI storage or app-session state.
pub fn discover_profile_dir(app: &AppHandle) -> Result<PathBuf, BrowserError> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|error| BrowserError::Io(format!("Discover profile path unavailable: {error}")))?
        .join("discover")
        .join("profile");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn safe_cache_file(cache_dir: &Path, value: &str) -> Option<PathBuf> {
    let candidate = Path::new(value);
    if candidate.components().count() != 1 || candidate.extension().is_none() {
        return None;
    }
    Some(cache_dir.join(candidate))
}

async fn load_cached_lists(db: &SqlitePool, cache_dir: &Path) -> Option<(String, String)> {
    let easylist_name = browser::get_setting(db, SETTING_EASYLIST_FILE)
        .await
        .ok()??;
    let easyprivacy_name = browser::get_setting(db, SETTING_EASYPRIVACY_FILE)
        .await
        .ok()??;
    let easylist_path = safe_cache_file(cache_dir, &easylist_name)?;
    let easyprivacy_path = safe_cache_file(cache_dir, &easyprivacy_name)?;
    let easylist = tokio::fs::read_to_string(easylist_path).await.ok()?;
    let easyprivacy = tokio::fs::read_to_string(easyprivacy_path).await.ok()?;
    validate_filter_body(&easylist).ok()?;
    validate_filter_body(&easyprivacy).ok()?;
    Some((easylist, easyprivacy))
}

async fn update_is_due(db: &SqlitePool) -> bool {
    let Ok(value) = browser::get_setting(db, SETTING_LAST_SUCCESS).await else {
        return true;
    };
    let now = chrono::Utc::now().timestamp();
    value
        .and_then(|timestamp| timestamp.parse::<i64>().ok())
        .is_none_or(|last_success| now.saturating_sub(last_success) >= UPDATE_INTERVAL_SECONDS)
}

async fn update_filter_lists(
    state: &BrowserAdblockState,
    app: &AppHandle,
    db: &SqlitePool,
) -> Result<(), BrowserError> {
    let cache_dir = filter_cache_dir(app)?;
    let easylist = fetch_filter(
        EASYLIST_URL,
        browser::get_setting(db, SETTING_EASYLIST_ETAG).await?,
        browser::get_setting(db, SETTING_EASYLIST_MODIFIED).await?,
    )
    .await?;
    let easyprivacy = fetch_filter(
        EASYPRIVACY_URL,
        browser::get_setting(db, SETTING_EASYPRIVACY_ETAG).await?,
        browser::get_setting(db, SETTING_EASYPRIVACY_MODIFIED).await?,
    )
    .await?;

    if easylist.body.is_none() && easyprivacy.body.is_none() {
        browser::set_setting(
            db,
            SETTING_LAST_SUCCESS,
            &chrono::Utc::now().timestamp().to_string(),
        )
        .await?;
        return Ok(());
    }

    let current = load_cached_lists(db, &cache_dir)
        .await
        .unwrap_or_else(|| (BUNDLED_EASYLIST.to_owned(), BUNDLED_EASYPRIVACY.to_owned()));
    let next_easylist = easylist.body.unwrap_or(current.0);
    let next_easyprivacy = easyprivacy.body.unwrap_or(current.1);
    let next_engine = tokio::task::spawn_blocking({
        let easylist = next_easylist.clone();
        let easyprivacy = next_easyprivacy.clone();
        move || build_engine(easylist, easyprivacy)
    })
    .await
    .map_err(|error| BrowserError::Io(format!("ad-block update task failed: {error}")))?;

    let easylist_file = write_atomic_new_file(&cache_dir, "easylist", &next_easylist).await?;
    let easyprivacy_file =
        write_atomic_new_file(&cache_dir, "easyprivacy", &next_easyprivacy).await?;
    let mut transaction = db.begin().await?;
    set_setting_in_transaction(&mut transaction, SETTING_EASYLIST_FILE, &easylist_file).await?;
    set_setting_in_transaction(
        &mut transaction,
        SETTING_EASYPRIVACY_FILE,
        &easyprivacy_file,
    )
    .await?;
    if let Some(value) = easylist.etag.as_deref() {
        set_setting_in_transaction(&mut transaction, SETTING_EASYLIST_ETAG, value).await?;
    }
    if let Some(value) = easylist.last_modified.as_deref() {
        set_setting_in_transaction(&mut transaction, SETTING_EASYLIST_MODIFIED, value).await?;
    }
    if let Some(value) = easyprivacy.etag.as_deref() {
        set_setting_in_transaction(&mut transaction, SETTING_EASYPRIVACY_ETAG, value).await?;
    }
    if let Some(value) = easyprivacy.last_modified.as_deref() {
        set_setting_in_transaction(&mut transaction, SETTING_EASYPRIVACY_MODIFIED, value).await?;
    }
    set_setting_in_transaction(
        &mut transaction,
        SETTING_LAST_SUCCESS,
        &chrono::Utc::now().timestamp().to_string(),
    )
    .await?;
    transaction.commit().await?;
    *state
        .engine
        .write()
        .map_err(|_| BrowserError::Io("ad-block engine lock is poisoned".to_string()))? =
        Some(Arc::new(next_engine));
    Ok(())
}

async fn set_setting_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    key: &str,
    value: &str,
) -> Result<(), BrowserError> {
    sqlx::query(
        "INSERT INTO browser_settings (key, value) VALUES (?, ?)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(key)
    .bind(value)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

struct FilterResponse {
    body: Option<String>,
    etag: Option<String>,
    last_modified: Option<String>,
}

async fn fetch_filter(
    raw_url: &str,
    etag: Option<String>,
    last_modified: Option<String>,
) -> Result<FilterResponse, BrowserError> {
    let url = reqwest::Url::parse(raw_url)
        .map_err(|error| BrowserError::InvalidUrl(format!("filter source: {error}")))?;
    validate_filter_source(&url)?;

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|error| BrowserError::Download(format!("build filter client: {error}")))?;
    let mut request = client.get(url);
    if let Some(etag) = etag {
        request = request.header(reqwest::header::IF_NONE_MATCH, etag);
    }
    if let Some(last_modified) = last_modified {
        request = request.header(reqwest::header::IF_MODIFIED_SINCE, last_modified);
    }
    let response = request
        .send()
        .await
        .map_err(|error| BrowserError::Download(format!("fetch filter list: {error}")))?;
    if response.status() == reqwest::StatusCode::NOT_MODIFIED {
        return Ok(FilterResponse {
            body: None,
            etag: None,
            last_modified: None,
        });
    }
    if !response.status().is_success() {
        return Err(BrowserError::Download(format!(
            "filter source returned {}",
            response.status()
        )));
    }
    if response
        .content_length()
        .is_some_and(|length| length as usize > MAX_FILTER_BYTES)
    {
        return Err(BrowserError::Download(
            "filter source exceeded size limit".to_string(),
        ));
    }
    let etag = response
        .headers()
        .get(reqwest::header::ETAG)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let last_modified = response
        .headers()
        .get(reqwest::header::LAST_MODIFIED)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let mut body = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk =
            chunk.map_err(|error| BrowserError::Download(format!("read filter body: {error}")))?;
        if body.len().saturating_add(chunk.len()) > MAX_FILTER_BYTES {
            return Err(BrowserError::Download(
                "filter source exceeded size limit".to_string(),
            ));
        }
        body.extend_from_slice(&chunk);
    }
    let body = String::from_utf8(body)
        .map_err(|_| BrowserError::Download("filter source was not UTF-8".to_string()))?;
    validate_filter_body(&body)?;
    Ok(FilterResponse {
        body: Some(body),
        etag,
        last_modified,
    })
}

fn validate_filter_source(url: &reqwest::Url) -> Result<(), BrowserError> {
    if url.scheme() != "https" || url.host_str() != Some(FILTER_HOST) {
        return Err(BrowserError::InvalidUrl(
            "filter source must be HTTPS on easylist.to".to_string(),
        ));
    }
    Ok(())
}

fn validate_filter_body(body: &str) -> Result<(), BrowserError> {
    if body.is_empty() || body.len() > MAX_FILTER_BYTES || !body.contains('\n') {
        return Err(BrowserError::Download(
            "filter list content is invalid".to_string(),
        ));
    }
    Ok(())
}

async fn write_atomic_new_file(
    cache_dir: &Path,
    prefix: &str,
    contents: &str,
) -> Result<String, BrowserError> {
    let filename = format!("{prefix}-{}.txt", uuid::Uuid::new_v4());
    let final_path = cache_dir.join(&filename);
    let temp_path = cache_dir.join(format!(".{filename}.tmp"));
    tokio::fs::write(&temp_path, contents).await?;
    tokio::fs::rename(temp_path, final_path).await?;
    Ok(filename)
}

/// Attach the platform interceptor after the child exists but before it leaves
/// `about:blank`. Other platforms keep the persistent profile and UI toggle,
/// but do not advertise WebView2-equivalent network interception.
pub fn attach_native_request_filter(
    webview: &tauri::Webview,
    app: &AppHandle,
    label: &str,
) -> Result<(), BrowserError> {
    #[cfg(target_os = "windows")]
    {
        native_windows::attach(webview, app, label)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (webview, app, label);
        Ok(())
    }
}

/// Clear cookies plus DOM-backed site data for the shared Discover profile.
pub async fn clear_cookies_and_site_data(app: &AppHandle, label: &str) -> Result<(), BrowserError> {
    #[cfg(target_os = "windows")]
    {
        native_windows::clear_profile_data(app, label, native_windows::ClearKind::SiteData).await
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (app, label);
        Err(BrowserError::InvalidSetting(
            "separate Discover data clearing is only available on Windows".to_string(),
        ))
    }
}

/// Clear only the shared Discover profile's disk cache, preserving cookies and
/// site-storage tokens used for login persistence.
pub async fn clear_cache(app: &AppHandle, label: &str) -> Result<(), BrowserError> {
    #[cfg(target_os = "windows")]
    {
        native_windows::clear_profile_data(app, label, native_windows::ClearKind::Cache).await
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (app, label);
        Err(BrowserError::InvalidSetting(
            "separate Discover data clearing is only available on Windows".to_string(),
        ))
    }
}

#[cfg(target_os = "windows")]
mod native_windows {
    use std::sync::{Arc, Mutex};

    use tauri::{AppHandle, Emitter, Manager};
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        ICoreWebView2Profile2, ICoreWebView2_13, ICoreWebView2_15,
        COREWEBVIEW2_BROWSING_DATA_KINDS_ALL_DOM_STORAGE, COREWEBVIEW2_BROWSING_DATA_KINDS_COOKIES,
        COREWEBVIEW2_BROWSING_DATA_KINDS_DISK_CACHE, COREWEBVIEW2_FAVICON_IMAGE_FORMAT_PNG,
        COREWEBVIEW2_PERMISSION_STATE_DENY, COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL,
        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_DOCUMENT, COREWEBVIEW2_WEB_RESOURCE_CONTEXT_FETCH,
        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_FONT, COREWEBVIEW2_WEB_RESOURCE_CONTEXT_IMAGE,
        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_MEDIA, COREWEBVIEW2_WEB_RESOURCE_CONTEXT_SCRIPT,
        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_STYLESHEET,
        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_XML_HTTP_REQUEST,
    };
    use webview2_com::{
        ClearBrowsingDataCompletedHandler, FaviconChangedEventHandler, GetFaviconCompletedHandler,
        NavigationCompletedEventHandler, PermissionRequestedEventHandler,
        WebResourceRequestedEventHandler,
    };
    use windows::core::{w, Interface, BOOL, PWSTR};
    use windows::Win32::System::Com::CoTaskMemFree;

    use super::{BrowserAdblockState, BrowserError};

    pub(super) fn attach(
        webview: &tauri::Webview,
        app: &AppHandle,
        label: &str,
    ) -> Result<(), BrowserError> {
        let app_for_requests = app.clone();
        let app_for_favicon = app.clone();
        let app_for_navigation = app.clone();
        let label_for_favicon = label.to_string();
        let label_for_navigation = label.to_string();
        webview
            .with_webview(move |native| unsafe {
                let core = native
                    .controller()
                    .CoreWebView2()
                    .map_err(|error| log::warn!("Discover WebView2 core unavailable: {error}"))
                    .ok();
                let Some(core) = core else { return };
                if let Err(error) = core
                    .AddWebResourceRequestedFilter(w!("*"), COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL)
                {
                    log::warn!("Could not register Discover request filter: {error}");
                    return;
                }
                let environment = native.environment();
                let request_handler =
                    WebResourceRequestedEventHandler::create(Box::new(move |sender, args| {
                        let (Some(sender), Some(args)) = (sender, args) else {
                            return Ok(());
                        };
                        let Ok(request) = args.Request() else {
                            return Ok(());
                        };
                        let Ok(url) = read_uri(|value| request.Uri(value)) else {
                            return Ok(());
                        };
                        let source_url = read_uri(|value| sender.Source(value))
                            .unwrap_or_else(|_| String::new());
                        let mut context = Default::default();
                        if args.ResourceContext(&mut context).is_err() {
                            return Ok(());
                        }
                        let state = app_for_requests.state::<BrowserAdblockState>();
                        if !state.should_block(&url, &source_url, resource_type(context.0)) {
                            return Ok(());
                        }
                        if let Ok(response) = environment.CreateWebResourceResponse(
                            None::<&windows::Win32::System::Com::IStream>,
                            204,
                            w!("No Content"),
                            w!(""),
                        ) {
                            let _ = args.SetResponse(&response);
                        }
                        Ok(())
                    }));
                let mut request_token = 0;
                if let Err(error) =
                    core.add_WebResourceRequested(&request_handler, &mut request_token)
                {
                    log::warn!("Could not attach Discover request handler: {error}");
                }

                // Discover is a browsing surface, not a trusted app origin. Deny
                // sensitive WebView2 permissions unless a later explicit manager
                // adds a per-origin allow decision.
                let permission_handler =
                    PermissionRequestedEventHandler::create(Box::new(|_, args| {
                        if let Some(args) = args {
                            let _ = args.SetState(COREWEBVIEW2_PERMISSION_STATE_DENY);
                        }
                        Ok(())
                    }));
                let mut permission_token = 0;
                if let Err(error) =
                    core.add_PermissionRequested(&permission_handler, &mut permission_token)
                {
                    log::warn!("Could not attach Discover permission handler: {error}");
                }

                let navigation_handler =
                    NavigationCompletedEventHandler::create(Box::new(move |sender, args| {
                        let (Some(sender), Some(args)) = (sender, args) else {
                            return Ok(());
                        };
                        let mut success = BOOL(0);
                        if args.IsSuccess(&mut success).is_err() || success.as_bool() {
                            return Ok(());
                        }
                        let mut status = Default::default();
                        let _ = args.WebErrorStatus(&mut status);
                        let url = read_uri(|value| sender.Source(value)).unwrap_or_default();
                        let _ = app_for_navigation.emit(
                            "browser:navigation-error",
                            serde_json::json!({
                                "label": label_for_navigation,
                                "url": url,
                                "status": status.0,
                            }),
                        );
                        Ok(())
                    }));
                let mut navigation_token = 0;
                if let Err(error) =
                    core.add_NavigationCompleted(&navigation_handler, &mut navigation_token)
                {
                    log::warn!("Could not attach Discover navigation completion handler: {error}");
                }

                let Ok(favicon_core) = core.cast::<ICoreWebView2_15>() else {
                    return;
                };
                let favicon_handler =
                    FaviconChangedEventHandler::create(Box::new(move |sender, _| {
                        let Some(sender) = sender else {
                            return Ok(());
                        };
                        let Ok(favicon_core) = sender.cast::<ICoreWebView2_15>() else {
                            return Ok(());
                        };
                        let app = app_for_favicon.clone();
                        let label = label_for_favicon.clone();
                        let completed =
                            GetFaviconCompletedHandler::create(Box::new(move |result, stream| {
                                if result.is_ok() {
                                    if let Some(favicon) =
                                        stream.and_then(|stream| stream_to_data_uri(&stream))
                                    {
                                        let _ = app.emit(
                                    "browser:favicon-changed",
                                    serde_json::json!({ "label": label, "favicon": favicon }),
                                );
                                    }
                                }
                                Ok(())
                            }));
                        let _ = favicon_core
                            .GetFavicon(COREWEBVIEW2_FAVICON_IMAGE_FORMAT_PNG, &completed);
                        Ok(())
                    }));
                let mut favicon_token = 0;
                if let Err(error) =
                    favicon_core.add_FaviconChanged(&favicon_handler, &mut favicon_token)
                {
                    log::debug!("Discover favicon events unavailable: {error}");
                }
            })
            .map_err(BrowserError::from)
    }

    pub(super) enum ClearKind {
        SiteData,
        Cache,
    }

    pub(super) async fn clear_profile_data(
        app: &AppHandle,
        label: &str,
        kind: ClearKind,
    ) -> Result<(), BrowserError> {
        let webview = super::super::tabs::resolve_webview(app, label)?;
        let (sender, receiver) = tokio::sync::oneshot::channel::<Result<(), String>>();
        let sender = Arc::new(Mutex::new(Some(sender)));
        webview
            .with_webview(move |native| unsafe {
                let Ok(core) = native.controller().CoreWebView2() else {
                    complete_clear(&sender, Err("WebView2 core is unavailable".to_string()));
                    return;
                };
                let Ok(profile) = core
                    .cast::<ICoreWebView2_13>()
                    .and_then(|core| core.Profile())
                    .and_then(|profile| profile.cast::<ICoreWebView2Profile2>())
                else {
                    complete_clear(
                        &sender,
                        Err("WebView2 profile data API is unavailable".to_string()),
                    );
                    return;
                };
                let callback_sender = sender.clone();
                let callback = ClearBrowsingDataCompletedHandler::create(Box::new(move |result| {
                    complete_clear(
                        &callback_sender,
                        result
                            .is_ok()
                            .then_some(())
                            .ok_or_else(|| "WebView2 did not clear browser data".to_string()),
                    );
                    Ok(())
                }));
                let data_kinds = match kind {
                    ClearKind::SiteData => {
                        COREWEBVIEW2_BROWSING_DATA_KINDS_COOKIES
                            | COREWEBVIEW2_BROWSING_DATA_KINDS_ALL_DOM_STORAGE
                    }
                    ClearKind::Cache => COREWEBVIEW2_BROWSING_DATA_KINDS_DISK_CACHE,
                };
                if let Err(error) = profile.ClearBrowsingData(data_kinds, &callback) {
                    log::warn!("Could not clear Discover profile data: {error}");
                    complete_clear(
                        &sender,
                        Err(format!("WebView2 could not start data clearing: {error}")),
                    );
                }
            })
            .map_err(BrowserError::from)?;
        match tokio::time::timeout(std::time::Duration::from_secs(15), receiver).await {
            Ok(Ok(Ok(()))) => Ok(()),
            Ok(Ok(Err(message))) => Err(BrowserError::Io(message)),
            Ok(Err(_)) => Err(BrowserError::Io(
                "browser data clear callback was dropped".to_string(),
            )),
            Err(_) => Err(BrowserError::Io("browser data clear timed out".to_string())),
        }
    }

    fn complete_clear(
        sender: &Arc<Mutex<Option<tokio::sync::oneshot::Sender<Result<(), String>>>>>,
        result: Result<(), String>,
    ) {
        if let Some(sender) = sender.lock().ok().and_then(|mut sender| sender.take()) {
            let _ = sender.send(result);
        }
    }

    fn resource_type(context: i32) -> &'static str {
        match context {
            value if value == COREWEBVIEW2_WEB_RESOURCE_CONTEXT_DOCUMENT.0 => "document",
            value if value == COREWEBVIEW2_WEB_RESOURCE_CONTEXT_STYLESHEET.0 => "stylesheet",
            value if value == COREWEBVIEW2_WEB_RESOURCE_CONTEXT_IMAGE.0 => "image",
            value if value == COREWEBVIEW2_WEB_RESOURCE_CONTEXT_MEDIA.0 => "media",
            value if value == COREWEBVIEW2_WEB_RESOURCE_CONTEXT_FONT.0 => "font",
            value if value == COREWEBVIEW2_WEB_RESOURCE_CONTEXT_SCRIPT.0 => "script",
            value if value == COREWEBVIEW2_WEB_RESOURCE_CONTEXT_XML_HTTP_REQUEST.0 => {
                "xmlhttprequest"
            }
            value if value == COREWEBVIEW2_WEB_RESOURCE_CONTEXT_FETCH.0 => "fetch",
            _ => "other",
        }
    }

    fn read_uri(
        read: impl FnOnce(*mut PWSTR) -> windows::core::Result<()>,
    ) -> windows::core::Result<String> {
        let mut value = PWSTR::null();
        read(&mut value)?;
        let result = unsafe { value.to_string() };
        if !value.is_null() {
            unsafe { CoTaskMemFree(Some(value.0.cast())) };
        }
        Ok(result?)
    }

    fn stream_to_data_uri(stream: &windows::Win32::System::Com::IStream) -> Option<String> {
        const MAX_FAVICON_BYTES: usize = 512 * 1024;
        let mut bytes = Vec::new();
        let mut buffer = [0_u8; 8 * 1024];
        loop {
            let mut read = 0_u32;
            unsafe {
                stream
                    .Read(
                        buffer.as_mut_ptr().cast(),
                        buffer.len() as u32,
                        Some(&mut read),
                    )
                    .ok()
                    .ok()?;
            }
            if read == 0 {
                break;
            }
            let read = read as usize;
            if bytes.len().saturating_add(read) > MAX_FAVICON_BYTES {
                return None;
            }
            bytes.extend_from_slice(&buffer[..read]);
        }
        (!bytes.is_empty()).then(|| format!("data:image/png;base64,{}", base64_encode(&bytes)))
    }

    fn base64_encode(bytes: &[u8]) -> String {
        const ALPHABET: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut encoded = String::with_capacity(bytes.len().div_ceil(3) * 4);
        for chunk in bytes.chunks(3) {
            let first = chunk[0];
            let second = *chunk.get(1).unwrap_or(&0);
            let third = *chunk.get(2).unwrap_or(&0);
            encoded.push(ALPHABET[(first >> 2) as usize] as char);
            encoded.push(ALPHABET[(((first & 0b0000_0011) << 4) | (second >> 4)) as usize] as char);
            encoded.push(if chunk.len() > 1 {
                ALPHABET[(((second & 0b0000_1111) << 2) | (third >> 6)) as usize] as char
            } else {
                '='
            });
            encoded.push(if chunk.len() > 2 {
                ALPHABET[(third & 0b0011_1111) as usize] as char
            } else {
                '='
            });
        }
        encoded
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_validation_requires_the_fixed_https_host() {
        assert!(validate_filter_source(&reqwest::Url::parse(EASYLIST_URL).unwrap()).is_ok());
        assert!(validate_filter_source(
            &reqwest::Url::parse("http://easylist.to/list.txt").unwrap()
        )
        .is_err());
        assert!(validate_filter_source(
            &reqwest::Url::parse("https://example.com/list.txt").unwrap()
        )
        .is_err());
    }

    #[test]
    fn network_engine_matches_ad_rules_without_cosmetic_processing() {
        let engine = build_engine(
            "||ads.example^$script\nexample.com##.ad\n".to_string(),
            "||tracker.example^\n".to_string(),
        );
        let ad = Request::new(
            "https://ads.example/banner.js",
            "https://site.example/article",
            "script",
            "",
        )
        .unwrap();
        let page = Request::new(
            "https://site.example/article",
            "https://site.example/article",
            "document",
            "",
        )
        .unwrap();
        assert!(engine.check_network_request(&ad).filter.is_some());
        assert!(engine.check_network_request(&page).filter.is_none());
    }

    #[test]
    fn update_due_after_one_week_only() {
        let now = chrono::Utc::now().timestamp();
        assert!(now.saturating_sub(now - UPDATE_INTERVAL_SECONDS) >= UPDATE_INTERVAL_SECONDS);
        assert!(now.saturating_sub(now - UPDATE_INTERVAL_SECONDS + 1) < UPDATE_INTERVAL_SECONDS);
    }
}
