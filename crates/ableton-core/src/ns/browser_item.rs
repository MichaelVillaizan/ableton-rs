use serde::{Deserialize, Serialize};

/// A browser item as serialized by the Remote Script.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RawBrowserItem {
    pub id: String,
    pub children: Vec<RawBrowserItem>,
    pub name: String,
    pub is_loadable: bool,
    pub is_selected: bool,
    pub is_device: bool,
    pub is_folder: bool,
    pub source: String,
    pub uri: String,
}

live_object! {
    /// An entry of Live's browser (a folder, a device, a sample, …).
    object BrowserItem { ns: "browser-item", raw: RawBrowserItem }
}

props! { BrowserItem {
    /// Same descendants as Live's `iter_children`.
    children: Vec<BrowserItem> [cached];
    is_device: bool [cached];
    is_folder: bool [cached];
    is_loadable: bool [get];
    is_selected: bool [get];
    name: String [cached];
    source: String [cached];
    uri: String [cached];
}}
