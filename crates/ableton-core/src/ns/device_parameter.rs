use serde::{Deserialize, Serialize};

string_enum! {
    /// Automation state of a parameter.
    pub enum AutomationState {
        None = "none",
        Playing = "playing",
        Overridden = "overridden",
    }
}

string_enum! {
    /// Whether a parameter can currently be changed.
    pub enum ParameterState {
        Enabled = "enabled",
        Irrelevant = "irrelevant",
        Disabled = "disabled",
    }
}

/// A device parameter as serialized by the Remote Script.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RawDeviceParameter {
    pub id: String,
    pub name: String,
    pub value: f64,
    pub is_quantized: bool,
}

live_object! {
    /// A parameter of a device (a knob, a switch, a chooser, …).
    object DeviceParameter { ns: "device-parameter", raw: RawDeviceParameter }
}

props! { DeviceParameter {
    automation_state: AutomationState [get observe];
    default_value: f64 [get];
    is_enabled: bool [get];
    is_quantized: bool [get];
    max: f64 [get];
    min: f64 [get];
    name: String [get observe];
    original_name: String [get];
    state: ParameterState [get observe];
    value: f64 [get set observe];
    value_items: Vec<String> [get];
}}
