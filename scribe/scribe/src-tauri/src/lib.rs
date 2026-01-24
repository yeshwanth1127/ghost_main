// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod activate;
mod api;
mod shortcuts;
mod window;
mod db;
mod capture;
mod assistant;
use tauri_plugin_posthog::{init as posthog_init, PostHogConfig, PostHogOptions};
use tauri::Manager;
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;
mod speaker;
use speaker::VadConfig;
use capture::CaptureState;

#[cfg(target_os = "macos")]
#[allow(deprecated)]
use tauri_nspanel::{
    cocoa::appkit::NSWindowCollectionBehavior, panel_delegate, WebviewWindowExt,
  };

  #[derive(Default)]
pub struct AudioState {
    stream_task: Arc<Mutex<Option<JoinHandle<()>>>>,
    vad_config: Arc<Mutex<VadConfig>>,
    is_capturing: Arc<Mutex<bool>>,
}

#[tauri::command]
fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Load .env for APP_ENDPOINT/API_ACCESS_KEY if present
    // Try to find .env file relative to the executable or current directory
    use std::path::PathBuf;
    
    // Get the directory where the executable is located
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()));
    
    // Try multiple possible locations
    let mut env_paths: Vec<PathBuf> = vec![
        PathBuf::from(".env"),                    // Current directory
        PathBuf::from("src-tauri/.env"),           // If running from project root
        PathBuf::from("../src-tauri/.env"),        // If running from build directory
    ];
    
    // Add executable-relative paths
    if let Some(exe_dir) = &exe_dir {
        env_paths.push(exe_dir.join(".env"));
        env_paths.push(exe_dir.join("..").join("src-tauri").join(".env"));
        env_paths.push(exe_dir.join("..").join(".env"));
    }
    
    // Try each path
    let mut env_loaded = false;
    for path in &env_paths {
        if path.exists() {
            if let Ok(env_path) = path.canonicalize() {
                if let Ok(_) = dotenv::from_path(&env_path) {
                    eprintln!("✅ Loaded .env from: {}", env_path.display());
                    env_loaded = true;
                    break;
                }
            }
        }
    }
    
    // Also try to load from src-tauri directory explicitly (for npm run tauri dev)
    if !env_loaded {
        if let Ok(current_dir) = std::env::current_dir() {
            // Try current directory/src-tauri/.env (when running from scribe/)
            let explicit_path = current_dir.join("src-tauri").join(".env");
            if explicit_path.exists() {
                if let Ok(_) = dotenv::from_path(&explicit_path) {
                    eprintln!("✅ Loaded .env from: {}", explicit_path.display());
                    env_loaded = true;
                }
            }
            
            // Also try current_dir/.env directly (when running from src-tauri/)
            if !env_loaded {
                let direct_path = current_dir.join(".env");
                if direct_path.exists() {
                    if let Ok(_) = dotenv::from_path(&direct_path) {
                        eprintln!("✅ Loaded .env from: {}", direct_path.display());
                        env_loaded = true;
                    }
                }
            }
        }
    }
    
    // Fallback to default dotenv behavior
    if !env_loaded {
        let _ = dotenv::dotenv();
    }
    
    // Log ALL environment variables starting with PAYMENT, APP, or API for debugging
    eprintln!("🔍 Checking environment variables...");
    eprintln!("   Current working directory: {:?}", std::env::current_dir().unwrap_or_default());
    
    // Check all relevant env vars
    let env_vars = vec!["PAYMENT_ENDPOINT", "APP_ENDPOINT", "API_ACCESS_KEY"];
    for var_name in env_vars {
        match std::env::var(var_name) {
            Ok(val) => {
                if var_name.contains("KEY") {
                    eprintln!("✅ {} found (length: {})", var_name, val.len());
                } else {
                    eprintln!("✅ {} found: {}", var_name, val);
                }
            }
            Err(_) => {
                // Check compile-time constant
                match std::env::var(format!("CARGO_{}", var_name)) {
                    Ok(_) => eprintln!("⚠️ {} not in runtime env, but found in compile-time", var_name),
                    Err(_) => {
                        // Try option_env! macro values (compile-time)
                        match var_name {
                            "PAYMENT_ENDPOINT" => {
                                #[cfg(feature = "debug")]
                                eprintln!("⚠️ {} not found (runtime or compile-time)", var_name);
                            }
                            _ => eprintln!("⚠️ {} not found", var_name),
                        }
                    }
                }
            }
        }
    }
    
    // Also dump all env vars starting with PAYMENT/APP/API for debugging
    eprintln!("📋 All env vars matching pattern:");
    for (key, value) in std::env::vars() {
        if key.contains("PAYMENT") || key.contains("APP_ENDPOINT") || key.contains("API_ACCESS") {
            if key.contains("KEY") {
                eprintln!("   {} = {} (length: {})", key, &value[..value.len().min(4)], value.len());
            } else {
                eprintln!("   {} = {}", key, value);
            }
        }
    }
    // Get PostHog API key
    let posthog_api_key = option_env!("POSTHOG_API_KEY")
        .unwrap_or("")
        .to_string();
    let mut builder = tauri::Builder::default()
        .plugin(
            tauri_plugin_sql::Builder::default()
                .add_migrations("sqlite:ghost.db", db::migrations())
                .build(),
        )
        .manage(AudioState::default())
        .manage(CaptureState::default())
        .manage(shortcuts::WindowVisibility {
            is_hidden: Mutex::new(false),
        })
        .manage(shortcuts::RegisteredShortcuts::default())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_keychain::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_machine_uid::init());
        
        if !posthog_api_key.is_empty() {
            builder = builder.plugin(posthog_init(PostHogConfig {
                api_key: posthog_api_key.clone(),
                options: Some(PostHogOptions {
                    disable_session_recording: Some(true),
                    capture_pageview: Some(false),
                    capture_pageleave: Some(false),
                    ..Default::default()
                }),
                ..Default::default()
            }));
        }
        #[cfg(target_os = "macos")]
        {
            builder = builder.plugin(tauri_nspanel::init());
        }
        let builder = builder.invoke_handler(tauri::generate_handler![
            get_app_version,
            window::set_window_height,
            capture::capture_to_base64,
            capture::start_screen_capture,
            capture::capture_selected_area,
            capture::close_overlay_window,
            shortcuts::check_shortcuts_registered,
            shortcuts::get_registered_shortcuts,
            shortcuts::update_shortcuts,
            shortcuts::validate_shortcut_key,
            shortcuts::set_app_icon_visibility,
            shortcuts::set_always_on_top,
            shortcuts::exit_app,
            activate::activate_license_api,
            activate::deactivate_license_api,
            activate::validate_license_api,
            activate::mask_license_key_cmd,
            activate::get_checkout_url,
            activate::create_trial_license,
            activate::is_first_launch,
            activate::reset_trial,
            activate::secure_storage_save,
            activate::secure_storage_get,
            activate::secure_storage_remove,
            api::transcribe_audio,
            api::chat_stream,
            api::fetch_models,
            api::create_system_prompt,
            api::submit_leave_application,
            api::check_license_status,
            speaker::start_system_audio_capture,
            speaker::stop_system_audio_capture,
            speaker::manual_stop_continuous,
            speaker::check_system_audio_access,
            speaker::request_system_audio_access,
            speaker::get_vad_config,
            speaker::update_vad_config,
            speaker::get_capture_status,
            speaker::get_audio_sample_rate,
            assistant::commands::parse_intent,
            assistant::commands::plan_with_llm,
            assistant::commands::verify_action_plan,
            assistant::commands::preview_action_plan,
            assistant::commands::execute_action_plan,
            assistant::commands::undo_action,
            assistant::commands::get_audit_history,
            assistant::commands::mint_capability_token,
        ])
        .setup(|app| {
            // Setup main window positioning
            window::setup_main_window(app).expect("Failed to setup main window");
            #[cfg(target_os = "macos")]
            init(app.app_handle());

            #[cfg(desktop)]
            {
                use tauri_plugin_autostart::MacosLauncher;

                #[allow(deprecated, unexpected_cfgs)]
                if let Err(e) = app.handle().plugin(tauri_plugin_autostart::init(
                    MacosLauncher::LaunchAgent,
                    Some(vec![]),
                )) {
                    eprintln!("Failed to initialize autostart plugin: {}", e);
                }
            }

            // Initialize global shortcut plugin with centralized handler
            app.handle().plugin(
                tauri_plugin_global_shortcut::Builder::new()
                    .with_handler(move |app, shortcut, event| {
                        use tauri_plugin_global_shortcut::{Shortcut, ShortcutState};
                        
                        if event.state() == ShortcutState::Pressed {
                            // Get registered shortcuts and find matching action
                            let state = app.state::<shortcuts::RegisteredShortcuts>();
                            let registered = match state.shortcuts.lock() {
                                Ok(guard) => guard,
                                Err(poisoned) => {
                                    eprintln!("Mutex poisoned in handler, recovering...");
                                    poisoned.into_inner()
                                }
                            };
                            
                            // Find which action this shortcut maps to
                            for (action_id, shortcut_str) in registered.iter() {
                                if let Ok(s) = shortcut_str.parse::<Shortcut>() {
                                    if &s == shortcut {
                                        eprintln!("Shortcut triggered: {} ({})", action_id, shortcut_str);
                                        shortcuts::handle_shortcut_action(&app, action_id);
                                        break;
                                    }
                                }
                            }
                        }
                    })
                    .build(),
            ).expect("Failed to initialize global shortcut plugin");
            if let Err(e) = shortcuts::setup_global_shortcuts(app.handle()) {
                eprintln!("Failed to setup global shortcuts: {}", e);
            }
           Ok(())
        });

    // Add macOS-specific permissions plugin
    #[cfg(target_os = "macos")]
    {
        builder = builder.plugin(tauri_plugin_macos_permissions::init());
    }

    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(target_os = "macos")]
