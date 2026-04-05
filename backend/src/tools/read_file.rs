use std::fs;

pub fn tool_read_file(path: &str) -> String {
    match fs::metadata(path) {
        Ok(meta) => {
            if meta.len() > 100 * 1024 {
                return "File too large (max 100KB)".to_string();
            }
        }
        Err(e) => return format!("Cannot access file: {}", e),
    }

    match fs::read_to_string(path) {
        Ok(content) => content,
        Err(e) => format!("Failed to read file: {}", e),
    }
}
