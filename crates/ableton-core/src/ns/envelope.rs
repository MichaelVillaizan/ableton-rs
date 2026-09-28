use serde::{Deserialize, Serialize};
use serde_json::json;

use super::DeviceParameter;
use crate::error::Result;

/// Bezier control points of an envelope event.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ControlCoefficients {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
}

/// A breakpoint of an envelope.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EnvelopeEvent {
    pub time: f64,
    pub value: f64,
    pub control_coefficients: Option<ControlCoefficients>,
}

crate::wire::wire_via_serde!(ControlCoefficients, EnvelopeEvent);

/// An envelope as serialized by the Remote Script.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RawEnvelope {
    pub id: String,
}

live_object! {
    /// An automation or modulation envelope.
    object Envelope { ns: "envelope", raw: RawEnvelope }
}

props! { Envelope {
    parameter: DeviceParameter [cached];
}}

impl Envelope {
    /// Creates an event at `time`, optionally curved by `control_coefficients`.
    pub async fn create_event(
        &self,
        time: f64,
        value: f64,
        control_coefficients: Option<ControlCoefficients>,
    ) -> Result<()> {
        self.handle
            .call(
                "create_event",
                json!({ "time": time, "value": value, "control_coefficients": control_coefficients }),
            )
            .await
    }

    /// Deletes the events between `from_time` and `to_time`.
    pub async fn delete_events_in_range(&self, from_time: f64, to_time: f64) -> Result<()> {
        self.handle
            .call(
                "delete_events_in_range",
                json!({ "from_time": from_time, "to_time": to_time }),
            )
            .await
    }

    /// The events between `from_time` and `to_time`.
    pub async fn events_in_range(
        &self,
        from_time: f64,
        to_time: f64,
    ) -> Result<Vec<EnvelopeEvent>> {
        self.handle
            .call(
                "events_in_range",
                json!({ "from_time": from_time, "to_time": to_time }),
            )
            .await
    }

    /// Creates a step of `length` with `value` at `start_time`.
    pub async fn insert_step(&self, start_time: f64, length: f64, value: f64) -> Result<()> {
        self.handle
            .call(
                "insert_step",
                json!({ "start_time": start_time, "length": length, "value": value }),
            )
            .await
    }

    /// The parameter value at `time`.
    pub async fn value_at_time(&self, time: f64) -> Result<f64> {
        self.handle
            .call("value_at_time", json!({ "time": time }))
            .await
    }
}