#[allow(deprecated, unexpected_cfgs)]
fn init(app_handle: &AppHandle) {
    let window: WebviewWindow = app_handle.get_webview_window("main").unwrap();
  
    let panel = window.to_panel().unwrap();
  
    let delegate = panel_delegate!(MyPanelDelegate {
      window_did_become_key,
      window_did_resign_key
    });
  
    let handle = app_handle.to_owned();
  
    delegate.set_listener(Box::new(move |delegate_name: String| {
      match delegate_name.as_str() {
        "window_did_become_key" => {
          let app_name = handle.package_info().name.to_owned();
  
          println!("[info]: {:?} panel becomes key window!", app_name);
        }
        "window_did_resign_key" => {
          println!("[info]: panel resigned from key window!");
        }
        _ => (),
      }
    }));
  
    // Set the window to float level
    #[allow(non_upper_case_globals)]
    const NSFloatWindowLevel: i32 = 4;
    panel.set_level(NSFloatWindowLevel);
  
    #[allow(non_upper_case_globals)]
    const NSWindowStyleMaskNonActivatingPanel: i32 = 1 << 7;
    panel.set_style_mask(NSWindowStyleMaskNonActivatingPanel);
  
    #[allow(deprecated)]
    panel.set_collection_behaviour(
      NSWindowCollectionBehavior::NSWindowCollectionBehaviorFullScreenAuxiliary
        | NSWindowCollectionBehavior::NSWindowCollectionBehaviorCanJoinAllSpaces,
    );
  
    panel.set_delegate(delegate);
  }
