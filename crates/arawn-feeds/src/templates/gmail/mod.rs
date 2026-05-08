//! Gmail feed templates.

pub mod common;
pub mod inbox_archive;
pub mod label_archive;
pub mod sender_filter;

pub use inbox_archive::InboxArchiveTemplate;
pub use label_archive::LabelArchiveTemplate;
pub use sender_filter::SenderFilterTemplate;
