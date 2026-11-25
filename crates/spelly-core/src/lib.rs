//! Spelly core: Harper (grammar, context ranking, curated dictionary) plus
//! forced corrections, custom dictionary packs and phone-style undo.

mod pack;
mod policy;
mod session;
mod spelly;
mod typist;

pub use pack::Pack;
pub use policy::Policy;
pub use session::Session;
pub use typist::{Edit, Key, Outcome, Typist};
pub use spelly::{Correction, Highlight, Spelly};
