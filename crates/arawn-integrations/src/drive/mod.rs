//! Google Drive integration. Same shape as Gmail / Calendar:
//!
//! - [`GoogleDriveIntegration`] implements [`crate::Integration`].
//! - [`DriveHub`] is the typed Google Drive v3 client built via shared
//!   plumbing in [`crate::google_common`].
//! - Nine [`arawn_tool::Tool`] impls: `drive_search`, `drive_list`,
//!   `drive_get_metadata`, `drive_read`, `drive_upload`, `drive_update`,
//!   `drive_delete`, and for doc comments `drive_list_open_comments` and
//!   `drive_reply_to_comment` (see [`comments`]).
//!
//! See `docs/src/integrations/drive.md` for setup.

mod client;
pub mod comments;
mod integration;
mod tools;
pub mod uat_tools;

pub use client::{DriveHub, client_from_token_store};
pub use integration::{
    DRIVE_OAUTH_SCOPE, GoogleDriveIntegration, GoogleDriveProviderConfig, SERVICE_NAME,
};
pub use tools::{
    DriveDeleteTool, DriveGetMetadataTool, DriveListOpenCommentsTool, DriveListTool, DriveReadTool,
    DriveReplyToCommentTool, DriveSearchTool, DriveUpdateTool, DriveUploadTool,
};
pub use uat_tools::{UatDriveListOpenCommentsTool, UatDriveSearchTool, uat_drive_tools};
