//! Compile-check for README quickstart snippets (see README.md § Quick start).
//!
//! Does not call the live API — verifies types and builder chains only.

#[test]
fn readme_option_a_service_quickstart_compiles() {
    fn assert_send<T: Send>(_value: T) {}

    assert_send(async {
        use translaas::{ClientBuilder, Service};

        let translaas: Service<_> = ClientBuilder::new()
            .api_key("test-key")
            .base_url("https://api.translaas.local")
            .default_project_id("translaassdksamples")
            .default_language("en")
            .accept_invalid_certs(true)
            .build_service()?;

        let text = translaas.t_lang("common", "welcome.message", "en").await?;
        let also = translaas.t("common", "welcome.message").await?;
        println!("{text}{also}");
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
