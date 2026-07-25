//! Response DTOs for SDK HTTP calls.

mod offline_cache_download;
mod project_locales;
mod translation_group;
mod translation_project;
mod validate_api_key;

pub use offline_cache_download::OfflineCacheDownloadResult;
pub use project_locales::ProjectLocales;
pub use translation_group::TranslationGroup;
pub use translation_project::TranslationProject;
pub use validate_api_key::ValidateApiKeyResponse;
