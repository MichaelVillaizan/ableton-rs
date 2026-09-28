use serde_json::json;

use super::device::device_members;
use super::{Chain, DeviceView, RawDevice};
use crate::error::Result;

/// Live's `class_name` of VST/AU/VST3 plugins.
pub const PLUGIN_CLASS_NAME: &str = "PluginDevice";

live_object! {
    /// A VST, AU or VST3 plugin.
    object PluginDevice { ns: "plugin-device", raw: RawDevice }
}

device_members!(PluginDevice);

props! { PluginDevice {
    chains: Vec<Chain> [cached];
    return_chains: Vec<Chain> [cached];
    /// Whether the plugin's editor window is open.
    is_editor_open: bool [get set observe];
    /// Presets the plugin offers.
    presets: Vec<String> [get observe];
    /// Index of the selected preset.
    selected_preset_index: i32 [get set observe];
}}

impl PluginDevice {
    /// View aspects of the device.
    pub fn view(&self) -> DeviceView {
        DeviceView::new(self.handle.client(), self.id())
    }

    /// Names of the plugin parameters from `begin` up to `end`, or up to the
    /// last parameter when `end` is `None`.
    pub async fn get_parameter_names(
        &self,
        begin: usize,
        end: Option<usize>,
    ) -> Result<Vec<String>> {
        let end = end.map_or(-1, |end| end as i64);
        self.handle
            .call("get_parameter_names", json!({ "begin": begin, "end": end }))
            .await
    }
}
