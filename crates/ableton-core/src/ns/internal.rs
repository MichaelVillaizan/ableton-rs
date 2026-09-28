use crate::PLUGIN_VERSION;
use crate::error::Result;

live_object! {
    /// Internal helpers of the Remote Script.
    singleton Internal { ns: "internal" }
}

props! { Internal {
    /// Version of the installed Remote Script.
    version: String [get];
    ping: bool [get];
}}

impl Internal {
    /// Whether the installed Remote Script matches [`PLUGIN_VERSION`].
    pub async fn is_plugin_up_to_date(&self) -> Result<bool> {
        Ok(self.version().await? == PLUGIN_VERSION)
    }
}
