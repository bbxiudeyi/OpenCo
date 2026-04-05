use std::fs;

pub fn tool_write_file(path: &str, content: &str) -> String {
    match fs::write(path, content) {
        Ok(()) => "OK".to_string(),
        Err(e) => format!("Failed to write file: {}", e),
    }
}
