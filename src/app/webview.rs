use crate::error::LauncherError;
use web_view::Content;
use std::sync::{Arc, Mutex};
use anyhow::Result;

use log::info;
use crate::utils::download_file;

pub(crate) async fn download_client<F>(url: &str, on_progress: F) -> Result<Vec<u8>> where F : Fn(u64, u64) {
    info!("Retrieving LiquidBounce download url from {}", url);

    let download_link_cell = Arc::new(Mutex::new(None));

    let cloned_cell = download_link_cell.clone();

    {
        let mut webview = web_view::builder()
            .title("Download LiquidBounce")
            .content(Content::Url(format!("{}&liquidlauncher={}", url, 0)))
            .size(1000, 600)
            .resizable(true)
            .debug(true)
            .user_data(())
            .invoke_handler(move |webview, arg| {
                let mut split = arg.split('|');

                if let Some(cmd) = split.next() {
                    if cmd == "download" {
                        if let Some(dl_url) = split.next() {
                            *cloned_cell.lock().unwrap() = Some(dl_url.to_owned());

                            webview.exit();

                            return Ok(());
                        }
                    }
                }

                Err(web_view::Error::Custom(Box::new(LauncherError::InvalidJavaScript("Invalid command".to_owned()))))
            })
            .build()?;

        webview.set_maximized(true);

        webview.run()?;
    }

    let url = {
        let mg= download_link_cell.lock().unwrap();

        mg.as_ref().ok_or_else(|| LauncherError::InvalidJavaScript("Failed to retrieve the download link".to_owned()))?.to_owned()
    };

    info!("Downloading LiquidBounce from {}", url);

    return download_file(&url, on_progress).await;
}