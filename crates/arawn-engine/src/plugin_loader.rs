use std::path::Path;

use fidius_host::PluginHandle;
use tracing::{info, warn};

use crate::plugin_adapter::PluginToolAdapter;
use crate::tool::Tool;

/// Scans a directory for `.arawn_tool` archives, unpacks, builds, loads,
/// and returns them as `Box<dyn Tool>` ready for registration.
pub struct PluginLoader;

impl PluginLoader {
    /// Load all `.arawn_tool` plugins from `tools_dir`.
    ///
    /// For each archive:
    /// 1. Unpack to `build_dir/<name>/`
    /// 2. Build via `cargo build`
    /// 3. Load compiled dylib via `PluginHost`
    /// 4. Wrap in `PluginToolAdapter`
    ///
    /// Failures are logged and skipped — one bad plugin doesn't block others.
    pub fn load_tools(tools_dir: &Path, build_dir: &Path) -> Vec<Box<dyn Tool>> {
        let mut tools: Vec<Box<dyn Tool>> = Vec::new();

        let archives = match Self::find_archives(tools_dir) {
            Ok(a) => a,
            Err(e) => {
                warn!(error = %e, "failed to scan plugins directory");
                return tools;
            }
        };

        if archives.is_empty() {
            info!("no .arawn_tool plugins found");
            return tools;
        }

        info!(count = archives.len(), "found .arawn_tool archives");

        for archive in &archives {
            let file_name = archive
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            match Self::load_single(archive, build_dir) {
                Ok(loaded) => {
                    for tool in loaded {
                        info!(tool = tool.name(), source = %file_name, "plugin tool loaded");
                        tools.push(tool);
                    }
                }
                Err(e) => {
                    warn!(archive = %file_name, error = %e, "failed to load plugin");
                }
            }
        }

        tools
    }

    fn find_archives(dir: &Path) -> Result<Vec<std::path::PathBuf>, std::io::Error> {
        if !dir.is_dir() {
            return Ok(Vec::new());
        }

        let mut archives = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file()
                && let Some(ext) = path.extension()
                && ext == "arawn_tool"
            {
                archives.push(path);
            }
        }
        archives.sort();
        Ok(archives)
    }

    fn load_single(
        archive: &Path,
        build_dir: &Path,
    ) -> Result<Vec<Box<dyn Tool>>, Box<dyn std::error::Error>> {
        // 1. Unpack
        let unpacked_dir = fidius_host::package::unpack_fid(archive, build_dir)?;
        info!(dir = ?unpacked_dir, "unpacked plugin archive");

        // 2. Build
        let is_release = !cfg!(debug_assertions);
        let dylib_path = fidius_host::package::build_package(&unpacked_dir, is_release)?;
        info!(dylib = ?dylib_path, "built plugin");

        // 3. Load — search in the directory containing the dylib
        let dylib_dir = dylib_path.parent().unwrap_or(&unpacked_dir);

        let host = fidius_host::PluginHost::builder()
            .search_path(dylib_dir)
            .build()?;

        // Discover all plugins in this dylib
        let discovered = host.discover()?;
        let mut tools: Vec<Box<dyn Tool>> = Vec::new();

        for plugin_info in &discovered {
            let loaded = host.load(&plugin_info.name)?;
            let handle = PluginHandle::from_loaded(loaded);
            let adapter = PluginToolAdapter::new(handle)?;
            tools.push(Box::new(adapter));
        }

        Ok(tools)
    }
}
