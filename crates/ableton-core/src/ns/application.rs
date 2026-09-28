use std::time::Duration;

use serde_json::json;

use super::{ApplicationView, Browser};
use crate::error::Result;

string_enum! {
    /// Live's `Application.MessageButtons`.
    pub enum MessageButtons {
        Ok = "OK_BUTTON",
        OkNewSet = "OK_NEW_SET_BUTTON",
        OkRetry = "OK_RETRY_BUTTON",
        SaveDontSave = "SAVE_DONT_SAVE_BUTTON",
        OkAccount = "OK_ACCOUNT_BUTTON",
        OkPurchase = "OK_PURCHASE_BUTTON",
    }
}

string_enum! {
    /// Live's `Application.PushDialogType`.
    pub enum PushDialogType {
        MessageBox = "MESSAGE_BOX",
        OutOfUnlocksDialog = "OUT_OF_UNLOCKS_DIALOG",
        RentToOwnLicenseExpiredDialog = "RENT_TO_OWN_LICENSE_EXPIRED_DIALOG",
    }
}

string_enum! {
    /// Live's `Application.Variants`.
    pub enum ApplicationVariant {
        Beta = "Beta",
        Intro = "Intro",
        Lite = "Lite",
        Standard = "Standard",
        Suite = "Suite",
        Trial = "Trial",
    }
}

string_enum! {
    /// Live's `Application.UnavailableFeature`.
    pub enum UnavailableFeature {
        NoteVelocityRangesAndProbabilities = "note_velocity_ranges_and_probabilities",
    }
}

/// Options of [`Application::show_on_the_fly_message`].
#[derive(Debug, Clone)]
pub struct ShowOnTheFlyMessageOptions {
    pub buttons: MessageButtons,
    pub enable_markup: bool,
    pub show_success_icon: bool,
    pub push_dialog_type: PushDialogType,
    /// How long to wait for the user to press a button.
    pub timeout: Duration,
}

impl Default for ShowOnTheFlyMessageOptions {
    fn default() -> Self {
        Self {
            buttons: MessageButtons::Ok,
            enable_markup: false,
            show_success_icon: false,
            push_dialog_type: PushDialogType::MessageBox,
            timeout: Duration::from_secs(60),
        }
    }
}

live_object! {
    /// The Live application: version, dialogs, browser and view.
    singleton Application { ns: "application" }
}

props! { Application {
    average_process_usage: f64 [get observe];
    bugfix_version: i32 [get];
    build_id: String [get];
    current_dialog_button_count: i32 [get];
    current_dialog_message: String [get];
    major_version: i32 [get];
    minor_version: i32 [get];
    number_of_push_apps_running: i32 [get];
    open_dialog_count: i32 [get observe];
    peak_process_usage: f64 [get observe];
    unavailable_features: Vec<UnavailableFeature> [cached observe];
    variant: ApplicationVariant [get];
    version: String [get];
}}

impl Application {
    /// Live's browser.
    pub fn browser(&self) -> Browser {
        Browser::new(self.handle.client())
    }

    /// View aspects of the application.
    pub fn view(&self) -> ApplicationView {
        ApplicationView::new(self.handle.client())
    }

    /// Whether `option_name` is set in Options.txt.
    pub async fn has_option(&self, option_name: &str) -> Result<bool> {
        self.handle.call("has_option", json!([option_name])).await
    }

    /// Presses a button of the current message box by index.
    pub async fn press_current_dialog_button(&self, index: usize) -> Result<()> {
        self.handle
            .call("press_current_dialog_button", json!([index]))
            .await
    }

    /// Shows a message box with free-form text and returns the index of the
    /// pressed button.
    ///
    /// Blocks until the user presses a button, which is why the timeout
    /// defaults to a minute.
    pub async fn show_on_the_fly_message(
        &self,
        message: &str,
        options: ShowOnTheFlyMessageOptions,
    ) -> Result<i32> {
        self.handle
            .call_with_timeout(
                "show_on_the_fly_message",
                json!({
                    "message": message,
                    "buttons": options.buttons,
                    "enable_markup": options.enable_markup,
                    "show_success_icon": options.show_success_icon,
                    "push_dialog_type": options.push_dialog_type,
                }),
                options.timeout,
            )
            .await
    }
}
