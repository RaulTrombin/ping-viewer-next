// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use ping_viewer_next::{cli, device, logger, server};
use tauri::Manager;
use tauri::path::BaseDirectory;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[tokio::main]
async fn main() {
    cli::manager::init();

    logger::manager::init();

    let (manager, handler) = device::manager::DeviceManager::new(10);

    let (recordings_manager, recordings_manager_handler) =
    device::recording::RecordingManager::new(10, "recordings", handler.clone());
    tokio::spawn(async move { recordings_manager.run().await });

    tokio::spawn(async move { manager.run().await });

    run_tauri_app(handler, recordings_manager_handler).await;
}

async fn run_tauri_app(handler: device::manager::ManagerActorHandler, recordings_handler: device::recording::RecordingsManagerHandler) {
    tauri::Builder::default()
        .setup(|app: &mut tauri::App| {
            // Initialize bundled firmwares into current working directory if missing
            if let Err(e) = ensure_default_firmwares(&app.handle()) {
                eprintln!("Failed to initialize default firmwares: {}", e);
            }

            let window = app.get_webview_window("main").unwrap();

            std::thread::spawn(move || {
                run_from_tauri(&cli::manager::server_address(), handler, recordings_handler).unwrap();
            });

            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(6));
                window.eval("window.location.replace('http://127.0.0.1:8080')").unwrap();
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn ensure_default_firmwares(app: &tauri::AppHandle) -> io::Result<()> {
    // Resolve bundled resources/firmwares path
    let resource_firmwares: PathBuf = app
        .path()
        .resolve("firmwares", BaseDirectory::Resource)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("resolve resource path: {}", e)))?;

    // Destination is the process current working directory ./firmwares
    let dest_base = std::env::current_dir()?.join("firmwares");

    // Copy only if missing or empty (preserve user changes)
    let needs_copy = !dest_base.exists() || is_dir_empty(&dest_base)?;
    if needs_copy {
        copy_dir_recursively(&resource_firmwares, &dest_base)?;
        // Ensure utils binaries are executable on Unix
        set_exec_if_exists(dest_base.join("utils").join("ping360-bootloader"))?;
        set_exec_if_exists(dest_base.join("utils").join("stm32flash"))?;
    }
    Ok(())
}

fn is_dir_empty(path: &Path) -> io::Result<bool> {
    if !path.exists() {
        return Ok(true);
    }
    let mut entries = fs::read_dir(path)?;
    Ok(entries.next().is_none())
}

fn copy_dir_recursively(src: &Path, dst: &Path) -> io::Result<()> {
    if !src.exists() {
        return Ok(()); // nothing to copy
    }
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursively(&src_path, &dst_path)?;
        } else {
            if let Some(parent) = dst_path.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)?;
                }
            }
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

fn set_exec_if_exists(path: PathBuf) -> io::Result<()> {
    if !path.exists() {
        return Ok(());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&path, perms)?;
    }
    Ok(())
}

#[actix_web::main]
pub async fn run_from_tauri(
    server_address: &str,
    handler: device::manager::ManagerActorHandler,
    recordings_handler: device::recording::RecordingsManagerHandler
) -> std::io::Result<()> {
    server::manager::run(server_address, handler, recordings_handler).await
}
