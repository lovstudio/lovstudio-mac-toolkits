mod helper;

use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::Mutex,
    thread,
    time::Duration,
};
use tauri::menu::{CheckMenuItem, CheckMenuItemBuilder, MenuBuilder, MenuItem, MenuItemBuilder};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, Runtime, WindowEvent, Wry};

const TOGGLE_PROTECT_ALL_ID: &str = "toggle_protect_all_apps";
const OPEN_SETTINGS_ID: &str = "open_settings";
const QUIT_ID: &str = "quit";
const APP_PROTECTION_EVENT: &str = "app-protection-changed";
const CLI_TOOLS: &[&str] = &[
    "codex", "claude", "cc", "opencode", "aider", "gemini", "blender",
];

#[derive(Clone, Serialize)]
struct RunningApp {
    name: String,
    count: usize,
    kind: &'static str,
    whitelisted: bool,
    protected: bool,
}

#[cfg(target_os = "macos")]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum RunningAppKind {
    Cli,
    UserApp,
    SystemApp,
}

#[cfg(target_os = "macos")]
impl RunningAppKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Cli => "cli",
            Self::UserApp => "app",
            Self::SystemApp => "system",
        }
    }
}

#[cfg(target_os = "macos")]
struct DetectedApp {
    name: String,
    kind: RunningAppKind,
}

#[cfg(target_os = "macos")]
struct RunningAppGroup {
    count: usize,
    kind: RunningAppKind,
}

#[derive(Clone, Serialize)]
struct AppProtectionState {
    protect_all_apps: bool,
    effective_lid_protection: bool,
    protected_count: usize,
    whitelist: Vec<String>,
    running_apps: Vec<RunningApp>,
}

struct ProtectionRuntime {
    protect_all_apps: bool,
    whitelist: BTreeSet<String>,
    last_effective_lid_protection: Option<bool>,
}

struct AppState {
    protection: Mutex<ProtectionRuntime>,
}

struct TrayItems {
    protect_all_toggle: CheckMenuItem<Wry>,
    status: MenuItem<Wry>,
}

impl ProtectionRuntime {
    fn load() -> Self {
        let config = AppConfig::load();
        Self {
            protect_all_apps: config.protect_all_apps,
            whitelist: config.whitelist,
            last_effective_lid_protection: None,
        }
    }

    fn config(&self) -> AppConfig {
        AppConfig {
            protect_all_apps: self.protect_all_apps,
            whitelist: self.whitelist.clone(),
        }
    }
}

struct AppConfig {
    protect_all_apps: bool,
    whitelist: BTreeSet<String>,
}

impl AppConfig {
    fn load() -> Self {
        let mut config = Self {
            protect_all_apps: query_lid_sleep_prevention().unwrap_or(false),
            whitelist: BTreeSet::new(),
        };

        let Ok(contents) = fs::read_to_string(config_path()) else {
            return config;
        };

        for line in contents.lines() {
            if let Some(value) = line.trim().strip_prefix("protect_all_apps=") {
                config.protect_all_apps = value == "true";
            } else if let Some(value) = line.trim().strip_prefix("whitelist=") {
                if !value.is_empty() {
                    config.whitelist.insert(normalize_app_name(value));
                }
            }
        }

        config
    }

    fn save(&self) -> Result<(), String> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("Create config directory: {error}"))?;
        }

        let mut contents = format!("protect_all_apps={}\n", self.protect_all_apps);
        for name in &self.whitelist {
            contents.push_str("whitelist=");
            contents.push_str(name);
            contents.push('\n');
        }
        fs::write(path, contents).map_err(|error| format!("Write config: {error}"))
    }
}

