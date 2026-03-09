fn main() {
    // Rebuild when icons change (so taskbar icon updates)
    println!("cargo:rerun-if-changed=icons/icon.ico");
    println!("cargo:rerun-if-changed=icons/32x32.png");

    // Try to load .env from multiple locations during build
    // build.rs runs from src-tauri/ directory
    let env_paths = vec![".env", "../.env"];
    
    let mut loaded = false;
    for path in &env_paths {
        if std::path::Path::new(path).exists() {
            if dotenv::from_filename(path).is_ok() {
                println!("cargo:warning=Loaded .env from {}", path);
                loaded = true;
                break;
            }
        }
    }
    
    // Fallback to default behavior
    if !loaded {
        dotenv::dotenv().ok();
    }

    // Embed environment variables as compile-time constants
    if let Ok(payment_endpoint) = std::env::var("PAYMENT_ENDPOINT") {
        println!("cargo:rustc-env=PAYMENT_ENDPOINT={}", payment_endpoint);
    }

    if let Ok(api_access_key) = std::env::var("API_ACCESS_KEY") {
        println!("cargo:rustc-env=API_ACCESS_KEY={}", api_access_key);
    }

    // Default to local backend for dev; override with APP_ENDPOINT in .env
    let app_endpoint = std::env::var("APP_ENDPOINT")
        .unwrap_or_else(|_| "http://127.0.0.1:8083".to_string());
    println!("cargo:rustc-env=APP_ENDPOINT={}", app_endpoint);

    if let Ok(posthog_api_key) = std::env::var("POSTHOG_API_KEY") {
        println!("cargo:rustc-env=POSTHOG_API_KEY={}", posthog_api_key);
    }

    tauri_build::build()
}
