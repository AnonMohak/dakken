//! Typing-test logic with no terminal code, so it is easy to test.

pub mod practice;
pub mod session;
pub mod stats;
pub mod words;

pub use session::{Key, Mode, QuoteLength, STRICT_MODES, Session, Strict};
