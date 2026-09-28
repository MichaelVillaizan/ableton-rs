use serde::{Deserialize, Serialize};

string_enum! {
    /// Grid a groove is based on.
    pub enum GrooveBase {
        Four = "gb_four",
        Eight = "gb_eight",
        EightTriplet = "gb_eight_triplet",
        Sixteen = "gb_sixteen",
        SixteenTriplet = "gb_sixteen_triplet",
        ThirtyTwo = "gb_thirtytwo",
    }
}

/// A groove as serialized by the Remote Script.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RawGroove {
    pub id: String,
    pub name: String,
}

live_object! {
    /// A groove of the groove pool.
    object Groove { ns: "groove", raw: RawGroove }
}

props! { Groove {
    base: GrooveBase [get set];
    name: String [get set observe];
    quantization_amount: f64 [get set observe];
    random_amount: f64 [get set observe];
    timing_amount: f64 [get set observe];
    velocity_amount: f64 [get set observe];
}}
