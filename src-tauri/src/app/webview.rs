use std::{sync::{Arc, Mutex}, time::Duration};
use serde::Deserialize;
use anyhow::Result;
use log::{info, debug};
use tauri::Manager;
use tokio::time::sleep;
use crate::{utils::download_file};

pub(crate) async fn download_client<F>(url: &str, on_progress: F, window: &Arc<Mutex<tauri::Window>>) -> Result<Vec<u8>> where F : Fn(u64, u64){
    let app_handle = window.lock().unwrap().app_handle();

    let download_page = format!("{}&liquidlauncher=1", url);
    let download_view = tauri::WindowBuilder::new(
        &app_handle,
        "client_download",
        tauri::WindowUrl::External(download_page.parse().unwrap())
    )
        .title("Download of LiquidBounce")
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
        debug!("Triggerd download event");

        if let Some(payload) = event.payload() {
            #[derive(Deserialize)]
            struct DownloadPayload {
                url: String
            }

            let payload = serde_json::from_str::<DownloadPayload>(payload).unwrap();

            info!("Received download link: {}", payload.url);
            *cloned_cell.lock().unwrap() = Some(payload.url);
        }
    });

    let url = loop {
        // sleep for 100ms
        sleep(Duration::from_millis(100)).await;

        // check if we got the download link
        if let Ok(mg) = download_link_cell.lock() {
            if let Some(received) = mg.clone() {
                break received;
            }
        }
    };

    download_view.close().unwrap();

    info!("Downloading LiquidBounce from {}", url);
    return download_file(&url, on_progress).await;
}