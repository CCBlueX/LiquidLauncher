// use crate::error::LauncherError;
//use web_view::Content;
// use std::sync::{Arc, Mutex};

use std::sync::{Arc, Mutex};

use anyhow::Result;
use log::info;
use tauri::Manager;

// use log::info;
use crate::{utils::download_file, error::LauncherError};

pub(crate) async fn download_client<F>(url: &str, on_progress: F, window: &Arc<Mutex<tauri::Window>>) -> Result<Vec<u8>> where F : Fn(u64, u64) {
    let app_handle = window.lock().unwrap().app_handle();

    let download_page = format!("{}&liquidlauncher=1", url);
    let download_view = tauri::WindowBuilder::new(
        &app_handle,
        "download",
        tauri::WindowUrl::External(download_page.parse().unwrap())
    )
        .title("Download of LiquidBounce")
        .user_agent("LiquidLauncher")
        .center()
        .focused(true)
        .maximized(true)
        .build().unwrap();
    
    // Show and maximize the download view
    download_view.show().unwrap();
    download_view.maximize().unwrap();

    // Wait for the download to finish
    let download_link_cell = Arc::new(Mutex::new(None));
    let cloned_cell = download_link_cell.clone();

    download_view.once("download", move |event| {
        if let Some(payload) = event.payload() {
            *cloned_cell.lock().unwrap() = Some(payload.to_owned());
        }
    });

    let url = {
        let mg= download_link_cell.lock().unwrap();
        
        mg.as_ref().ok_or_else(|| LauncherError::InvalidJavaScript("Failed to retrieve the download link".to_owned()))?.to_owned()
    };
        
    info!("Downloading LiquidBounce from {}", url);

    return download_file(&url, on_progress).await;
}