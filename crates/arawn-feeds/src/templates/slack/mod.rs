//! Slack feed templates.

mod channel_archive;
mod common;
mod dm_archive;
mod my_mentions;

pub use channel_archive::ChannelArchiveTemplate;
pub use dm_archive::DmArchiveTemplate;
pub use my_mentions::MyMentionsTemplate;