pub fn run_helper() -> Result<(), String> {
    helper::run()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            app.manage(AppState {
                protection: Mutex::new(ProtectionRuntime::load()),
            });

            let state = refresh_app_protection(app.handle()).unwrap_or_else(|error| {
                eprintln!("{error}");
                fallback_app_protection_state()
            });
            let status_item = MenuItemBuilder::with_id("status", protection_status_label(&state))
                .enabled(false)
                .build(app)?;
            let protect_all_toggle =
                CheckMenuItemBuilder::with_id(TOGGLE_PROTECT_ALL_ID, "Prevent Lid Sleep")
                    .checked(state.protect_all_apps)
                    .build(app)?;
            let open_settings =
                MenuItemBuilder::with_id(OPEN_SETTINGS_ID, "Open Settings").build(app)?;
            let quit = MenuItemBuilder::with_id(QUIT_ID, "Quit").build(app)?;
            let menu = MenuBuilder::new(app)
                .item(&status_item)
                .separator()
                .item(&protect_all_toggle)
                .separator()
                .item(&open_settings)
                .item(&quit)
                .build()?;

            app.manage(TrayItems {
                protect_all_toggle: protect_all_toggle.clone(),
                status: status_item.clone(),
            });

            start_protection_monitor(app.handle().clone());

            let tray_icon_bytes = include_bytes!("../icons/tray-icon.png");
            log::info!("tray-icon.png embedded bytes: {}", tray_icon_bytes.len());
            let tray_icon = image::load_from_memory(tray_icon_bytes)
                .map(|img| {
                    let rgba = img.to_rgba8();
                    let (w, h) = rgba.dimensions();
                    log::info!("tray icon decoded: {}x{}", w, h);
                    tauri::image::Image::new_owned(rgba.into_raw(), w, h)
                })
                .expect("failed to decode tray-icon.png");

            let protect_all_for_menu = protect_all_toggle.clone();
            TrayIconBuilder::new()
                .icon(tray_icon)
                .icon_as_template(true)
                .tooltip("Lovstudio.ai Mac Menu Manager")
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(move |app, event| match event.id().as_ref() {
                    TOGGLE_PROTECT_ALL_ID => {
                        let requested = protect_all_for_menu.is_checked().unwrap_or(false);
                        let app = app.clone();
                        std::thread::spawn(move || {
                            if let Err(error) = set_protect_all_apps_for_app(&app, requested) {
                                eprintln!("{error}");
                                if let Ok(state) = current_app_protection_state(&app) {
                                    let _ = sync_app_protection_state(&app, &state);
                                }
                            }
                        });
                    }
                    OPEN_SETTINGS_ID => {
                        let _ = show_settings_window(app);
                    }
                    QUIT_ID => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_app_protection,
            set_protect_all_apps,
            set_app_whitelist,
            open_settings_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn get_app_protection(app: AppHandle) -> Result<AppProtectionState, String> {
    let state = refresh_app_protection(&app)?;
    sync_app_protection_state(&app, &state)?;
    Ok(state)
}

#[tauri::command]
fn set_protect_all_apps(app: AppHandle, enabled: bool) -> Result<AppProtectionState, String> {
    set_protect_all_apps_for_app(&app, enabled)
}

#[tauri::command]
fn set_app_whitelist(
    app: AppHandle,
    name: String,
    enabled: bool,
) -> Result<AppProtectionState, String> {
    {
        let state = app.state::<AppState>();
        let mut runtime = state
            .protection
            .lock()
            .map_err(|_| "Protection state is unavailable".to_string())?;
        if enabled {
            runtime.whitelist.insert(name);
        } else {
            runtime.whitelist.remove(&name);
        }
        runtime.config().save()?;
    }

    let state = refresh_app_protection(&app)?;
    sync_app_protection_state(&app, &state)?;
    Ok(state)
}

#[tauri::command]
fn open_settings_window(app: AppHandle) -> Result<(), String> {
    show_settings_window(&app)
}

fn start_protection_monitor(app: AppHandle) {
    thread::spawn(move || loop {
        match refresh_app_protection(&app).and_then(|state| sync_app_protection_state(&app, &state))
        {
            Ok(()) => {}
            Err(error) => eprintln!("{error}"),
        }
        thread::sleep(Duration::from_secs(10));
    });
}

fn set_protect_all_apps_for_app<R: Runtime>(
    app: &AppHandle<R>,
    enabled: bool,
) -> Result<AppProtectionState, String> {
    {
        let state = app.state::<AppState>();
        let mut runtime = state
            .protection
            .lock()
            .map_err(|_| "Protection state is unavailable".to_string())?;
        runtime.protect_all_apps = enabled;
        runtime.config().save()?;
    }

    let state = refresh_app_protection(app)?;
    sync_app_protection_state(app, &state)?;
    Ok(state)
}

fn current_app_protection_state<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<AppProtectionState, String> {
    let running_apps = find_running_apps()?;
    build_app_protection_state(app, running_apps, None)
}

fn refresh_app_protection<R: Runtime>(app: &AppHandle<R>) -> Result<AppProtectionState, String> {
    let running_apps = find_running_apps()?;
    let active_whitelist = running_apps.iter().any(|app| app.whitelisted);
    let state = app.state::<AppState>();
    let mut runtime = state
        .protection
        .lock()
        .map_err(|_| "Protection state is unavailable".to_string())?;
    let effective = runtime.protect_all_apps || active_whitelist;
    let current = query_lid_sleep_prevention().unwrap_or(false);

    if current != effective || runtime.last_effective_lid_protection != Some(effective) {
        apply_lid_sleep_prevention(effective)?;
        runtime.last_effective_lid_protection = Some(effective);
    }

    Ok(build_app_protection_state_from_runtime(
        &runtime,
        running_apps,
        effective,
    ))
}

fn build_app_protection_state<R: Runtime>(
    app: &AppHandle<R>,
    running_apps: Vec<RunningApp>,
    effective_override: Option<bool>,
) -> Result<AppProtectionState, String> {
    let state = app.state::<AppState>();
    let runtime = state
        .protection
        .lock()
        .map_err(|_| "Protection state is unavailable".to_string())?;
    let active_whitelist = running_apps.iter().any(|app| app.whitelisted);
    let effective = effective_override.unwrap_or(runtime.protect_all_apps || active_whitelist);
    Ok(build_app_protection_state_from_runtime(
        &runtime,
        running_apps,
        effective,
    ))
}

fn build_app_protection_state_from_runtime(
    runtime: &ProtectionRuntime,
    mut running_apps: Vec<RunningApp>,
    effective: bool,
) -> AppProtectionState {
    for app in &mut running_apps {
        app.protected = runtime.protect_all_apps || app.whitelisted;
    }
    running_apps.sort_by(|left, right| {
        right
            .whitelisted
            .cmp(&left.whitelisted)
            .then(running_app_kind_rank(left.kind).cmp(&running_app_kind_rank(right.kind)))
            .then(right.count.cmp(&left.count))
            .then(left.name.cmp(&right.name))
    });

    AppProtectionState {
        protect_all_apps: runtime.protect_all_apps,
        effective_lid_protection: effective,
        protected_count: running_apps.iter().filter(|app| app.protected).count(),
        whitelist: runtime.whitelist.iter().cloned().collect(),
        running_apps,
    }
}

fn running_app_kind_rank(kind: &str) -> u8 {
    match kind {
        "cli" => 0,
        "app" => 1,
        "system" => 2,
        _ => 3,
    }
}

fn sync_app_protection_state<R: Runtime>(
    app: &AppHandle<R>,
    state: &AppProtectionState,
) -> Result<(), String> {
    if let Some(items) = app.try_state::<TrayItems>() {
        items
            .protect_all_toggle
            .set_checked(state.protect_all_apps)
            .map_err(|error| error.to_string())?;
        items
            .status
            .set_text(protection_status_label(state))
            .map_err(|error| error.to_string())?;
    }

    app.emit(APP_PROTECTION_EVENT, state.clone())
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn fallback_app_protection_state() -> AppProtectionState {
    AppProtectionState {
        protect_all_apps: query_lid_sleep_prevention().unwrap_or(false),
        effective_lid_protection: query_lid_sleep_prevention().unwrap_or(false),
        protected_count: 0,
        whitelist: Vec::new(),
        running_apps: Vec::new(),
    }
}

fn show_settings_window<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "Settings window is not available".to_string())?;
    window.unminimize().map_err(|error| error.to_string())?;
    window.show().map_err(|error| error.to_string())?;
    window.set_focus().map_err(|error| error.to_string())
}

fn protection_status_label(state: &AppProtectionState) -> String {
    if state.effective_lid_protection {
        format!("Lid Sleep Guard: {} apps", state.protected_count)
    } else {
        "Lid Sleep Guard: Off".to_string()
    }
}

fn find_running_apps() -> Result<Vec<RunningApp>, String> {
    #[cfg(target_os = "macos")]
    {
        let output = Command::new("/bin/ps")
            .args(["-axo", "pid=,uid=,command="])
            .output()
            .map_err(|error| format!("Failed to run ps: {error}"))?;

        if !output.status.success() {
            return Err(command_error("ps failed", &output));
        }

        let own_pid = std::process::id();
        let uid = unsafe { libc::getuid() };
        let whitelist = AppConfig::load().whitelist;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut groups = BTreeMap::<String, RunningAppGroup>::new();

        for line in stdout.lines() {
            if let Some(app) = classify_user_app(line, own_pid, uid) {
                let group = groups.entry(app.name).or_insert(RunningAppGroup {
                    count: 0,
                    kind: app.kind,
                });
                group.count += 1;
                group.kind = group.kind.min(app.kind);
            }
        }

        Ok(groups
            .into_iter()
            .map(|(name, group)| {
                let whitelisted = whitelist.contains(&name);
                RunningApp {
                    name,
                    count: group.count,
                    kind: group.kind.as_str(),
                    whitelisted,
                    protected: whitelisted,
                }
            })
            .collect())
    }

    #[cfg(not(target_os = "macos"))]
    {
        Ok(Vec::new())
    }
}

#[cfg(target_os = "macos")]
fn classify_user_app(line: &str, own_pid: u32, uid: libc::uid_t) -> Option<DetectedApp> {
    let line = line.trim_start();
    let pid_split = line.find(char::is_whitespace)?;
    let pid = line[..pid_split].parse::<u32>().ok()?;
    if pid == own_pid {
        return None;
    }

    let rest = line[pid_split..].trim_start();
    let uid_split = rest.find(char::is_whitespace)?;
    let process_uid = rest[..uid_split].parse::<libc::uid_t>().ok()?;
    if process_uid != uid {
        return None;
    }

    let command_line = rest[uid_split..].trim_start();
    let lower = command_line.to_lowercase();
    if lower.contains("/lovstudio mac toolkits.app/")
        || lower.contains("/lovstudio.ai mac menu manager.app/")
    {
        return None;
    }

    if let Some(name) = app_bundle_name(command_line) {
        return Some(name);
    }

    let executable = command_line.split_whitespace().next()?;
    let executable_name = Path::new(executable)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(executable);
    let executable_lower = executable_name.to_lowercase();
    if CLI_TOOLS.contains(&executable_lower.as_str()) {
        return Some(DetectedApp {
            name: cli_display_name(&executable_lower),
            kind: RunningAppKind::Cli,
        });
    }

    None
}

#[cfg(target_os = "macos")]
fn app_bundle_name(command_line: &str) -> Option<DetectedApp> {
    let lower = command_line.to_lowercase();
    let marker = ".app/contents/";
    let marker_start = lower.find(marker)?;
    let app_end = marker_start + ".app".len();
    let before_contents = &command_line[..app_end];
    let app_start = before_contents
        .rfind('/')
        .map(|index| index + 1)
        .unwrap_or(0);
    let name = &before_contents[app_start..before_contents.len() - ".app".len()];

    if name == "Codex Computer Use" {
        return Some(DetectedApp {
            name: normalize_app_name(name),
            kind: app_kind_for_path(before_contents),
        });
    }

    if name.ends_with(" Helper") || name.contains(" Helper ") || name == "Electron" {
        return None;
    }

    Some(DetectedApp {
        name: normalize_app_name(name),
        kind: app_kind_for_path(before_contents),
    })
}

#[cfg(target_os = "macos")]
fn app_kind_for_path(path: &str) -> RunningAppKind {
    if path.to_lowercase().starts_with("/system/") {
        RunningAppKind::SystemApp
    } else {
        RunningAppKind::UserApp
    }
}

#[cfg(target_os = "macos")]
fn cli_display_name(name: &str) -> String {
    match name {
        "codex" => "Codex".to_string(),
        "claude" | "cc" => "Claude Code".to_string(),
        "opencode" => "OpenCode".to_string(),
        "aider" => "Aider".to_string(),
        "gemini" => "Gemini CLI".to_string(),
        "blender" => "Blender".to_string(),
        other => other.to_string(),
    }
}

fn normalize_app_name(name: &str) -> String {
    match name {
        "Codex Computer Use" | "Codex CLI" => "Codex".to_string(),
        other => other.to_string(),
    }
}

fn query_lid_sleep_prevention() -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    {
        let output = Command::new("/usr/bin/pmset")
            .arg("-g")
            .output()
            .map_err(|error| format!("Failed to run pmset: {error}"))?;

        if !output.status.success() {
            return Err(command_error("pmset failed", &output));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.lines().any(|line| {
            let mut parts = line.split_whitespace();
            matches!(
                (parts.next(), parts.next()),
                (Some("disablesleep"), Some("1")) | (Some("SleepDisabled"), Some("1"))
            )
        }))
    }

    #[cfg(not(target_os = "macos"))]
    {
        Err("Lid close protection is only available on macOS".to_string())
    }
}

fn apply_lid_sleep_prevention(enabled: bool) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        helper::install_if_needed()?;
        helper::set_lid_sleep_prevention(enabled).map(|_| ())
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = enabled;
        Err("Lid close protection is only available on macOS".to_string())
    }
}

fn config_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join("com.lovstudio.mactoolkits")
        .join("config")
}

#[cfg(target_os = "macos")]
fn command_error(prefix: &str, output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if !stderr.is_empty() {
        format!("{prefix}: {stderr}")
    } else if !stdout.is_empty() {
        format!("{prefix}: {stdout}")
    } else {
        prefix.to_string()
    }
}
