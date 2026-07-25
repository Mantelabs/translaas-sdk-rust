//! Per-request context for SDK calls.

/// Per-request options and response metadata for SDK calls.
///
/// .NET SDK name: `TranslaasRequestContext`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RequestContext {
    /// Channel query parameter.
    pub channel: Option<String>,
    /// Version query parameter (`v`).
    pub version: Option<String>,
    /// Project query parameter (text endpoint only).
    pub project: Option<String>,
    /// Whether to include entry context in responses.
    pub include_context: Option<bool>,
    /// `If-None-Match` request header value.
    pub if_none_match: Option<String>,
    /// Response `ETag` header from the last request.
    pub response_etag: Option<String>,
    /// Whether the last response was `304 Not Modified`.
    pub not_modified: bool,
}

impl RequestContext {
    /// Clears response fields before a new request.
    pub fn reset(&mut self) {
        self.response_etag = None;
        self.not_modified = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reset_clears_response_fields_only() {
        let mut ctx = RequestContext {
            channel: Some("stable".to_string()),
            version: Some("42".to_string()),
            project: Some("my-project".to_string()),
            include_context: Some(true),
            if_none_match: Some("etag-req".to_string()),
            response_etag: Some("etag-res".to_string()),
            not_modified: true,
        };
        ctx.reset();
        assert_eq!(ctx.channel.as_deref(), Some("stable"));
        assert_eq!(ctx.version.as_deref(), Some("42"));
        assert_eq!(ctx.project.as_deref(), Some("my-project"));
        assert_eq!(ctx.include_context, Some(true));
        assert_eq!(ctx.if_none_match.as_deref(), Some("etag-req"));
        assert!(ctx.response_etag.is_none());
        assert!(!ctx.not_modified);
    }

    #[test]
    fn reset_is_idempotent() {
        let mut ctx = RequestContext::default();
        ctx.reset();
        ctx.reset();
        assert!(ctx.response_etag.is_none());
        assert!(!ctx.not_modified);
    }
}
