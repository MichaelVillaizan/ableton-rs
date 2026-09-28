use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::Error;

/// A color as Live represents it: `0xRRGGBB`.
///
/// Serializes as its number representation, which is what Live expects when
/// setting a color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Color(u32);

/// Red, green and blue channels of a [`Color`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    /// Creates a color from its `0xRRGGBB` number representation.
    pub const fn new(value: u32) -> Self {
        Self(value & 0xFF_FFFF)
    }

    /// Creates a color from its channels.
    pub const fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self(((r as u32) << 16) | ((g as u32) << 8) | b as u32)
    }

    /// `#rrggbb`.
    pub fn hex(&self) -> String {
        format!("#{:06x}", self.0)
    }

    /// The color's channels.
    pub const fn rgb(&self) -> Rgb {
        Rgb {
            r: (self.0 >> 16) as u8,
            g: (self.0 >> 8) as u8,
            b: self.0 as u8,
        }
    }

    /// `0xRRGGBB`, as Live stores it.
    pub const fn number_representation(&self) -> u32 {
        self.0
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.hex())
    }
}

impl FromStr for Color {
    type Err = Error;

    /// Parses `rrggbb` or `#rrggbb`.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let digits = value.strip_prefix('#').unwrap_or(value);
        if digits.len() != 6 {
            return Err(Error::InvalidArgument(format!(
                "Color {value} is not in a valid format"
            )));
        }
        u32::from_str_radix(digits, 16)
            .map(Self)
            .map_err(|_| Error::InvalidArgument(format!("Color {value} is not in a valid format")))
    }
}

impl From<u32> for Color {
    fn from(value: u32) -> Self {
        Self::new(value)
    }
}

impl From<i32> for Color {
    fn from(value: i32) -> Self {
        Self::new(value as u32)
    }
}

impl From<i64> for Color {
    fn from(value: i64) -> Self {
        Self::new(value as u32)
    }
}

impl From<Rgb> for Color {
    fn from(Rgb { r, g, b }: Rgb) -> Self {
        Self::from_rgb(r, g, b)
    }
}

impl Serialize for Color {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u32(self.0)
    }
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = f64::deserialize(deserializer)?;
        Ok(Self::new(value as u32))
    }
}

crate::wire::wire_via_serde!(Color);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_between_representations() {
        let color = Color::new(0xFF8000);
        assert_eq!(color.hex(), "#ff8000");
        assert_eq!(
            color.rgb(),
            Rgb {
                r: 255,
                g: 128,
                b: 0
            }
        );
        assert_eq!("#ff8000".parse::<Color>().unwrap(), color);
        assert_eq!("ff8000".parse::<Color>().unwrap(), color);
        assert!("#fff".parse::<Color>().is_err());
        assert_eq!(
            serde_json::to_value(color).unwrap(),
            serde_json::json!(0xFF8000)
        );
    }
}
