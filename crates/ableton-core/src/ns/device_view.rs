live_object! {
    /// View aspects of a device.
    view DeviceView { ns: "device-view" }
}

props! { DeviceView {
    is_collapsed: bool [get set observe];
}}
