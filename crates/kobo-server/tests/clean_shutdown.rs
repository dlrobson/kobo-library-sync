//! Tests clean shutdown of the server.

use std::sync::Arc;

use kobo_server::{App, CommandLineArguments};

#[tokio::test]
async fn test_clean_shutdown() {
    let app = Arc::new(App::new());
    // Create command line arguments with port 0 to get a random available port
    let args = CommandLineArguments {
        port: 0,
        ..Default::default()
    };

    // Start the app in a background task
    let app_clone = app.clone();
    let app_handle = tokio::spawn(async move { app_clone.run(args).await });

    // Cancel the task to simulate shutdown
    app.shutdown();

    app_handle
        .await
        .unwrap()
        .expect("App did not shut down cleanly");
}
