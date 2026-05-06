# Google Drive

Lets the agent search, list, read, upload, update, and delete files in your Drive. Seven tools land when configured: `drive_search`, `drive_list`, `drive_get_metadata`, `drive_read`, `drive_upload`, `drive_update`, `drive_delete`.

For setup, see the [Connecting integrations](../getting-started.md#6-connect-integrations) section of Getting Started — Drive is covered under the **Google** subsection alongside Gmail and Calendar (one Google Cloud project covers all three).

After `/connect google_drive` succeeds, try:

```
list the files in my Drive root
```

## Notes

- **Full read+write scope by default.** Drive uses `https://www.googleapis.com/auth/drive`, not the read-only scope. This is so `drive_upload` / `drive_update` / `drive_delete` work. If you only want read access, omit the upload/update/delete tools at the agent level (a more granular per-tool scope option may come later).
- **Delete is recoverable.** `drive_delete` moves files to Drive's trash, not permadelete. You can restore from <https://drive.google.com/drive/trash>.
- **Read content dispatch.** Google native files are exported automatically:
  - Docs → markdown
  - Sheets → CSV
  - Slides → plain text
  - Drawings → PNG
  - Forms / Sites / Scripts → no machine-readable export (open in browser)
  - Anything else → raw bytes (UTF-8 decoded for text-like MIME types, base64 otherwise; capped at 1 MB by default).
- **Permission prompts on writes.** `drive_upload` / `drive_update` / `drive_delete` ask for permission in the default permission mode. Read tools auto-allow.
