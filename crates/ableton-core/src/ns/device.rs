use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    Chain, DeviceView, Handle, LOOPER_CLASS_NAME, LiveObject, LooperDevice, PLUGIN_CLASS_NAME,
    PluginDevice, RackDevice, is_rack_class_name,
};
use crate::Ableton;
use crate::error::Result;
use crate::wire::{FromWire, from_serde};

string_enum! {
    /// Kind of device.
    pub enum DeviceType {
        AudioEffect = "audio_effect",
        Instrument = "instrument",
        MidiEffect = "midi_effect",
        Undefined = "undefined",
    }
}

/// A device as serialized by the Remote Script.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RawDevice {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: DeviceType,
    pub class_name: String,
}

live_object! {
    /// A device on a track or chain.
    ///
    /// Lists of devices come as [`AnyDevice`], which tells racks, Loopers and
    /// plugins apart.
    object Device { ns: "device", raw: RawDevice }
}

/// Properties and functions every kind of device has.
macro_rules! device_members {
    ($ty:ident) => {
        props! { $ty {
            can_have_chains: bool [get];
            can_have_drum_pads: bool [get];
            can_compare_ab: bool [get];
            class_display_name: String [get];
            class_name: String [get];
            /// Drum Racks only; empty for other devices.
            drum_pads: Vec<$crate::ns::DrumPad> [cached];
            is_active: bool [get observe];
            is_using_compare_preset_b: bool [get observe];
            latency_in_ms: f64 [get observe];
            latency_in_samples: i32 [get observe];
            name: String [get set observe];
            parameters: Vec<$crate::ns::DeviceParameter> [cached observe];
            device_type = "type": $crate::ns::DeviceType [get];
        }}

        impl $ty {
            /// Saves the current state of the device to the compare AB slot.
            /// Fails unless `can_compare_ab`.
            pub async fn save_preset_to_compare_ab_slot(&self) -> $crate::Result<()> {
                self.handle
                    .call("save_preset_to_compare_ab_slot", None)
                    .await
            }

            /// Stores the selected bank in the device for persistence.
            pub async fn store_chosen_bank(
                &self,
                script_index: i32,
                bank_index: i32,
            ) -> $crate::Result<()> {
                self.handle
                    .call(
                        "store_chosen_bank",
                        ::serde_json::json!([script_index, bank_index]),
                    )
                    .await
            }
        }
    };
}

pub(crate) use device_members;

device_members!(Device);

props! { Device {
    /// Rack devices only; empty for other devices.
    chains: Vec<Chain> [cached];
    /// Rack devices only; empty for other devices.
    return_chains: Vec<Chain> [cached];
}}

impl Device {
    /// View aspects of the device.
    pub fn view(&self) -> DeviceView {
        DeviceView::new(self.handle.client(), self.id())
    }
}

/// Any kind of device.
#[derive(Debug, Clone)]
pub enum AnyDevice {
    Device(Device),
    Looper(LooperDevice),
    Plugin(PluginDevice),
    Rack(RackDevice),
}

impl AnyDevice {
    /// Wraps a serialized device as the most specific type its
    /// `class_name` allows.
    pub fn new(client: &Ableton, raw: RawDevice) -> Self {
        match raw.class_name.as_str() {
            LOOPER_CLASS_NAME => Self::Looper(LooperDevice::new(client, raw)),
            PLUGIN_CLASS_NAME => Self::Plugin(PluginDevice::new(client, raw)),
            name if is_rack_class_name(name) => Self::Rack(RackDevice::new(client, raw)),
            _ => Self::Device(Device::new(client, raw)),
        }
    }

    /// Id of the Live object.
    pub fn id(&self) -> &str {
        self.raw().id.as_str()
    }

    /// The device as Live serialized it.
    pub fn raw(&self) -> &RawDevice {
        match self {
            Self::Device(device) => device.raw(),
            Self::Looper(device) => device.raw(),
            Self::Plugin(device) => device.raw(),
            Self::Rack(device) => device.raw(),
        }
    }

    /// The device as a plain [`Device`], giving access to the properties
    /// every device has.
    pub fn as_device(&self) -> Device {
        match self {
            Self::Device(device) => device.clone(),
            other => Device::new(other.handle().client(), other.raw().clone()),
        }
    }

    /// The device as a [`RackDevice`], if it is one.
    pub fn as_rack(&self) -> Option<&RackDevice> {
        match self {
            Self::Rack(device) => Some(device),
            _ => None,
        }
    }

    /// The device as a [`LooperDevice`], if it is one.
    pub fn as_looper(&self) -> Option<&LooperDevice> {
        match self {
            Self::Looper(device) => Some(device),
            _ => None,
        }
    }

    /// The device as a [`PluginDevice`], if it is one.
    pub fn as_plugin(&self) -> Option<&PluginDevice> {
        match self {
            Self::Plugin(device) => Some(device),
            _ => None,
        }
    }

    /// Whether the device is a rack.
    pub fn is_rack(&self) -> bool {
        matches!(self, Self::Rack(_))
    }

    /// Whether the device is a Looper.
    pub fn is_looper(&self) -> bool {
        matches!(self, Self::Looper(_))
    }

    /// Whether the device is a plugin.
    pub fn is_plugin(&self) -> bool {
        matches!(self, Self::Plugin(_))
    }
}

impl LiveObject for AnyDevice {
    fn handle(&self) -> &Handle {
        match self {
            Self::Device(device) => device.handle(),
            Self::Looper(device) => device.handle(),
            Self::Plugin(device) => device.handle(),
            Self::Rack(device) => device.handle(),
        }
    }
}

impl FromWire for AnyDevice {
    fn from_wire(client: &Ableton, value: Value) -> Result<Self> {
        Ok(Self::new(client, from_serde(value)?))
    }
}
