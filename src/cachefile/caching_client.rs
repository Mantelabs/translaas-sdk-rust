//! Offline cache decorator with fallback modes for read operations.

use crate::client::{
    Error, GetEntryOptions, GetGroupOptions, GetOfflineCacheOptions, GetProjectLocalesOptions,
    GetProjectOptions, TranslaasClient,
};
use crate::models::{
    ConfigurationError, OfflineCacheDownloadResult, OfflineCacheError, OfflineCacheMissError,
    ProjectLocales, ReportMissingKeyItem, TranslationGroup, TranslationProject,
    ValidateApiKeyResponse,
};

use super::caching_options::{CachingOptions, FallbackMode};
use super::fallback::is_network_or_api_error;
use super::offline_entry::resolve_entry_from_group;
use super::provider::{Provider, SaveOptions};
use super::update_group_cache::try_update_group_cache;

/// Decorates [`TranslaasClient`] with offline cache fallback for read operations.
#[derive(Debug)]
pub struct CachingClient<C, P> {
    inner: C,
    cache: P,
    opts: CachingOptions,
}

impl<C, P> CachingClient<C, P> {
    /// Wraps `inner` with offline cache behavior using `opts`.
    pub fn new(inner: C, cache: P, opts: CachingOptions) -> Result<Self, ConfigurationError> {
        if opts.default_project_id.trim().is_empty() {
            return Err(ConfigurationError {
                message: "cachefile: default_project_id must not be empty".to_string(),
            });
        }
        Ok(Self {
            inner,
            cache,
            opts: CachingOptions {
                default_project_id: opts.default_project_id.trim().to_string(),
                fallback_mode: opts.fallback_mode,
            },
        })
    }

    /// Returns a reference to the wrapped inner client.
    pub fn inner(&self) -> &C {
        &self.inner
    }

    /// Returns a reference to the offline cache provider.
    pub fn cache(&self) -> &P {
        &self.cache
    }

    /// Returns the configured fallback options.
    pub fn options(&self) -> &CachingOptions {
        &self.opts
    }
}

