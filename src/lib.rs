use std::panic::AssertUnwindSafe;
use std::sync::Arc;

use aviutl2::AnyResult;

use crate::config::{PLUGIN_AUTHOR, PLUGIN_DISPLAY_NAME, PROJECT_CONFIG_KEY, PluginConfig};
use crate::controller::{LiveController, SharedState};

mod bake;
pub mod config;
pub mod controller;
pub mod media;
pub mod ndi;
pub mod player;
mod ui;

#[aviutl2::plugin(GenericPlugin)]
pub struct NdiLivePlugin {
    window: aviutl2_eframe::EframeWindow,
    controller: Arc<LiveController>,
}

impl aviutl2::generic::GenericPlugin for NdiLivePlugin {
    fn new(_info: aviutl2::AviUtl2Info) -> AnyResult<Self> {
        init_logging();
        tracing::info!("Initializing {PLUGIN_DISPLAY_NAME}");

        let shared = Arc::new(SharedState::new());
        let controller = Arc::new(LiveController::new(Arc::clone(&shared)));
        let ui_controller = Arc::clone(&controller);
        let window =
            aviutl2_eframe::EframeWindow::new("AviUtl2NetworkVideoOutput", move |cc, _handle| {
                Ok(Box::new(ui::UiApp::new(cc, ui_controller)))
            })?;

        Ok(Self { window, controller })
    }

    fn plugin_info(&self) -> aviutl2::generic::GenericPluginTable {
        aviutl2::generic::GenericPluginTable {
            name: PLUGIN_DISPLAY_NAME.to_string(),
            information: format!(
                "今開いているシーンを NDI® で送出 / {PLUGIN_AUTHOR} / v{version} / https://github.com/MikanseiLaboratory/aviutl2-network-video-output",
                version = env!("CARGO_PKG_VERSION")
            ),
        }
    }

    fn register(&mut self, registry: &mut aviutl2::generic::HostAppHandle) {
        let handle = Arc::new(registry.create_edit_handle());
        self.controller.set_edit_handle(handle);
        if let Ok(handle) = self.window.handle() {
            let _ = registry.register_window_client(PLUGIN_DISPLAY_NAME, &handle);
        }
    }

    fn on_project_load(&mut self, project: &mut aviutl2::generic::ProjectFile<'_>) {
        self.controller.stop();
        if let Ok(config) = project.deserialize::<PluginConfig>(PROJECT_CONFIG_KEY) {
            self.controller.shared().set_config(config.clamped());
        }
        self.controller.mark_scene_dirty();
    }

    fn on_project_save(&mut self, project: &mut aviutl2::generic::ProjectFile<'_>) {
        let config = self.controller.shared().config_snapshot();
        if let Err(e) = project.serialize(PROJECT_CONFIG_KEY, &config) {
            tracing::warn!("failed to save live config: {e}");
        }
    }

    fn event_change_edit_frame(&mut self) {
        request_ui_repaint(&self.window);
    }

    fn event_change_scene_info(&mut self) {
        self.controller.mark_scene_dirty();
        request_ui_repaint(&self.window);
    }
}

impl Drop for NdiLivePlugin {
    fn drop(&mut self) {
        self.controller.stop();
    }
}

fn request_ui_repaint(window: &aviutl2_eframe::EframeWindow) {
    let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
        if let Ok(ctx) = window.egui_ctx() {
            ctx.request_repaint();
        }
    }));
}

fn init_logging() {
    let _ = aviutl2::tracing_subscriber::fmt()
        .with_max_level(if cfg!(debug_assertions) {
            tracing::Level::DEBUG
        } else {
            tracing::Level::INFO
        })
        .event_format(aviutl2::logger::AviUtl2Formatter)
        .with_writer(aviutl2::logger::AviUtl2LogWriter)
        .try_init();
}

aviutl2::register_generic_plugin!(NdiLivePlugin);
