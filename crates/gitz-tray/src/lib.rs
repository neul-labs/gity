//! Cross-platform system tray for gitz daemon.
//!
//! Provides a minimal tray icon with Info and Exit actions.

use gitz_ipc::{DaemonCommand, DaemonResponse, DaemonService};
use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use thiserror::Error;
use tray_icon::{TrayIcon, TrayIconBuilder};

#[derive(Debug, Error)]
pub enum TrayError {
    #[error("failed to create tray: {0}")]
    Creation(String),
    #[error("failed to create menu: {0}")]
    Menu(String),
    #[error("platform error: {0}")]
    Platform(String),
}

/// Menu item IDs
const INFO_ID: &str = "info";
const EXIT_ID: &str = "exit";

/// Configuration for the system tray.
pub struct TrayConfig {
    pub daemon_address: String,
}

/// System tray instance.
pub struct GitzTray {
    _tray: TrayIcon,
    running: Arc<AtomicBool>,
}

impl GitzTray {
    /// Create and show the system tray icon.
    pub fn new(config: TrayConfig) -> Result<Self, TrayError> {
        let running = Arc::new(AtomicBool::new(true));

        // Create the menu
        let menu = create_menu()?;

        // Create tray icon with embedded icon data
        let icon = load_icon()?;
        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("Gitz - Git Helper Daemon")
            .with_icon(icon)
            .build()
            .map_err(|e| TrayError::Creation(e.to_string()))?;

        // Set up menu event handler
        let running_clone = Arc::clone(&running);
        let daemon_address = config.daemon_address;
        std::thread::spawn(move || {
            loop {
                if let Ok(event) = MenuEvent::receiver().recv() {
                    match event.id.0.as_str() {
                        INFO_ID => {
                            handle_info_action(&daemon_address);
                        }
                        EXIT_ID => {
                            running_clone.store(false, Ordering::SeqCst);
                            break;
                        }
                        _ => {}
                    }
                }
            }
        });

        Ok(Self {
            _tray: tray,
            running,
        })
    }

    /// Check if the tray is still running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Request shutdown of the tray.
    pub fn shutdown(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

fn create_menu() -> Result<Menu, TrayError> {
    let menu = Menu::new();

    let info_item = MenuItem::with_id(INFO_ID, "Info", true, None);
    let separator = PredefinedMenuItem::separator();
    let exit_item = MenuItem::with_id(EXIT_ID, "Exit", true, None);

    menu.append(&info_item)
        .map_err(|e| TrayError::Menu(e.to_string()))?;
    menu.append(&separator)
        .map_err(|e| TrayError::Menu(e.to_string()))?;
    menu.append(&exit_item)
        .map_err(|e| TrayError::Menu(e.to_string()))?;

    Ok(menu)
}

fn load_icon() -> Result<tray_icon::Icon, TrayError> {
    // Create a simple 32x32 blue square icon
    let width = 32u32;
    let height = 32u32;
    let mut rgba = vec![0u8; (width * height * 4) as usize];

    // Fill with a blue color
    for i in 0..((width * height) as usize) {
        rgba[i * 4] = 64;      // R
        rgba[i * 4 + 1] = 128; // G
        rgba[i * 4 + 2] = 255; // B
        rgba[i * 4 + 3] = 255; // A
    }

    tray_icon::Icon::from_rgba(rgba, width, height)
        .map_err(|e| TrayError::Creation(e.to_string()))
}

fn handle_info_action(daemon_address: &str) {
    // Create a runtime for the async call
    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("failed to create runtime: {e}");
            return;
        }
    };

    rt.block_on(async {
        let client = gitz_daemon::NngClient::new(daemon_address.to_string());
        match client.execute(DaemonCommand::HealthCheck).await {
            Ok(DaemonResponse::Health(health)) => {
                let info = format!(
                    "Gitz Daemon Status\n\
                     ------------------\n\
                     Repositories: {}\n\
                     Pending Jobs: {}\n\
                     Uptime: {}s",
                    health.repo_count, health.pending_jobs, health.uptime_seconds
                );
                show_notification("Gitz Info", &info);
            }
            Ok(response) => {
                show_notification("Gitz Info", &format!("Unexpected response: {:?}", response));
            }
            Err(e) => {
                show_notification("Gitz Error", &format!("Daemon not running: {}", e));
            }
        }
    });
}

fn show_notification(title: &str, message: &str) {
    // Simple console output for now - can be replaced with native notifications
    println!("{}: {}", title, message);

    // On desktop platforms, we could use a notification library like notify-rust
    // For now, we just print to console
}

/// Run the tray event loop. Call this from the main thread.
#[cfg(target_os = "linux")]
pub fn run_tray_loop(tray: &GitzTray) {
    use gtk::glib;
    use gtk::prelude::*;

    if gtk::init().is_err() {
        eprintln!("Failed to initialize GTK");
        return;
    }

    let running = Arc::clone(&tray.running);

    glib::idle_add_local(move || {
        if running.load(Ordering::SeqCst) {
            glib::ControlFlow::Continue
        } else {
            gtk::main_quit();
            glib::ControlFlow::Break
        }
    });

    gtk::main();
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
pub fn run_tray_loop(tray: &GitzTray) {
    use winit::event_loop::{ControlFlow, EventLoop};

    let event_loop = match EventLoop::new() {
        Ok(el) => el,
        Err(e) => {
            eprintln!("Failed to create event loop: {e}");
            return;
        }
    };

    let running = Arc::clone(&tray.running);

    let _ = event_loop.run(move |_event, elwt| {
        elwt.set_control_flow(ControlFlow::Wait);
        if !running.load(Ordering::SeqCst) {
            elwt.exit();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_loads_successfully() {
        let icon = load_icon();
        assert!(icon.is_ok());
    }

    #[test]
    fn menu_creates_successfully() {
        let menu = create_menu();
        assert!(menu.is_ok());
    }
}
