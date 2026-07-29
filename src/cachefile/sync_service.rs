//! Offline cache synchronization with the Translaas API.

use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;

use chrono::Utc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::client::{
    Error, GetOfflineCacheOptions, GetProjectLocalesOptions, GetProjectOptions, TranslaasClient,
};
use crate::models::ConfigurationError;

use super::offline_cache_options::OfflineCacheOptions;
use super::provider::{Provider, SaveOptions};
use super::sync_events::{SyncCallbacks, SyncCompletedEvent, SyncFailedEvent, SyncResult};
use super::sync_language_filter::filter_sync_languages;
use super::zip_bundle::{apply_offline_bundle, parse_offline_zip, resolve_project_key};

struct BackgroundSyncState {
    cancel: Option<CancellationToken>,
    thread: Option<std::thread::JoinHandle<()>>,
    running: Arc<std::sync::atomic::AtomicBool>,
}

impl Default for BackgroundSyncState {
    fn default() -> Self {
        Self {
            cancel: None,
            thread: None,
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }
}

/// Synchronizes offline cache files with the Translaas API via the inner client.
///
/// Wire this with the **inner** [`TranslaasClient`], not [`super::CachingClient`].
/// Disk writes call synchronous [`Provider`] methods inline on the async task thread;
/// wrap at the app layer with [`tokio::task::spawn_blocking`] if isolation is required.
pub struct SyncService<C, P> {
    client: C,
    cache: P,
    options: OfflineCacheOptions,
    callbacks: SyncCallbacks,
    sync_mu: Mutex<()>,
    bg: StdMutex<BackgroundSyncState>,
}

impl<C, P> SyncService<C, P> {
    /// Constructs a sync service using the inner HTTP client and cache provider.
    pub fn new(
        client: C,
        cache: P,
        options: OfflineCacheOptions,
        callbacks: SyncCallbacks,
    ) -> Self {
        Self {
            client,
            cache,
            options,
            callbacks,
            sync_mu: Mutex::new(()),
            bg: StdMutex::new(BackgroundSyncState::default()),
        }
    }

    /// Returns a reference to the inner client.
    pub fn client(&self) -> &C {
        &self.client
    }

    /// Returns a reference to the cache provider.
    pub fn cache(&self) -> &P {
        &self.cache
    }

