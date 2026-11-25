//! Spelly core: Harper (grammar, context ranking, curated dictionary) plus
//! forced corrections, custom dictionary packs and phone-style undo.

mod pack;
mod pack_edit;
mod policy;
mod runtime;
mod session;
mod settings;
mod store;
mod spelly;
mod typist;

pub use pack::Pack;
pub use pack_edit::{Entry, Issue, PackFile};
pub use settings::{Accent, Settings, Strictness};
pub use store::{now, start_of_today, LogEntry, Store};
pub use policy::Policy;
pub use runtime::Runtime;
pub use session::Session;
pub use typist::{Edit, Key, Outcome, Typist};
pub use spelly::{Correction, Highlight, Spelly};
