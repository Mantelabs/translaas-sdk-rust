//! [`ClientBuilder`](crate::client::ClientBuilder) extension that returns [`Service`].

use crate::client::ClientBuilder;

use super::{Error, Service};

impl ClientBuilder {
    /// Builds a [`Service`] wrapping the live HTTP client.
    ///
    /// Uses [`crate::client::Client::default_language`] for automatic locale selection
    /// when no custom resolver is configured. Equivalent to `Service::new(self.build()?)`.
    pub fn build_service(self) -> Result<Service<crate::client::Client>, Error> {
        let client = self.build().map_err(crate::client::Error::from)?;
        Ok(Service::new(client))
    }
}
