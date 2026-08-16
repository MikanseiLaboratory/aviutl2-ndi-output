use serde::{Deserialize, Serialize};

pub const PLUGIN_DISPLAY_NAME: &str = "AviUtl2 Network Video Output";
pub const PLUGIN_AUTHOR: &str = "未完成成果物研究所";
pub const PROJECT_CONFIG_KEY: &str = "ndi_live_config";
pub const DEFAULT_QUEUE_DEPTH: usize = 4;
pub const MIN_QUEUE_DEPTH: usize = 1;
pub const MAX_QUEUE_DEPTH: usize = 16;
pub const MAX_SOURCE_NAME_LEN: usize = 63;
pub const MAX_GROUPS_LEN: usize = 255;
pub const PREVIEW_MAX_WIDTH: u32 = 320;
pub const SCENE_HINT: &str =
    "出したいシーンを開いてから描画開始してください。描画中はそのシーンを編集しないでください。";
pub const NDI_SITE: &str = "https://ndi.video/";
pub const NDI_TRADEMARK: &str = "NDI® is a registered trademark of Vizrt NDI AB.";
pub const LICENSE_NOTICE: &str =
    "プラグイン本体は MIT ライセンスです。NDI® ランタイムは同梱の NDI_TERMS.txt に従います。";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    #[serde(default = "default_source_name")]
    pub source_name: String,
    #[serde(default)]
    pub groups: String,
    #[serde(default = "default_true")]
    pub send_video: bool,
    #[serde(default = "default_true")]
    pub send_audio: bool,
    #[serde(default)]
    pub send_alpha: bool,
    #[serde(default = "default_queue_depth")]
    pub send_queue_depth: usize,
}

fn default_source_name() -> String {
    "AviUtl2".to_string()
}

fn default_true() -> bool {
    true
}

fn default_queue_depth() -> usize {
    DEFAULT_QUEUE_DEPTH
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            source_name: default_source_name(),
            groups: String::new(),
            send_video: true,
            send_audio: true,
            send_alpha: false,
            send_queue_depth: DEFAULT_QUEUE_DEPTH,
        }
    }
}

impl PluginConfig {
    pub fn clamped(mut self) -> Self {
        self.send_queue_depth = self
            .send_queue_depth
            .clamp(MIN_QUEUE_DEPTH, MAX_QUEUE_DEPTH);
        if self.source_name.trim().is_empty() {
            self.source_name = default_source_name();
        }
        if self.source_name.chars().count() > MAX_SOURCE_NAME_LEN {
            self.source_name = self.source_name.chars().take(MAX_SOURCE_NAME_LEN).collect();
        }
        if self.groups.chars().count() > MAX_GROUPS_LEN {
            self.groups = self.groups.chars().take(MAX_GROUPS_LEN).collect();
        }
        if !self.send_video && !self.send_audio {
            self.send_video = true;
        }
        self
    }

    pub fn groups_opt(&self) -> Option<&str> {
        let trimmed = self.groups.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    }
}