impl<C, P> CachingClient<C, P>
where
    C: TranslaasClient,
    P: Provider,
{
    async fn get_entry_cache_first(
        &self,
        group: &str,
        entry: &str,
        lang: &str,
        opts: GetEntryOptions<'_>,
    ) -> Result<String, Error> {
        if let Some(value) = self.resolve_entry_from_cache(group, entry, lang, &opts)? {
            return Ok(value);
        }

        match self.inner.get_entry(group, entry, lang, opts).await {
            Ok(result) => {
                try_update_group_cache(
                    &self.inner,
                    &self.cache,
                    &self.opts.default_project_id,
                    group,
                    lang,
                )
                .await;
                Ok(result)
            }
            Err(err) if is_network_or_api_error(&err) => Err(entry_miss_error(
                &self.opts.default_project_id,
                lang,
                group,
                entry,
            )),
            Err(err) => Err(err),
        }
    }

    async fn get_entry_api_first(
        &self,
        group: &str,
        entry: &str,
        lang: &str,
        opts: GetEntryOptions<'_>,
    ) -> Result<String, Error> {
        let number = opts.number;
        let parameters = opts.parameters.clone();

        match self.inner.get_entry(group, entry, lang, opts).await {
            Ok(result) => {
                try_update_group_cache(
                    &self.inner,
                    &self.cache,
                    &self.opts.default_project_id,
                    group,
                    lang,
                )
                .await;
                Ok(result)
            }
            Err(err) if is_network_or_api_error(&err) => {
                let lookup_opts = GetEntryOptions {
                    number,
                    parameters,
                    request_context: None,
                };
                if let Some(value) =
                    self.resolve_entry_from_cache(group, entry, lang, &lookup_opts)?
                {
                    Ok(value)
                } else {
                    Err(entry_miss_error(
                        &self.opts.default_project_id,
                        lang,
                        group,
                        entry,
                    ))
                }
            }
            Err(err) => Err(err),
        }
    }

    async fn get_entry_cache_only(
        &self,
        group: &str,
        entry: &str,
        lang: &str,
        opts: GetEntryOptions<'_>,
    ) -> Result<String, Error> {
        if let Some(value) = self.resolve_entry_from_cache(group, entry, lang, &opts)? {
            Ok(value)
        } else {
            Err(entry_miss_error(
                &self.opts.default_project_id,
                lang,
                group,
                entry,
            ))
        }
    }

    fn resolve_entry_from_cache(
        &self,
        group: &str,
        entry: &str,
        lang: &str,
        opts: &GetEntryOptions<'_>,
    ) -> Result<Option<String>, Error> {
        let cached_group = self
            .cache
            .get_group(&self.opts.default_project_id, group, lang)
            .map_err(Error::from)?;

        let Some(group_data) = cached_group else {
            return Ok(None);
        };

        Ok(resolve_entry_from_group(
            &group_data,
            entry,
            opts.number,
            &opts.parameters,
        ))
    }

    async fn get_group_cache_first(
        &self,
        project: &str,
        group: &str,
        lang: &str,
        opts: GetGroupOptions<'_>,
    ) -> Result<TranslationGroup, Error> {
        if let Some(cached) = self
            .cache
            .get_group(project, group, lang)
            .map_err(Error::from)?
        {
            return Ok(cached);
        }

        let result = self.inner.get_group(project, group, lang, opts).await;
        match result {
            Ok(group_data) => {
                try_update_group_cache(&self.inner, &self.cache, project, group, lang).await;
                Ok(group_data)
            }
            Err(err) if is_network_or_api_error(&err) => {
                Err(group_miss_error(project, lang, group))
            }
            Err(err) => Err(err),
        }
    }

    async fn get_group_api_first(
        &self,
        project: &str,
        group: &str,
        lang: &str,
        opts: GetGroupOptions<'_>,
    ) -> Result<TranslationGroup, Error> {
        match self.inner.get_group(project, group, lang, opts).await {
            Ok(group_data) => {
                try_update_group_cache(&self.inner, &self.cache, project, group, lang).await;
                Ok(group_data)
            }
            Err(err) if is_network_or_api_error(&err) => {
                if let Some(cached) = self
                    .cache
                    .get_group(project, group, lang)
                    .map_err(Error::from)?
                {
                    Ok(cached)
                } else {
                    Err(group_miss_error(project, lang, group))
                }
            }
            Err(err) => Err(err),
        }
    }

    async fn get_group_cache_only(
        &self,
        project: &str,
        group: &str,
        lang: &str,
    ) -> Result<TranslationGroup, Error> {
        if let Some(cached) = self
            .cache
            .get_group(project, group, lang)
            .map_err(Error::from)?
        {
            Ok(cached)
        } else {
            Err(group_miss_error(project, lang, group))
        }
    }

    async fn get_project_cache_first(
        &self,
        project: &str,
        lang: &str,
        opts: GetProjectOptions<'_>,
    ) -> Result<TranslationProject, Error> {
        if let Some(cached) = self.cache.get_project(project, lang).map_err(Error::from)? {
            return Ok(cached);
        }

        let result = self.inner.get_project(project, lang, opts).await;
        match result {
            Ok(project_data) => {
                self.cache
                    .save_project(project, lang, &project_data, SaveOptions::new())
                    .map_err(Error::from)?;
                Ok(project_data)
            }
            Err(err) if is_network_or_api_error(&err) => Err(project_miss_error(project, lang)),
            Err(err) => Err(err),
        }
    }

    async fn get_project_api_first(
        &self,
        project: &str,
        lang: &str,
        opts: GetProjectOptions<'_>,
    ) -> Result<TranslationProject, Error> {
        match self.inner.get_project(project, lang, opts).await {
            Ok(result) => {
                self.cache
                    .save_project(project, lang, &result, SaveOptions::new())
                    .map_err(Error::from)?;
                Ok(result)
            }
            Err(err) if is_network_or_api_error(&err) => {
                if let Some(cached) = self.cache.get_project(project, lang).map_err(Error::from)? {
                    Ok(cached)
                } else {
                    Err(project_miss_error(project, lang))
                }
            }
            Err(err) => Err(err),
        }
    }

    async fn get_project_cache_only(
        &self,
        project: &str,
        lang: &str,
    ) -> Result<TranslationProject, Error> {
        if let Some(cached) = self.cache.get_project(project, lang).map_err(Error::from)? {
            Ok(cached)
        } else {
            Err(project_miss_error(project, lang))
        }
    }

    async fn get_project_locales_cache_first(
        &self,
        project: &str,
        opts: GetProjectLocalesOptions<'_>,
    ) -> Result<ProjectLocales, Error> {
        if let Some(cached) = self.cache.get_locales(project).map_err(Error::from)? {
            return Ok(cached);
        }

        let result = self.inner.get_project_locales(project, opts).await;
        match result {
            Ok(locales) => {
                self.cache
                    .save_locales(project, &locales, SaveOptions::new())
                    .map_err(Error::from)?;
                Ok(locales)
            }
            Err(err) if is_network_or_api_error(&err) => {
                Err(locales_offline_cache_error(project, Some(&err)))
            }
            Err(err) => Err(err),
        }
    }

    async fn get_project_locales_api_first(
        &self,
        project: &str,
        opts: GetProjectLocalesOptions<'_>,
    ) -> Result<ProjectLocales, Error> {
        match self.inner.get_project_locales(project, opts).await {
            Ok(locales) => {
                self.cache
                    .save_locales(project, &locales, SaveOptions::new())
                    .map_err(Error::from)?;
                Ok(locales)
            }
            Err(err) if is_network_or_api_error(&err) => {
                if let Some(cached) = self.cache.get_locales(project).map_err(Error::from)? {
                    Ok(cached)
                } else {
                    Err(locales_offline_cache_error(project, Some(&err)))
                }
            }
            Err(err) => Err(err),
        }
    }

    async fn get_project_locales_cache_only(&self, project: &str) -> Result<ProjectLocales, Error> {
        if let Some(cached) = self.cache.get_locales(project).map_err(Error::from)? {
            Ok(cached)
        } else {
            Err(locales_offline_cache_error(project, None))
        }
    }
}

