use crate::server::listener::{fake_listener::FakeListener, into_listener::IntoListener};

pub struct FakeListenerBuilder;

#[async_trait::async_trait]
impl IntoListener for FakeListenerBuilder {
    type Listener = FakeListener;

    async fn into_listener(self, port: u16) -> anyhow::Result<Self::Listener> {
        Ok(FakeListener::new(port))
    }
}
