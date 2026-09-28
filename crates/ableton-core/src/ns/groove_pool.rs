use super::Groove;

live_object! {
    /// The grooves of the current Live Set.
    singleton GroovePool { ns: "groove-pool" }
}

props! { GroovePool {
    grooves: Vec<Groove> [cached observe];
}}
