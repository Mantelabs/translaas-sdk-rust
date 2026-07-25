//! Offline ZIP download result populated by the HTTP client.

/// Offline cache ZIP download metadata and bytes.
///
/// This type is not deserialized from JSON; the HTTP client populates it from
/// response headers and body bytes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OfflineCacheDownloadResult {
    /// Whether the response was `304 Not Modified`.
    pub not_modified: bool,
    /// Response `ETag` header value.
    pub etag: Option<String>,
    /// Suggested filename from `Content-Disposition`.
    pub suggested_file_name: Option<String>,
    /// Response body bytes when present.
    pub content: Option<Vec<u8>>,
}
