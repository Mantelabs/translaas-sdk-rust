//! Compile-check for README quickstart snippets (see README.md § Quick start).
//!
//! Does not call the live API — verifies types and builder chains only.

#[test]
fn readme_option_a_service_quickstart_compiles() {
    fn assert_send<T: Send>(_value: T) {}

    assert_send(async {
        use translaas::cache::CacheMode;
        use translaas::client::ClientBuilder;
        use translaas::service::{
            DefaultLanguageProvider, LanguageResolver, Service, ServiceOptions, TOptions,
        };

        let client = ClientBuilder::new()
            .api_key("test-key")
            .base_url("https://sdk-api.translaas.local")
            .default_project_id("test-project")
            .cache_mode(CacheMode::Group)
            .build()?;

        let resolver = LanguageResolver::new([DefaultLanguageProvider::new("en")])?;
        let service = Service::new(
            client,
            ServiceOptions {
                resolver: Some(resolver),
            },
        );

        let text = service
            .t("ui", "button.save", TOptions::new().lang("en"))
            .await?;
        println!("{text}");
        Ok::<(), Box<dyn std::error::Error>>(())
    });
}

#[test]
fn readme_option_b_client_quickstart_compiles() {
    fn assert_send<T: Send>(_value: T) {}

    assert_send(async {
        use translaas::client::{Client, GetEntryOptions};

        let client = Client::builder()
            .api_key("test-key")
            .base_url("https://api.translaas.local")
            .default_project_id("test-project")
            .build()?;

        let text = client
            .get_entry("ui", "greeting", "en", GetEntryOptions::new())
            .await?;
        println!("{text}");
        Ok::<(), Box<dyn std::error::Error>>(())
    });
}
