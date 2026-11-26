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
    if let Err(error) = wayland::apply(window) {
        tracing::warn!("Failed to blur the window: {error:#}");
    }

    #[cfg(not(any(windows, target_os = "linux")))]
    let _ = window;
}

#[cfg(target_os = "linux")]
mod wayland {
    use anyhow::{bail, Result};
    use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle};
    use tauri::{WebviewWindow, WindowEvent};
    use wayland_client::backend::{Backend, ObjectId};
    use wayland_client::globals::{registry_queue_init, GlobalListContents};
    use wayland_client::protocol::wl_compositor::WlCompositor;
    use wayland_client::protocol::wl_region::WlRegion;
    use wayland_client::protocol::wl_registry::WlRegistry;
    use wayland_client::protocol::wl_surface::WlSurface;
    use wayland_client::{delegate_noop, Connection, Dispatch, EventQueue, Proxy, QueueHandle};
    use wayland_protocols::ext::background_effect::v1::client::ext_background_effect_manager_v1::ExtBackgroundEffectManagerV1;
    use wayland_protocols::ext::background_effect::v1::client::ext_background_effect_surface_v1::ExtBackgroundEffectSurfaceV1;

    /// Windows 11's window corner radius; must match `.window.rounded` in Window.svelte.
    const CORNER_RADIUS: i32 = 8;

    struct State;

    impl Dispatch<WlRegistry, GlobalListContents> for State {
        fn event(
            _: &mut Self,
            _: &WlRegistry,
            _: <WlRegistry as Proxy>::Event,
            _: &GlobalListContents,
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
        }
    }

    delegate_noop!(State: WlCompositor);
    delegate_noop!(State: WlRegion);
    delegate_noop!(State: ignore ExtBackgroundEffectManagerV1);
    delegate_noop!(State: ExtBackgroundEffectSurfaceV1);

    struct Blur {
        connection: Connection,
        // Never dispatched, but the proxies are bound to it.
        _events: EventQueue<State>,
        queue: QueueHandle<State>,
        compositor: WlCompositor,
        effect: ExtBackgroundEffectSurfaceV1,
    }

    impl Blur {
        /// Blurs the window size in logical pixels, cut to the rounded corners.
        fn set_size(&self, width: i32, height: i32) {
            let region = self.compositor.create_region(&self.queue, ());
            let radius = CORNER_RADIUS.min(width / 2).min(height / 2);
            region.add(0, radius, width, height - 2 * radius);
            for y in 0..radius {
                let dy = radius as f64 - (y as f64 + 0.5);
                let inset =
                    radius - (radius as f64 * radius as f64 - dy * dy).sqrt().round() as i32;
                region.add(inset, y, width - 2 * inset, 1);
                region.add(inset, height - 1 - y, width - 2 * inset, 1);
            }
            self.effect.set_blur_region(Some(&region));
            region.destroy();
            // Applied with GTK's next commit of the surface.
            let _ = self.connection.flush();
        }

        fn fit(&self, window: &WebviewWindow) {
            let (Ok(size), Ok(scale)) = (window.inner_size(), window.scale_factor()) else {
                return;
            };
            let size = size.to_logical::<i32>(scale);
            self.set_size(size.width, size.height);
        }
    }

    /// Does nothing on X11 and on compositors without `ext_background_effect_manager_v1`.
    pub fn apply(window: &WebviewWindow) -> Result<()> {
        let RawDisplayHandle::Wayland(display) = window.display_handle()?.as_raw() else {
            return Ok(());
        };
        let RawWindowHandle::Wayland(surface) = window.window_handle()?.as_raw() else {
            return Ok(());
        };

        // GTK owns the display and the surface; we only add our own queue to them.
        let connection = Connection::from_backend(unsafe {
            Backend::from_foreign_display(display.display.as_ptr().cast())
        });
        let (globals, event_queue) = registry_queue_init::<State>(&connection)?;
        let queue = event_queue.handle();
        let Ok(manager) = globals.bind::<ExtBackgroundEffectManagerV1, _, _>(&queue, 1..=1, ())
        else {
            return Ok(());
        };
        let compositor = globals.bind::<WlCompositor, _, _>(&queue, 1..=6, ())?;
        let surface_id =
            unsafe { ObjectId::from_ptr(WlSurface::interface(), surface.surface.as_ptr().cast()) }?;
        let Ok(surface) = WlSurface::from_id(&connection, surface_id) else {
            bail!("GTK's surface is not a wl_surface");
        };

        let blur = Blur {
            effect: manager.get_background_effect(&surface, &queue, ()),
            connection,
            _events: event_queue,
            queue,
            compositor,
        };
        blur.fit(window);

        let resized = window.clone();
        window.on_window_event(move |event| {
            if matches!(
                event,
                WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. }
            ) {
                blur.fit(&resized);
            }
        });
        Ok(())
    }
}
