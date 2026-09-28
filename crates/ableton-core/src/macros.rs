//! Declarative building blocks of the Live object model.
//!
//! Every Live object is declared once with [`live_object!`] and its
//! properties are listed with [`props!`]:
//!
//! ```ignore
//! live_object! {
//!     /// A cue point in the arrangement.
//!     object CuePoint { ns: "cue-point", raw: RawCuePoint }
//! }
//!
//! props! { CuePoint {
//!     name: String [get set observe];
//!     time: f64 [get observe];
//! }}
//! ```
//!
//! Each flag generates one method:
//!
//! | flag           | generated method                          |
//! |----------------|-------------------------------------------|
//! | `get`          | `async fn name(&self) -> Result<T>`       |
//! | `cached`       | same, served through the etag cache       |
//! | `set`          | `async fn set_name(&self, impl Into<T>)`  |
//! | `set(U)`       | same, but takes `impl Into<U>`            |
//! | `observe`      | `async fn on_name(&self, Fn(T))`          |
//! | `observe(U)`   | same, but the listener receives `U`       |
//!
//! When the wire name isn't a valid method name (`loop`, `type`), rename it
//! with `method = "wire_name": T [...]`.

/// Declares a string enum that round-trips unknown values.
macro_rules! string_enum {
    (
        $(#[$meta:meta])*
        pub enum $name:ident {
            $( $(#[$variant_meta:meta])* $variant:ident = $wire:literal ),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub enum $name {
            $( $(#[$variant_meta])* $variant, )+
            /// A value this version of the crate doesn't know about.
            Unknown(String),
        }

        impl $name {
            /// The value as it's sent over the wire.
            pub fn as_str(&self) -> &str {
                match self {
                    $( Self::$variant => $wire, )+
                    Self::Unknown(other) => other,
                }
            }
        }

        impl ::std::default::Default for $name {
            fn default() -> Self {
                Self::Unknown(String::new())
            }
        }

        impl ::std::str::FromStr for $name {
            type Err = ::std::convert::Infallible;

            fn from_str(value: &str) -> ::std::result::Result<Self, Self::Err> {
                Ok(match value {
                    $( $wire => Self::$variant, )+
                    other => Self::Unknown(other.to_owned()),
                })
            }
        }

        impl ::std::convert::From<&str> for $name {
            fn from(value: &str) -> Self {
                match value.parse() {
                    Ok(parsed) => parsed,
                    Err(never) => match never {},
                }
            }
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl ::serde::Serialize for $name {
            fn serialize<S: ::serde::Serializer>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error> {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> ::serde::Deserialize<'de> for $name {
            fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> ::std::result::Result<Self, D::Error> {
                let value = <::std::string::String as ::serde::Deserialize>::deserialize(deserializer)?;
                Ok(Self::from(value.as_str()))
            }
        }

        $crate::wire::wire_via_serde!($name);
    };
}

/// Declares a Live object type.
///
/// - `object`: identified by the id of its serialized (`raw`) form.
/// - `view`: the view of another object, sharing its id.
/// - `singleton`: a namespace without id (`song`, `application`, …).
macro_rules! live_object {
    (
        $(#[$meta:meta])*
        object $name:ident { ns: $ns:literal, raw: $raw:ty $(,)? }
    ) => {
        $(#[$meta])*
        #[derive(Clone)]
        pub struct $name {
            handle: $crate::ns::Handle,
            raw: ::std::sync::Arc<$raw>,
        }

        impl $name {
            /// Namespace of this object type in the Remote Script.
            pub const NS: &'static str = $ns;

            /// Wraps an object serialized by the Remote Script.
            pub fn new(client: &$crate::Ableton, raw: $raw) -> Self {
                Self {
                    handle: $crate::ns::Handle::new(client, $ns, Some(raw.id.clone())),
                    raw: ::std::sync::Arc::new(raw),
                }
            }

            /// The object as Live serialized it. A snapshot: it doesn't update.
            pub fn raw(&self) -> &$raw {
                &self.raw
            }

            /// Id of the Live object.
            pub fn id(&self) -> &str {
                &self.raw.id
            }
        }

        impl $crate::wire::FromWire for $name {
            fn from_wire(client: &$crate::Ableton, value: ::serde_json::Value) -> $crate::Result<Self> {
                Ok(Self::new(client, $crate::wire::from_serde(value)?))
            }
        }

        live_object!(@common $name);
    };

    (
        $(#[$meta:meta])*
        view $name:ident { ns: $ns:literal $(,)? }
    ) => {
        $(#[$meta])*
        #[derive(Clone)]
        pub struct $name {
            handle: $crate::ns::Handle,
        }

        impl $name {
            /// Namespace of this object type in the Remote Script.
            pub const NS: &'static str = $ns;

            /// The view of the Live object with the given id.
            pub fn new(client: &$crate::Ableton, nsid: impl Into<String>) -> Self {
                Self {
                    handle: $crate::ns::Handle::new(client, $ns, Some(nsid.into())),
                }
            }
        }

        live_object!(@common $name);
    };

    (
        $(#[$meta:meta])*
        singleton $name:ident { ns: $ns:literal $(,)? }
    ) => {
        $(#[$meta])*
        #[derive(Clone)]
        pub struct $name {
            handle: $crate::ns::Handle,
        }

        impl $name {
            /// Namespace of this object in the Remote Script.
            pub const NS: &'static str = $ns;

            /// Addresses the namespace through `client`.
            pub fn new(client: &$crate::Ableton) -> Self {
                Self {
                    handle: $crate::ns::Handle::new(client, $ns, None),
                }
            }
        }

        live_object!(@common $name);
    };

    (@common $name:ident) => {
        impl $crate::ns::LiveObject for $name {
            fn handle(&self) -> &$crate::ns::Handle {
                &self.handle
            }
        }

        impl ::std::fmt::Debug for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.debug_struct(stringify!($name))
                    .field("id", &self.handle.nsid())
                    .finish()
            }
        }

        impl $name {
            /// Low-level access to this object: untyped `get`/`set`/`observe`
            /// and raw function calls.
            pub fn handle(&self) -> &$crate::ns::Handle {
                &self.handle
            }

            /// Readable property names discovered on the Live object.
            ///
            /// Mainly for exploring Live's API; not every property might be
            /// supported by this crate yet.
            pub async fn get_available_properties(&self) -> $crate::Result<Vec<String>> {
                self.handle.call("get_available_properties", None).await
            }

            /// Observable property names (Live's `add_<prop>_listener` APIs).
            pub async fn get_observable_properties(&self) -> $crate::Result<Vec<String>> {
                self.handle.call("get_observable_properties", None).await
            }

            /// Callable method names discovered on the Live object.
            pub async fn get_available_functions(&self) -> $crate::Result<Vec<String>> {
                self.handle.call("get_available_functions", None).await
            }

            /// Sends a raw function invocation to Live. Use with caution.
            pub async fn send_command(
                &self,
                name: &str,
                args: Option<::serde_json::Value>,
            ) -> $crate::Result<::serde_json::Value> {
                self.handle.call(name, args).await
            }
        }
    };
}

/// Generates typed property accessors. See the module docs.
macro_rules! props {
    ($ty:ident { $($body:tt)* }) => {
        impl $ty {
            props!(@entries $($body)*);
        }
    };

    (@entries) => {};
    (@entries $(#[$attr:meta])* $name:ident $(= $wire:literal)? : $t:ty [$($flags:tt)*]; $($rest:tt)*) => {
        props!(@flags [$(#[$attr])*] ($name $(= $wire)?) ($t) $($flags)*);
        props!(@entries $($rest)*);
    };

    (@flags [$($attrs:tt)*] ($($id:tt)*) ($t:ty)) => {};
    (@flags [$($attrs:tt)*] ($($id:tt)*) ($t:ty) get $($rest:tt)*) => {
        props!(@get [$($attrs)*] ($($id)*) ($t) false);
        props!(@flags [$($attrs)*] ($($id)*) ($t) $($rest)*);
    };
    (@flags [$($attrs:tt)*] ($($id:tt)*) ($t:ty) cached $($rest:tt)*) => {
        props!(@get [$($attrs)*] ($($id)*) ($t) true);
        props!(@flags [$($attrs)*] ($($id)*) ($t) $($rest)*);
    };
    (@flags [$($attrs:tt)*] ($($id:tt)*) ($t:ty) set($st:ty) $($rest:tt)*) => {
        props!(@set ($($id)*) ($st));
        props!(@flags [$($attrs)*] ($($id)*) ($t) $($rest)*);
    };
    (@flags [$($attrs:tt)*] ($($id:tt)*) ($t:ty) set $($rest:tt)*) => {
        props!(@set ($($id)*) ($t));
        props!(@flags [$($attrs)*] ($($id)*) ($t) $($rest)*);
    };
    (@flags [$($attrs:tt)*] ($($id:tt)*) ($t:ty) observe($ot:ty) $($rest:tt)*) => {
        props!(@observe ($($id)*) ($ot));
        props!(@flags [$($attrs)*] ($($id)*) ($t) $($rest)*);
    };
    (@flags [$($attrs:tt)*] ($($id:tt)*) ($t:ty) observe $($rest:tt)*) => {
        props!(@observe ($($id)*) ($t));
        props!(@flags [$($attrs)*] ($($id)*) ($t) $($rest)*);
    };

    (@get [$($attrs:tt)*] ($name:ident $(= $wire:literal)?) ($t:ty) $cache:literal) => {
        #[doc = concat!("Reads the `", stringify!($name), "` property.")]
        #[doc = ""]
        $($attrs)*
        pub async fn $name(&self) -> $crate::Result<$t> {
            self.handle.get(props!(@wire $name $(= $wire)?), $cache).await
        }
    };

    (@set ($name:ident $(= $wire:literal)?) ($t:ty)) => {
        ::paste::paste! {
            #[doc = concat!("Sets the `", stringify!($name), "` property.")]
            pub async fn [<set_ $name>](&self, value: impl Into<$t>) -> $crate::Result<()> {
                let value: $t = value.into();
                self.handle.set(props!(@wire $name $(= $wire)?), value).await
            }
        }
    };

    (@observe ($name:ident $(= $wire:literal)?) ($t:ty)) => {
        ::paste::paste! {
            #[doc = concat!("Calls `listener` whenever the `", stringify!($name), "` property changes.")]
            pub async fn [<on_ $name>]<F>(&self, listener: F) -> $crate::Result<$crate::client::Subscription>
            where
                F: Fn($t) + Send + Sync + 'static,
            {
                self.handle.observe::<$t, F>(props!(@wire $name $(= $wire)?), listener).await
            }
        }
    };

    (@wire $name:ident) => { stringify!($name) };
    (@wire $name:ident = $wire:literal) => { $wire };
}