impl<C, P> TranslaasClient for CachingClient<C, P>
where
    C: TranslaasClient,
    P: Provider,
{
    async fn get_entry(
        &self,
        group: &str,
        entry: &str,
        lang: &str,
        opts: GetEntryOptions<'_>,
    ) -> Result<String, Error> {
        match self.opts.fallback_mode {
            FallbackMode::ApiFirst => self.get_entry_api_first(group, entry, lang, opts).await,
            FallbackMode::CacheOnly => self.get_entry_cache_only(group, entry, lang, opts).await,
            FallbackMode::CacheFirst => self.get_entry_cache_first(group, entry, lang, opts).await,
        }
    }

    async fn get_group(
        &self,
        project: &str,
        group: &str,
        lang: &str,
        opts: GetGroupOptions<'_>,
    ) -> Result<TranslationGroup, Error> {
        match self.opts.fallback_mode {
            FallbackMode::ApiFirst => self.get_group_api_first(project, group, lang, opts).await,
            FallbackMode::CacheOnly => self.get_group_cache_only(project, group, lang).await,
            FallbackMode::CacheFirst => {
                self.get_group_cache_first(project, group, lang, opts).await
            }
        }
    }

    async fn get_project(
        &self,
        project: &str,
        lang: &str,
        opts: GetProjectOptions<'_>,
    ) -> Result<TranslationProject, Error> {
        match self.opts.fallback_mode {
            FallbackMode::ApiFirst => self.get_project_api_first(project, lang, opts).await,
            FallbackMode::CacheOnly => self.get_project_cache_only(project, lang).await,
            FallbackMode::CacheFirst => self.get_project_cache_first(project, lang, opts).await,
        }
    }

    async fn get_project_locales(
        &self,
        project: &str,
        opts: GetProjectLocalesOptions<'_>,
    ) -> Result<ProjectLocales, Error> {
        match self.opts.fallback_mode {
            FallbackMode::ApiFirst => self.get_project_locales_api_first(project, opts).await,
            FallbackMode::CacheOnly => self.get_project_locales_cache_only(project).await,
            FallbackMode::CacheFirst => self.get_project_locales_cache_first(project, opts).await,
        }
    }

    async fn get_offline_cache(
        &self,
        project: &str,
        opts: GetOfflineCacheOptions<'_>,
    ) -> Result<OfflineCacheDownloadResult, Error> {
        self.inner.get_offline_cache(project, opts).await
    }

    async fn report_missing_keys(&self, keys: &[ReportMissingKeyItem]) -> Result<(), Error> {
        self.inner.report_missing_keys(keys).await
    }

    async fn validate_api_key(&self) -> Result<ValidateApiKeyResponse, Error> {
        self.inner.validate_api_key().await
    }
}

fn entry_miss_error(project: &str, lang: &str, group: &str, entry: &str) -> Error {
    Error::from(OfflineCacheMissError::new_offline_cache_miss_error(
        project, lang, group, entry,
    ))
}

fn group_miss_error(project: &str, lang: &str, group: &str) -> Error {
    Error::from(OfflineCacheMissError::new_offline_cache_miss_error(
        project, lang, group, "",
    ))
}

fn project_miss_error(project: &str, lang: &str) -> Error {
    Error::from(OfflineCacheMissError::new_offline_cache_miss_error(
        project, lang, "", "",
    ))
}

fn locales_offline_cache_error(project: &str, cause: Option<&Error>) -> Error {
    let message = if cause.is_some() {
        format!(
            "Project locales for '{project}' not found in the offline cache and API is unavailable."
        )
    } else {
        format!("Project locales for '{project}' not found in the offline cache.")
    };
    Error::from(OfflineCacheError::new(
        message,
        None,
        Some(project.to_string()),
        None,
        None,
    ))
}
