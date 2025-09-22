use std::path::PathBuf;

lazy_static::lazy_static! {
    pub static ref PROJECT_NAME: String = String::from("BrokerX").to_uppercase();
}

/// Get the data directory for the application
pub fn get_data_dir() -> PathBuf {
    let project_name = PROJECT_NAME.clone().to_lowercase();

    if let Ok(data_dir) = std::env::var("XDG_DATA_HOME") {
        PathBuf::from(data_dir).join(&project_name)
    } else if let Ok(home_dir) = std::env::var("HOME") {
        PathBuf::from(home_dir)
            .join(".local")
            .join("share")
            .join(&project_name)
    } else {
        // Fallback to current directory if no home directory is found
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(&project_name)
    }
}
