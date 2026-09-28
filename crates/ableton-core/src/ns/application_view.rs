use serde_json::json;

use crate::error::Result;

string_enum! {
    /// A view of Live's window.
    pub enum View {
        Browser = "Browser",
        Session = "Session",
        Arranger = "Arranger",
        Detail = "Detail",
        DetailClip = "Detail/Clip",
        DetailDeviceChain = "Detail/DeviceChain",
    }
}

string_enum! {
    /// The main document view.
    pub enum DocumentView {
        Session = "Session",
        Arranger = "Arranger",
    }
}

string_enum! {
    /// Direction to scroll or zoom a view in.
    pub enum NavDirection {
        Up = "up",
        Down = "down",
        Left = "left",
        Right = "right",
    }
}

live_object! {
    /// View aspects of the application.
    singleton ApplicationView { ns: "application-view" }
}

props! { ApplicationView {
    browse_mode: bool [get observe];
    focused_document_view: DocumentView [get observe];
}}

impl ApplicationView {
    /// The available main views (e.g. Session and Arranger).
    pub async fn available_main_views(&self) -> Result<Vec<View>> {
        self.handle.call_cached("available_main_views", None).await
    }

    /// Shows and focuses `view`.
    pub async fn focus_view(&self, view: View) -> Result<()> {
        self.handle.call("focus_view", json!([view])).await
    }

    /// Hides `view`.
    pub async fn hide_view(&self, view: View) -> Result<()> {
        self.handle.call("hide_view", json!([view])).await
    }

    /// Whether `view` is visible. With `main_window_only` false the second
    /// window is checked too.
    pub async fn is_view_visible(&self, view: View, main_window_only: bool) -> Result<bool> {
        self.handle
            .call("is_view_visible", json!([view, main_window_only]))
            .await
    }

    /// Scrolls `view` in `direction`, if possible.
    pub async fn scroll_view(
        &self,
        view: View,
        direction: NavDirection,
        modifier_pressed: bool,
    ) -> Result<()> {
        self.handle
            .call(
                "scroll_view",
                json!({ "direction": direction, "view": view, "modifier_pressed": modifier_pressed }),
            )
            .await
    }

    /// Shows `view`.
    pub async fn show_view(&self, view: View) -> Result<()> {
        self.handle.call("show_view", json!([view])).await
    }

    /// Reveals the device chain and browser and starts hot-swapping the
    /// selected device. Calling it again stops hot-swapping.
    pub async fn toggle_browse(&self) -> Result<()> {
        self.handle.call("toggle_browse", None).await
    }

    /// Zooms `view` in `direction`, if possible.
    pub async fn zoom_view(
        &self,
        view: View,
        direction: NavDirection,
        modifier_pressed: bool,
    ) -> Result<()> {
        self.handle
            .call(
                "zoom_view",
                json!({ "direction": direction, "view": view, "modifier_pressed": modifier_pressed }),
            )
            .await
    }
}
