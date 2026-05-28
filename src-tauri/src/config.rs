use crate::models::{AppConfig, LlmProvider};
use std::path::PathBuf;

const CONFIG_FILE: &str = "config.json";

pub fn default_config() -> AppConfig {
    AppConfig {
        provider: LlmProvider::OpenAI,
        api_key: String::new(),
        custom_endpoint: None,
        language: "pt-BR".to_string(),
    }
}

pub fn config_path() -> PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("pdf-injection-checker");
    std::fs::create_dir_all(&path).ok();
    path.push(CONFIG_FILE);
    path
}

pub fn load_config() -> AppConfig {
    let path = config_path();
    if path.exists() {
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_else(|_| default_config())
    } else {
        default_config()
    }
}

pub fn save_config(config: &AppConfig) -> Result<(), String> {
    let path = config_path();
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Serialization error: {}", e))?;
    std::fs::write(&path, json).map_err(|e| format!("Write error: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Mutex;
    use std::time::{SystemTime, UNIX_EPOCH};

    static TEST_MUTEX: Mutex<()> = Mutex::new(());

    struct TestEnvGuard {
        home: Option<String>,
        xdg_config_home: Option<String>,
        test_root: PathBuf,
    }

    impl Drop for TestEnvGuard {
        fn drop(&mut self) {
            match &self.home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }

            match &self.xdg_config_home {
                Some(value) => std::env::set_var("XDG_CONFIG_HOME", value),
                None => std::env::remove_var("XDG_CONFIG_HOME"),
            }

            let _ = fs::remove_dir_all(&self.test_root);
        }
    }

    fn setup_test_env() -> TestEnvGuard {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let test_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("config-tests")
            .join(unique.to_string());
        let home_dir = test_root.join("home");

        fs::create_dir_all(&home_dir).unwrap();

        let previous_home = std::env::var("HOME").ok();
        let previous_xdg_config_home = std::env::var("XDG_CONFIG_HOME").ok();

        std::env::set_var("HOME", &home_dir);
        std::env::remove_var("XDG_CONFIG_HOME");

        TestEnvGuard {
            home: previous_home,
            xdg_config_home: previous_xdg_config_home,
            test_root,
        }
    }

    #[test]
    fn test_default_config() {
        let config = default_config();

        assert_eq!(config.language, "pt-BR");
        assert!(config.api_key.is_empty());
    }

    #[test]
    fn test_save_and_load_config() {
        let _lock = TEST_MUTEX.lock().unwrap();
        let _guard = setup_test_env();

        let config = AppConfig {
            provider: LlmProvider::OpenAI,
            api_key: "secret".to_string(),
            custom_endpoint: None,
            language: "en".to_string(),
        };

        save_config(&config).unwrap();

        let loaded = load_config();
        assert_eq!(loaded.language, "en");
    }
}