    /// Returns the offline cache options.
    pub fn options(&self) -> &OfflineCacheOptions {
        &self.options
    }
}

impl<C, P> SyncService<C, P>
where
    C: TranslaasClient + Send + Sync + 'static,
    P: Provider + Send + Sync + 'static,
{
    /// Fetches one project language from the API and persists it to disk.
    pub async fn sync_project(
        &self,
        project: &str,
        lang: &str,
        cancel: &CancellationToken,
    ) -> Result<(), Error> {
        let project = project.trim();
        if project.is_empty() {
            return Err(Error::Configuration(ConfigurationError {
                message: "cachefile: project must not be empty".to_string(),
            }));
        }
        let lang = lang.trim();
        if lang.is_empty() {
            return Err(Error::Configuration(ConfigurationError {
                message: "cachefile: language must not be empty".to_string(),
            }));
        }

        if cancel.is_cancelled() {
            return Err(Error::Canceled);
        }

        let _guard = self.sync_mu.lock().await;

        match self
            .client
            .get_project(project, lang, GetProjectOptions::new())
            .await
        {
            Ok(project_data) => {
                if let Err(err) =
                    self.cache
                        .save_project(project, lang, &project_data, SaveOptions::new())
                {
                    let error = Error::from(err);
                    self.emit_sync_failed(SyncFailedEvent {
                        project: project.to_string(),
                        language: lang.to_string(),
                        error: Error::Configuration(ConfigurationError {
                            message: error.to_string(),
                        }),
                    });
                    return Err(error);
                }

                self.emit_sync_completed(SyncCompletedEvent {
                    project: project.to_string(),
                    language: lang.to_string(),
                    synced_at: Utc::now(),
                });
                Ok(())
            }
            Err(err) => {
                self.emit_sync_failed(SyncFailedEvent {
                    project: project.to_string(),
                    language: lang.to_string(),
                    error: Error::Configuration(ConfigurationError {
                        message: err.to_string(),
                    }),
                });
                Err(err)
            }
        }
    }

    /// Fetches locales and syncs each configured language for a project.
    pub async fn sync_project_all_languages(
        &self,
        project: &str,
        cancel: &CancellationToken,
    ) -> Result<(), Error> {
        let project = project.trim();
        if project.is_empty() {
            return Err(Error::Configuration(ConfigurationError {
                message: "cachefile: project must not be empty".to_string(),
            }));
        }

        if cancel.is_cancelled() {
            return Err(Error::Canceled);
        }

        let _guard = self.sync_mu.lock().await;

        let locales = self
            .client
            .get_project_locales(project, GetProjectLocalesOptions::new())
            .await?;

        self.cache
            .save_locales(project, &locales, SaveOptions::new())
            .map_err(Error::from)?;

        let languages = filter_sync_languages(&locales.locales, &self.options.languages);

        for lang in languages {
            if cancel.is_cancelled() {
                return Err(Error::Canceled);
            }

            match self
                .client
                .get_project(project, &lang, GetProjectOptions::new())
                .await
            {
                Ok(project_data) => {
                    if let Err(err) =
                        self.cache
                            .save_project(project, &lang, &project_data, SaveOptions::new())
                    {
                        let error = Error::from(err);
                        self.emit_sync_failed(SyncFailedEvent {
                            project: project.to_string(),
                            language: lang.clone(),
                            error: Error::Configuration(ConfigurationError {
                                message: error.to_string(),
                            }),
                        });
                        continue;
                    }

                    self.emit_sync_completed(SyncCompletedEvent {
                        project: project.to_string(),
                        language: lang,
                        synced_at: Utc::now(),
                    });
                }
                Err(err) => {
                    self.emit_sync_failed(SyncFailedEvent {
                        project: project.to_string(),
                        language: lang,
                        error: Error::Configuration(ConfigurationError {
                            message: err.to_string(),
                        }),
                    });
                }
            }
        }

        Ok(())
    }

    /// Synchronizes every project listed in [`OfflineCacheOptions::projects`].
    pub async fn sync_all(&self, cancel: &CancellationToken) -> Result<SyncResult, Error> {
        if cancel.is_cancelled() {
            return Err(Error::Canceled);
        }

        let mut result = SyncResult {
            synced_projects: Vec::with_capacity(self.options.projects.len()),
            failed_projects: Vec::new(),
            completed_at: Utc::now(),
        };

        for project in &self.options.projects {
            if cancel.is_cancelled() {
                return Err(Error::Canceled);
            }

            match self.sync_project_all_languages(project, cancel).await {
                Ok(()) => result.synced_projects.push(project.clone()),
                Err(_) => result.failed_projects.push(project.clone()),
            }
        }

        result.completed_at = Utc::now();
        self.emit_sync_all_completed(result.clone());
        Ok(result)
    }

    /// Downloads the offline ZIP for `project` via the inner client and imports it.
    ///
    /// Returns `Ok(())` when the download is **304 Not Modified** or the body is empty.
    pub async fn sync_from_offline_zip(
        &self,
        project: &str,
        cancel: &CancellationToken,
    ) -> Result<(), Error> {
        let project = project.trim();
        if project.is_empty() {
            return Err(Error::Configuration(ConfigurationError {
                message: "cachefile: project must not be empty".to_string(),
            }));
        }

        if cancel.is_cancelled() {
            return Err(Error::Canceled);
        }

        let _guard = self.sync_mu.lock().await;

        let result = match self
            .client
            .get_offline_cache(project, GetOfflineCacheOptions::new())
            .await
        {
            Ok(result) => result,
            Err(err) => {
                self.emit_sync_failed(SyncFailedEvent {
                    project: project.to_string(),
                    language: String::new(),
                    error: Error::Configuration(ConfigurationError {
                        message: err.to_string(),
                    }),
                });
                return Err(err);
            }
        };

        if result.not_modified
            || result
                .content
                .as_ref()
                .is_none_or(|content| content.is_empty())
        {
            return Ok(());
        }

        let content = result.content.as_ref().expect("checked non-empty above");

        let bundle = match parse_offline_zip(content) {
            Ok(bundle) => bundle,
            Err(err) => {
                let error = Error::from(err);
                self.emit_sync_failed(SyncFailedEvent {
                    project: project.to_string(),
                    language: String::new(),
                    error: Error::Configuration(ConfigurationError {
                        message: error.to_string(),
                    }),
                });
                return Err(error);
            }
        };

        let key = match resolve_project_key(&bundle, project) {
            Ok(key) => key,
            Err(err) => {
                let error = Error::from(err);
                self.emit_sync_failed(SyncFailedEvent {
                    project: project.to_string(),
                    language: String::new(),
                    error: Error::Configuration(ConfigurationError {
                        message: error.to_string(),
                    }),
                });
                return Err(error);
            }
        };

        if let Err(err) = apply_offline_bundle(&self.cache, project, &key, &bundle) {
            let error = Error::from(err);
            self.emit_sync_failed(SyncFailedEvent {
                project: project.to_string(),
                language: String::new(),
                error: Error::Configuration(ConfigurationError {
                    message: error.to_string(),
                }),
            });
            return Err(error);
        }

        self.emit_sync_completed(SyncCompletedEvent {
            project: project.to_string(),
            language: String::new(),
            synced_at: Utc::now(),
        });
        Ok(())
    }

    /// Runs an initial [`sync_all`], then repeats on the configured interval until cancelled.
    ///
    /// No-op when `auto_sync` is false, `auto_sync_interval` is `None`, or a loop is already
    /// running. Requires `self: &Arc<Self>` so the background task can hold a reference.
    pub fn start_background_sync(self: &Arc<Self>, parent: CancellationToken) {
        if !self.options.auto_sync {
            return;
        }
        let Some(interval) = self.options.auto_sync_interval else {
            return;
        };

        let mut bg = match self.bg.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };
        if bg.running.load(std::sync::atomic::Ordering::SeqCst) {
            return;
        }

        let child = parent.child_token();
        bg.cancel = Some(child.clone());
        bg.running.store(true, std::sync::atomic::Ordering::SeqCst);

        let service = Arc::clone(self);
        let running = Arc::clone(&bg.running);
        let handle = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_time()
                .build()
                .expect("background sync runtime");
            rt.block_on(run_background_sync(service, child, interval));
            running.store(false, std::sync::atomic::Ordering::SeqCst);
        });
        bg.thread = Some(handle);
    }

    /// Cancels the background loop and waits for it to exit.
    pub async fn stop_background_sync(&self) {
        let (cancel, thread) = {
            let mut bg = self.bg.lock().expect("background sync mutex poisoned");
            (bg.cancel.take(), bg.thread.take())
        };

        if let Some(cancel) = cancel {
            cancel.cancel();
        }

        if let Some(thread) = thread {
            let _ = tokio::task::spawn_blocking(move || thread.join()).await;
        }
    }

    /// Reports whether a background sync loop started by [`Self::start_background_sync`] is active.
    pub fn is_background_sync_running(&self) -> bool {
        let Ok(bg) = self.bg.lock() else {
            return false;
        };
        bg.running.load(std::sync::atomic::Ordering::SeqCst)
    }

    fn emit_sync_completed(&self, event: SyncCompletedEvent) {
        if let Some(ref callback) = self.callbacks.on_sync_completed {
            callback(event);
        }
    }

    fn emit_sync_failed(&self, event: SyncFailedEvent) {
        if let Some(ref callback) = self.callbacks.on_sync_failed {
            callback(event);
        }
    }

    fn emit_sync_all_completed(&self, result: SyncResult) {
        if let Some(ref callback) = self.callbacks.on_sync_all_completed {
            callback(result);
        }
    }
}

