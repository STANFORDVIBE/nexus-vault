//! IPC surface exposed to the WebView. Every command the frontend may call lives here;
//! commands stay thin and delegate to `core` / `crypto` / `database`.

pub mod app;
