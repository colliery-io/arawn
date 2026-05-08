//! Concrete `FeedTemplate` impls organized per provider.

pub mod slack;
pub mod stub;

use std::sync::Arc;

use crate::registry::FeedTemplateRegistry;

/// Build the registry of every template the binary supports. Wire all
/// new templates here. Order doesn't matter — registry is keyed by
/// template name.
pub fn default_registry() -> FeedTemplateRegistry {
    let mut r = FeedTemplateRegistry::new();
    r.register(Arc::new(stub::EchoTemplate));
    r.register(Arc::new(slack::ChannelArchiveTemplate));
    r.register(Arc::new(slack::DmArchiveTemplate));
    r.register(Arc::new(slack::MyMentionsTemplate));
    r
}
