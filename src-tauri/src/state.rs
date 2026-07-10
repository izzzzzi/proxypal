use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::Arc;
use std::sync::Mutex;
use tauri_plugin_shell::process::CommandChild;

use crate::config::AppConfig;
use crate::types::{AuthStatus, CopilotStatus, OAuthState, ProxyStatus};

/// App state shared across all Tauri commands
pub struct AppState {
    pub proxy_status: Mutex<ProxyStatus>,
    pub auth_status: Mutex<AuthStatus>,
    pub config: Mutex<AppConfig>,
    pub pending_oauth: Mutex<Option<OAuthState>>,
    pub proxy_process: Mutex<Option<CommandChild>>,
    pub copilot_status: Mutex<CopilotStatus>,
    pub copilot_process: Mutex<Option<CommandChild>>,
    pub log_watcher_running: Arc<AtomicBool>,
    /// Generation counter for the usage-queue collector thread.
    /// Each `start` bumps and captures its own generation;
    /// the thread exits when `gen.load() != my_gen`.
    /// `stop` and `exit` bump to invalidate all current threads.
    pub usage_queue_collector_gen: Arc<AtomicU64>,
    pub request_counter: Arc<AtomicU64>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            proxy_status: Mutex::new(ProxyStatus::default()),
            auth_status: Mutex::new(AuthStatus::default()),
            config: Mutex::new(AppConfig::default()),
            pending_oauth: Mutex::new(None),
            proxy_process: Mutex::new(None),
            copilot_status: Mutex::new(CopilotStatus::default()),
            copilot_process: Mutex::new(None),
            log_watcher_running: Arc::new(AtomicBool::new(false)),
            usage_queue_collector_gen: Arc::new(AtomicU64::new(0)),
            request_counter: Arc::new(AtomicU64::new(0)),
        }
    }
}
