//! Windows integration: clipboard, locking, tray, autostart and window behaviour.

pub mod autostart;
pub mod clipboard;
pub mod lock;
#[cfg(windows)]
mod session;
pub mod tray;
pub mod window;

/// Events emitted to the frontend.
pub mod events {
    pub const VAULT_LOCKED: &str = "vault-locked";
    pub const QUICK_SEARCH: &str = "quick-search";
    pub const CLIPBOARD_CLEARED: &str = "clipboard-cleared";
}
