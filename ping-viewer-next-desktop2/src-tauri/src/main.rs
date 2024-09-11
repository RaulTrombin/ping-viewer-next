// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use ping_viewer_next::{cli, device, logger, server};
use tauri::Manager;

#[tokio::main]
async fn main() {
    // CLI should be started before logger to allow control over verbosity
    cli::manager::init();
    // Logger should start before everything else to register any log information
    logger::manager::init();

    let (manager, handler) = device::manager::DeviceManager::new(10);

    tokio::spawn(async move { manager.run().await });

    run_tauri_app(handler).await;
}

async fn run_tauri_app(handler: device::manager::ManagerActorHandler) {
    tauri::Builder::default()
        .setup(|app: &mut tauri::App| {
            // Load the preloading window first
            let window = app.get_window("main").unwrap();
            let _ = window.eval("window.location.replace('preload.html')");

            // Spawn a new thread to start the server
            std::thread::spawn(move || {
                run_from_tauri(&cli::manager::server_address(), handler).unwrap();
            });

            // Replace the loading screen with the server content once ready
            std::thread::spawn(move || {
                // Wait for the server to be ready, add delay if necessary
                std::thread::sleep(std::time::Duration::from_secs(10)); // Example delay
                window.eval("window.location.replace('http://0.0.0.0:8080')").unwrap();
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[actix_web::main]
pub async fn run_from_tauri(
    server_address: &str,
    handler: device::manager::ManagerActorHandler,
) -> std::io::Result<()> {
    server::manager::run(server_address, handler).await
}
