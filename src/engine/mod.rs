//! Typing-test logic with no terminal code, so it is easy to test.

pub mod session;
pub mod stats;
pub mod words;

pub use session::{Key, Mode, STRICT_MODES, Session, Strict};
