//! System configuration.

use std::fs;
use std::path::Path;

use reportify::ResultExt;

use crate::config::system::SystemConfig;

use super::SystemResult;

/// Path of the system configuration file.
pub const SYSTEM_CONFIG_PATH: &str = "/etc/rugix/system.toml";

/// Load the system configuration.
pub fn load_system_config() -> SystemResult<SystemConfig> {
    Ok(if Path::new(SYSTEM_CONFIG_PATH).exists() {
        toml::from_str(
            &fs::read_to_string(SYSTEM_CONFIG_PATH)
                .whatever("unable to read system configuration file")?,
        )
        .whatever("unable to parse system configuration file")?
    } else {
        SystemConfig::default()
    })
}

#[cfg(test)]
mod tests {
    use indoc::indoc;

    use super::SystemConfig;

    #[test]
    fn test_from_toml() {
        toml::from_str::<SystemConfig>(indoc! {r#"
            [config-partition]
            disabled = false
            device = "/dev/sda1"

            [data-partition]
            disabled = false
            partition = 7

            [boot-flow]
            type = "u-boot"

            [slots.boot-a]
            type = "block"
            partition = 2

            [slots.boot-b]
            type = "block"
            device = "/dev/sda3"

            [slots.system-a]
            type = "block"
            device = "/dev/sda4"

            [slots.system-b]
            type = "block"
            device = "/dev/sda5"

            [slots.app-config]
            type = "block"
            device = "/dev/sda6"
            protected = true

            [boot-groups.a]
            slots = { boot = "boot-a", system = "system-a" }

            [boot-groups.b]
            slots = { boot = "boot-b", system = "system-b" }
        "#})
        .unwrap();
    }
}
