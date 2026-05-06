//! Google Drive integration. Same shape as Gmail / Calendar:
//!
//! - [`GoogleDriveIntegration`] implements [`crate::Integration`].
//! - [`DriveHub`] is the typed Google Drive v3 client built via shared
//!   plumbing in [`crate::google_common`].
//! - Seven [`arawn_tool::Tool`] impls: `drive_search`, `drive_list`,
//!   `drive_get_metadata`, `drive_read`, `drive_upload`, `drive_update`,
//!   `drive_delete`.
//!
//! See `docs/src/integrations/drive.md` for setup.

mod client;
mod integration;
mod tools;

pub use client::{DriveHub, client_from_token_store};
pub use integration::{
    DRIVE_OAUTH_SCOPE, GoogleDriveIntegration, GoogleDriveProviderConfig,
};
pub use tools::{
    DriveDeleteTool, DriveGetMetadataTool, DriveListTool, DriveReadTool, DriveSearchTool,
    DriveUpdateTool, DriveUploadTool,
};
