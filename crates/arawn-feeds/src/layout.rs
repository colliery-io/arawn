//! On-disk path layout for feed data.
//!
//! ```text
//! {data_root}/data/<provider>/<template>/<feed_id>/
//!   ├── meta.json                  # runtime-managed (cursor + last_run)
//!   └── <whatever the template wants>
//! ```
//!
//! `data_root` defaults to `~/.arawn/`; configurable via
//! `[storage].data_dir` in arawn.toml. The runtime guarantees the
//! `<feed_id>` dir exists and is writable before calling
//! `template.run()`. Everything under the feed dir except `meta.json`
//! is the template's territory.

use std::path::{Path, PathBuf};

use crate::error::FeedError;

pub struct DataLayout {
    /// `{data_root}/data/`. e.g. `~/.arawn/data`.
    root: PathBuf,
}

impl DataLayout {
    /// `data_root` is the arawn data dir (e.g. `~/.arawn`). The feeds
    /// runtime appends `data/` to keep its tree separate from
    /// `arawn.db`, `tokens/`, etc.
    pub fn new(data_root: impl Into<PathBuf>) -> Self {
        Self {
            root: data_root.into().join("data"),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// `{root}/<provider>/<template_name>/<feed_id>/`. Splits a
    /// template name like `slack/channel-archive` into its provider
    /// and template segments so two feeds with different templates
    /// don't collide.
    pub fn feed_dir(&self, template_name: &str, feed_id: &str) -> Result<PathBuf, FeedError> {
        let (provider, template) = template_name.split_once('/').ok_or_else(|| {
            FeedError::InvalidParams(format!(
                "template name '{template_name}' must be '<provider>/<template>'"
            ))
        })?;
        Ok(self.root.join(provider).join(template).join(feed_id))
    }

    /// Create the feed dir if it doesn't exist; return its path.
    pub fn ensure_feed_dir(
        &self,
        template_name: &str,
        feed_id: &str,
    ) -> Result<PathBuf, FeedError> {
        let dir = self.feed_dir(template_name, feed_id)?;
        std::fs::create_dir_all(&dir)
            .map_err(|e| FeedError::Storage(format!("create feed dir {}: {e}", dir.display())))?;
        Ok(dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feed_dir_splits_on_slash() {
        let layout = DataLayout::new("/tmp/arawn");
        let p = layout.feed_dir("slack/channel-archive", "design").unwrap();
        assert_eq!(p, PathBuf::from("/tmp/arawn/data/slack/channel-archive/design"));
    }

    #[test]
    fn feed_dir_rejects_template_without_provider() {
        let layout = DataLayout::new("/tmp/arawn");
        let err = layout.feed_dir("nope-no-slash", "x").unwrap_err();
        assert!(matches!(err, FeedError::InvalidParams(_)));
    }

    #[test]
    fn ensure_feed_dir_creates_path() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = DataLayout::new(tmp.path());
        let dir = layout
            .ensure_feed_dir("stub/echo", "feed-1")
            .unwrap();
        assert!(dir.exists());
    }
}