async fn run_background_sync<C, P>(
    service: Arc<SyncService<C, P>>,
    cancel: CancellationToken,
    interval: Duration,
) where
    C: TranslaasClient + Send + Sync + 'static,
    P: Provider + Send + Sync + 'static,
{
    let run_once = || async {
        if let Err(err) = service.sync_all(&cancel).await {
            if cancel.is_cancelled() {
                return;
            }
            let _ = err;
        }
    };

    run_once().await;

    let mut ticker = tokio::time::interval(interval);
    loop {
        tokio::select! {
            () = cancel.cancelled() => break,
            _ = ticker.tick() => run_once().await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::{
        GetEntryOptions, GetGroupOptions, GetOfflineCacheOptions, GetProjectLocalesOptions,
        GetProjectOptions, TranslaasClient,
    };
    use crate::models::{
        OfflineCacheDownloadResult, ProjectLocales, ReportMissingKeyItem, TranslationGroup,
        TranslationProject, ValidateApiKeyResponse,
    };

    struct NoopClient;

    impl TranslaasClient for NoopClient {
        async fn get_entry(
            &self,
            _: &str,
            _: &str,
            _: &str,
            _: GetEntryOptions<'_>,
        ) -> Result<String, Error> {
            unreachable!()
        }

        async fn get_group(
            &self,
            _: &str,
            _: &str,
            _: &str,
            _: GetGroupOptions<'_>,
        ) -> Result<TranslationGroup, Error> {
            unreachable!()
        }

        async fn get_project(
            &self,
            _: &str,
            _: &str,
            _: GetProjectOptions<'_>,
        ) -> Result<TranslationProject, Error> {
            unreachable!()
        }

        async fn get_project_locales(
            &self,
            _: &str,
            _: GetProjectLocalesOptions<'_>,
        ) -> Result<ProjectLocales, Error> {
            unreachable!()
        }

        async fn get_offline_cache(
            &self,
            _: &str,
            _: GetOfflineCacheOptions<'_>,
        ) -> Result<OfflineCacheDownloadResult, Error> {
            unreachable!()
        }

        async fn report_missing_keys(&self, _: &[ReportMissingKeyItem]) -> Result<(), Error> {
            unreachable!()
        }

        async fn validate_api_key(&self) -> Result<ValidateApiKeyResponse, Error> {
            unreachable!()
        }
    }

    struct NoopCache;

    impl Provider for NoopCache {
        fn get_project(
            &self,
            _: &str,
            _: &str,
        ) -> Result<Option<TranslationProject>, crate::models::OfflineCacheError> {
            Ok(None)
        }

        fn save_project(
            &self,
            _: &str,
            _: &str,
            _: &TranslationProject,
            _: SaveOptions,
        ) -> Result<(), crate::models::OfflineCacheError> {
            Ok(())
        }

        fn get_group(
            &self,
            _: &str,
            _: &str,
            _: &str,
        ) -> Result<Option<TranslationGroup>, crate::models::OfflineCacheError> {
            Ok(None)
        }

        fn get_locales(
            &self,
            _: &str,
        ) -> Result<Option<ProjectLocales>, crate::models::OfflineCacheError> {
            Ok(None)
        }

        fn save_locales(
            &self,
            _: &str,
            _: &ProjectLocales,
            _: SaveOptions,
        ) -> Result<(), crate::models::OfflineCacheError> {
            Ok(())
        }

        fn get_manifest(
            &self,
        ) -> Result<Option<super::super::types::CacheManifest>, crate::models::OfflineCacheError>
        {
            Ok(None)
        }

        fn update_manifest(
            &self,
            _: &mut dyn FnMut(
                &mut super::super::types::CacheManifest,
            ) -> Result<(), crate::models::OfflineCacheError>,
        ) -> Result<(), crate::models::OfflineCacheError> {
            Ok(())
        }

        fn is_cached(&self, _: &str, _: &str) -> Result<bool, crate::models::OfflineCacheError> {
            Ok(false)
        }

        fn clear(&self) -> Result<(), crate::models::OfflineCacheError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn sync_project_rejects_empty_project_or_language() {
        let svc = SyncService::new(
            NoopClient,
            NoopCache,
            OfflineCacheOptions::default_offline_cache_options(),
            SyncCallbacks::default(),
        );
        let cancel = CancellationToken::new();

        assert!(svc.sync_project("", "en", &cancel).await.is_err());
        assert!(svc.sync_project("demo", "", &cancel).await.is_err());
    }
}
