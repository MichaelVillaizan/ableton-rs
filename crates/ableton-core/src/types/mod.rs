//! Value types shared by several namespaces.

mod color;
mod note;

pub use color::{Color, Rgb};
pub use note::{Note, NoteExtended, NoteTuple};
