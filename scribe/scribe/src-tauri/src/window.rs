use tauri::{App, Manager, WebviewWindow, Size, LogicalSize};

// The offset from the top of the screen to the window
const TOP_OFFSET: i32 = 0;

/// Sets up the main window with custom positioning
pub fn setup_main_window(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    // Try different possible window labels
    let window = app
        .get_webview_window("main")
        .or_else(|| app.get_webview_window("Ghost"))
        .or_else(|| {
            // Get the first window if specific labels don't work
            app.webview_windows().values().next().cloned()
        })
        .ok_or("No window found")?;

    // Ensure an initial size that fits the full UI before React mounts
    let _ = window.set_size(Size::Logical(LogicalSize::new(1200.0, 800.0)));
    
    // Try to position window, but don't fail if it doesn't work
    let _ = position_window_top_center(&window, TOP_OFFSET);

    // Ensure window is visible and focused on startup
    let _ = window.show();
    let _ = window.set_focus();

    // Set window as non-focusable on Windows
    // #[cfg(target_os = "windows")]
    // {
    //     let _ = window.set_focusable(false);
    // }

    Ok(())
}

/// Positions a window at the top center of the screen with a specified Y offset
pub fn position_window_top_center(
    window: &WebviewWindow,
    y_offset: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    // Get the primary monitor - use a timeout-safe approach
    match window.primary_monitor() {
        Ok(Some(monitor)) => {
            if let Ok(window_size) = window.outer_size() {
                let monitor_size = monitor.size();
                
                // Calculate center X position
                let center_x = (monitor_size.width as i32 - window_size.width as i32) / 2;

                // Set the window position - ignore errors
                let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition {
                    x: center_x,
                    y: y_offset,
                }));
            }
        }
        _ => {
            // If monitor detection fails, just skip positioning
            eprintln!("Warning: Could not get primary monitor for window positioning");
        }
    }

    Ok(())
}

#[tauri::command]
pub fn center_main_window(window: tauri::WebviewWindow) -> Result<(), String> {
    position_window_top_center(&window, TOP_OFFSET)
        .map_err(|e| format!("Failed to center window: {}", e))
}

/// Future function for centering window completely (both X and Y)
#[allow(dead_code)]
pub fn center_window_completely(window: &WebviewWindow) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(monitor) = window.primary_monitor()? {
        let monitor_size = monitor.size();
        let window_size = window.outer_size()?;

        let center_x = (monitor_size.width as i32 - window_size.width as i32) / 2;
        let center_y = (monitor_size.height as i32 - window_size.height as i32) / 2;

        window.set_position(tauri::Position::Physical(tauri::PhysicalPosition {
            x: center_x,
            y: center_y,
        }))?;
    }

    Ok(())
}

#[tauri::command]
pub fn set_window_height(window: tauri::WebviewWindow, height: u32) -> Result<(), String> {
    use tauri::{LogicalSize, Size};

    // Simply set the window size with fixed width and new height
    let new_size = LogicalSize::new(1200.0, height as f64);
    window
        .set_size(Size::Logical(new_size))
        .map_err(|e| format!("Failed to resize window: {}", e))?;

    Ok(())
}

#[tauri::command]
pub fn set_window_size(window: tauri::WebviewWindow, width: u32, height: u32) -> Result<(), String> {
    use tauri::{LogicalSize, Size};

    let new_size = LogicalSize::new(width as f64, height as f64);
    window
        .set_size(Size::Logical(new_size))
        .map_err(|e| format!("Failed to resize window: {}", e))?;

    Ok(())
}

#[tauri::command]
pub fn set_window_position(
    window: tauri::WebviewWindow,
    x: i32,
    y: i32,
) -> Result<(), String> {
    use tauri::{PhysicalPosition, Position};

    window
        .set_position(Position::Physical(PhysicalPosition { x, y }))
        .map_err(|e| format!("Failed to set window position: {}", e))?;

    Ok(())
}

#[tauri::command]
pub fn get_window_position(window: tauri::WebviewWindow) -> Result<(i32, i32), String> {
    let pos = window
        .outer_position()
        .map_err(|e| format!("Failed to get window position: {}", e))?;
    Ok((pos.x, pos.y))
}

#[tauri::command]
pub fn get_bottom_right_position_for_size(
    window: tauri::WebviewWindow,
    width: u32,
    height: u32,
) -> Result<(i32, i32), String> {
    let monitor = window
        .primary_monitor()
        .map_err(|e| format!("Failed to get primary monitor: {}", e))?
        .ok_or("No primary monitor")?;
    let monitor_size = monitor.size();
    let monitor_pos = monitor.position();
    let x = monitor_pos.x + (monitor_size.width as i32) - (width as i32);
    let y = monitor_pos.y + (monitor_size.height as i32) - (height as i32);
    Ok((x, y))
}

/// Returns (x, y) for the collapsed logo. Uses work area (excludes taskbar)
/// so the logo stays visible above the taskbar. Large margin keeps it in view.
const LOGO_MARGIN_RIGHT: i32 = 600;
const LOGO_MARGIN_BOTTOM: i32 = 100;

#[tauri::command]
pub fn get_logo_position_clamped(
    window: tauri::WebviewWindow,
    width: u32,
    height: u32,
    _saved_x: Option<i32>,
    _saved_y: Option<i32>,
) -> Result<(i32, i32), String> {
    let monitor = window
        .primary_monitor()
        .map_err(|e| format!("Failed to get primary monitor: {}", e))?
        .ok_or("No primary monitor")?;

    let work = monitor.work_area();
    let wx = work.position.x;
    let wy = work.position.y;
    let ww = work.size.width as i32;
    let wh = work.size.height as i32;
    let w = width as i32;
    let h = height as i32;

    let max_x = wx + ww - w - LOGO_MARGIN_RIGHT;
    let max_y = wy + wh - h - LOGO_MARGIN_BOTTOM;
    let x = max_x.clamp(wx, wx + ww - w);
    let y = max_y.clamp(wy, wy + wh - h);

    Ok((x, y))
}
