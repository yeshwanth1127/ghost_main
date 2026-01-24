use std::path::{Path, PathBuf};

/// Validate path to prevent traversal attacks but allow any directory on the system
/// Only prevents malicious path traversal (too many ..) and ensures absolute paths are valid
pub fn validate_path(path_str: &str, _user_home: &Path) -> Result<PathBuf, String> {
    let path = Path::new(path_str);

    // For absolute paths, validate they're canonical
    if path.is_absolute() {
        let canonical = match path.canonicalize() {
            Ok(p) => p,
            Err(_) => {
                // Path doesn't exist yet, but validate structure
                // Prevent excessive .. components
                let mut depth = 0;
                for component in path.components() {
                    match component {
                        std::path::Component::ParentDir => {
                            if depth == 0 {
                                return Err(format!("Invalid path: excessive parent directory references: {}", path_str));
                            }
                            depth -= 1;
                        }
                        std::path::Component::RootDir | std::path::Component::Prefix(_) => {
                            // Reset depth at root
                            depth = 0;
                        }
                        std::path::Component::Normal(_) => {
                            depth += 1;
                        }
                        _ => {}
                    }
                }
                path.to_path_buf()
            }
        };
        return Ok(canonical);
    }

    // For relative paths, resolve from current working directory
    let cwd = std::env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?;
    
    let resolved = cwd.join(path);
    
    // Normalize path (resolve .. components)
    let normalized = resolved.normalize_path();
    
    // Validate the normalized path structure
    match normalized.canonicalize() {
        Ok(canonical) => Ok(canonical),
        Err(_) => {
            // Path doesn't exist yet, validate structure
            let mut depth = 0;
            for component in normalized.components() {
                match component {
                    std::path::Component::ParentDir => {
                        if depth == 0 {
                            return Err(format!("Path traversal detected: {}", path_str));
                        }
                        depth -= 1;
                    }
                    std::path::Component::Normal(_) => {
                        depth += 1;
                    }
                    _ => {}
                }
            }
            Ok(normalized)
        }
    }
}

/// Normalize a path by resolving .. and .
trait NormalizePath {
    fn normalize_path(&self) -> PathBuf;
}

impl NormalizePath for PathBuf {
    fn normalize_path(&self) -> PathBuf {
        let mut components = Vec::new();
        for component in self.components() {
            match component {
                std::path::Component::CurDir => continue,
                std::path::Component::ParentDir => {
                    components.pop();
                }
                _ => components.push(component),
            }
        }
        components.iter().collect()
    }
}

