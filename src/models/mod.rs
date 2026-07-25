//! Data transfer objects, errors, and per-request context for the Translaas SDK.
//!
//! This module has **no HTTP dependencies**. Wire contracts are documented in the
//! umbrella [porting reference](https://github.com/Mantelabs/translaas-all/blob/main/.docs/translaas-sdk-dotnet-porting-reference.md).

#![warn(missing_docs)]

mod api_key;
mod context;
mod errors;
pub mod language_codes;
mod plural;
mod requests;
mod responses;
mod serde_utils;

pub use api_key::{read_json_ulid, resolve_default_project_id};
pub use context::RequestContext;
pub use errors::{
    parse_translaas_error, ApiError, ConfigurationError, NoLanguageError, OfflineCacheError,
    OfflineCacheMissError, TranslaasError,
};
pub use plural::PluralCategory;
pub use requests::{
    GetGroupTranslationsRequest, GetOfflineCacheRequest, GetProjectLocalesRequest,
    GetProjectTranslationsRequest, GetTranslationRequest, ReportMissingKeyItem,
    ReportMissingKeysRequest,
};
pub use responses::{
    OfflineCacheDownloadResult, ProjectLocales, TranslationGroup, TranslationProject,
    ValidateApiKeyResponse,
};
