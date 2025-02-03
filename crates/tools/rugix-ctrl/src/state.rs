use std::fs;

use reportify::ResultExt;
use tracing::warn;

use crate::config::state::StateConfig;
use crate::system::SystemResult;

/// The default directory with the configurations for state management.
pub const STATE_CONFIG_DIR: &str = "/etc/rugix/state";
pub const STATE_CONFIG_PATH: &str = "/etc/rugix/state.toml";

/// Loads the state configuration from the provided directory.
pub fn load_state_config() -> SystemResult<StateConfig> {
    let mut combined = StateConfig::new();

    if let Ok(state) = fs::read_to_string(STATE_CONFIG_PATH) {
        merge(
            &mut combined,
            toml::from_str(&state).whatever("unable to load state config")?,
        );
    }

    if let Ok(read_dir) = fs::read_dir(STATE_CONFIG_DIR) {
        for entry in read_dir {
            if let Some(config) = entry
                .ok()
                .and_then(|entry| fs::read_to_string(entry.path()).ok())
                .and_then(|config| toml::from_str(&config).ok())
            {
                merge(&mut combined, config);
            }
        }
    }
    Ok(combined)
}

fn merge(target: &mut StateConfig, other: StateConfig) {
    if target.overlay.is_none() {
        target.overlay = other.overlay;
    } else if other.overlay.is_some() {
        warn!("Conflicting overlay options. Will use {:?}", target.overlay);
    }
    if let Some(persist) = &mut target.persist {
        if let Some(other) = other.persist {
            persist.extend(other);
        }
    } else {
        target.persist = other.persist;
    }
}
