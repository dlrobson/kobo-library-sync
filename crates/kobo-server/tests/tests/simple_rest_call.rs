//! Integration tests that make simple REST calls to the server.

use std::sync::Arc;

use kobo_server::{
    App,
    CommandLineArguments,
};

#[tokio::test]
async fn test_hello_world_endpoint_integration() {
    let app = Arc::new(App::new());

    let args = CommandLineArguments {
        port: 0,
        ..Default::default()
    };

    // Start the server
    let app_clone = app.clone();
    let app_handle = tokio::spawn(async move { app_clone.run(args).await });
    app.wait_until_running().await;
    let app_port = app
        .server_address()
        .expect("Server should be running")
        .port();

    // Make an HTTP request to the hello world endpoint
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{app_port}/");

    let response = client
        .get(&url)
        .send()
        .await
        .expect("Failed to send request");

    // Requests forwarded to the kobo server without authentication will be rejected
    assert_eq!(response.status(), 401);

    app.shutdown().await.expect("Should shutdown cleanly");
    app_handle
        .await
        .unwrap()
        .expect("App did not shut down cleanly");
}
