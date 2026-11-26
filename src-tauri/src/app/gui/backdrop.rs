/*
 * This file is part of LiquidLauncher (https://github.com/CCBlueX/LiquidLauncher)
 *
 * Copyright (c) 2015 - 2024 CCBlueX
 *
 * LiquidLauncher is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * LiquidLauncher is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with LiquidLauncher. If not, see <https://www.gnu.org/licenses/>.
 */

//! Blurs the desktop behind the translucent window.

use tauri::WebviewWindow;

pub fn apply(window: &WebviewWindow) {
    #[cfg(windows)]
    {
        use tauri::window::{Effect, EffectsBuilder};

        // Acrylic lags while the window is dragged before Windows 11 22H2, blur from 22H2 on.
        let acrylic = matches!(
            os_info::get().version(),
            os_info::Version::Semantic(_, _, build) if *build >= 22621
        );
        let effect = if acrylic {
            Effect::Acrylic
        } else {
            Effect::Blur
        };
        if let Err(error) = window.set_effects(EffectsBuilder::new().effect(effect).build()) {
            tracing::warn!("Failed to blur the window: {error}");
        }
    }

    #[cfg(target_os = "linux")]
    match blur_rounded(window) {
        Ok(()) => {
            let resized = window.clone();
            window.on_window_event(move |event| {
                if matches!(
                    event,
                    tauri::WindowEvent::Resized(_) | tauri::WindowEvent::ScaleFactorChanged { .. }
                ) {
                    let _ = blur_rounded(&resized);
                }
            });
        }
        Err(error) => tracing::info!("No window blur: {error:#}"),
    }

    #[cfg(not(any(windows, target_os = "linux")))]
    let _ = window;
}

/// Blurs the window inside its rounded corners: Windows 11's radius, as `.window.rounded` in
/// Window.svelte.
#[cfg(target_os = "linux")]
fn blur_rounded(window: &WebviewWindow) -> anyhow::Result<()> {
    let size = window
        .inner_size()?
        .to_logical::<u32>(window.scale_factor()?);
    window_vibrancy::apply_wayland_blur(
        window,
        Some(window_vibrancy::RoundedRect {
            width: size.width,
            height: size.height,
            radius: 8,
        }),
    )?;
    Ok(())
}
