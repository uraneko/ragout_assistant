pub mod input;

use std::io::StdoutLock;

pub use input::init;
pub use input::{History, Input};

// this trait can be implemented be it at the ragout lib or ragout_custom_events macro, once
// InputAction has been defined,
// NOTE: if this is not implemented, input.write() also can't be implemented
pub trait DebugLog<E> {
    fn log(&mut self, event: &E);

    fn dl_rfd(&self) -> i32;
}

pub trait Writer<E> {
    fn write(&mut self, h: &mut History, ia: &E, sol: &mut StdoutLock<'_>, ui: &mut String);
}
