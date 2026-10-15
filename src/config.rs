// SPDX-License-Identifier: GPL-2.0-or-later
//
// Copyright (c) 2024 IFM Ecomatic GmbH
// Copyright (c) 2026 Stefano Babic <stefano.babic@swupdate.org>
//
//  - Description: Contains utility functions for configuration file

use crate::system::{self, EnvSettings};
use std::fmt;
use std::path::Path;
use std::sync::OnceLock;

/* This is the configuration file that is loaded at start of recovery */
pub const RECOVERY_CONFIG_FILE_PATH: &str = "/etc/recovery_gui/config.txt";

/* Maximum number of network interfaces taken from the configuration */
pub const IFACE_COUNT_MAX: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub version: String,
    pub app_id: String,
    pub logo: String,
    pub interfaces: String,
    pub mediapath: String,
    pub keypad_dev: String,
    pub theme_color: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            version: "1.0".into(),
            app_id: "Recovery App".into(),
            logo: String::new(),
            interfaces: String::new(),
            mediapath: "/media/usb/sda".into(),
            keypad_dev: "/dev/input/keyboard0".into(),
            theme_color: "orange".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidTag(pub String);

impl fmt::Display for InvalidTag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Invalid tag in config: {}", self.0)
    }
}

/// Split a configuration line into tag and value.
///
/// Double quotes are removed, all spaces are removed from the tag and the value is trimmed.
/// Empty lines and lines starting with '#' are ignored.
fn parse_line(line: &str) -> Option<(String, String)> {
    let line = line.replace('"', "");
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }

    let (tag, value) = trimmed.split_once('=').unwrap_or((trimmed, ""));
    let tag = tag.chars().filter(|c| !c.is_whitespace()).collect();

    Some((tag, value.trim().to_string()))
}

impl Config {
    /// Parse the configuration text; tags missing from the text keep their default value.
    pub fn parse(text: &str) -> Result<Config, InvalidTag> {
        let mut config = Config::default();

        for (tag, value) in text.lines().filter_map(parse_line) {
            let field = match tag.as_str() {
                "VERSION" => &mut config.version,
                "APP_ID" => &mut config.app_id,
                "LOGO" => &mut config.logo,
                "NETWORK_INTERFACES" => &mut config.interfaces,
                "MEDIAPATH" => &mut config.mediapath,
                "KEYPAD_DEVICE" => &mut config.keypad_dev,
                "THEME_COLOR" => &mut config.theme_color,
                _ => return Err(InvalidTag(tag)),
            };
            *field = value;
        }

        Ok(config)
    }

    /// Load the configuration file. The defaults are used when the file can't be read,
    /// an invalid tag terminates the program.
    pub fn load(path: &Path) -> Config {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(err) => {
                log::warn!("Failed to open configuration {}: {err}", path.display());
                return Config::default();
            }
        };

        Config::parse(&text).unwrap_or_else(|err| {
            log::error!("{err}\nPlease check {}", path.display());
            std::process::exit(-1);
        })
    }

    /// The configured network interfaces (comma separated in the configuration file).
    pub fn interface_names(&self) -> Vec<String> {
        parse_interfaces(&self.interfaces)
    }
}

/// Parse a comma separated list of interfaces. Whitespaces around the names are removed.
pub fn parse_interfaces(interfaces: &str) -> Vec<String> {
    interfaces
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .take(IFACE_COUNT_MAX)
        .map(String::from)
        .collect()
}

pub struct RecoveryParameters {
    pub config: Config,
    pub env: EnvSettings,
}

/// All parameters for the recovery program. On the first call the configuration file at the
/// default location and the environment are parsed.
pub fn recovery_parameters() -> &'static RecoveryParameters {
    static PARAMETERS: OnceLock<RecoveryParameters> = OnceLock::new();

    PARAMETERS.get_or_init(|| RecoveryParameters {
        config: Config::load(Path::new(RECOVERY_CONFIG_FILE_PATH)),
        env: system::env_settings(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_config_valid() {
        let text = "APP_ID = \"Recovery App\"\n\
                    NETWORK_INTERFACES = \"eth0, eth1\"\n\
                    VERSION = \"1.0\"\n\
                    MEDIAPATH =\"/run/media/sda\"\n\
                    KEYPAD_DEVICE =\"/dev/input/keypad0\"\n";

        let config = Config::parse(text).unwrap();

        assert_eq!(config.version, "1.0");
        assert_eq!(config.app_id, "Recovery App");
        assert_eq!(config.logo, "");
        assert_eq!(config.interfaces, "eth0, eth1");
        assert_eq!(config.mediapath, "/run/media/sda");
        assert_eq!(config.keypad_dev, "/dev/input/keypad0");
        assert_eq!(config.theme_color, "orange");
    }

    #[test]
    fn get_config_empty_values() {
        let text = "APP_ID =\nVERSION =\n\n\nMEDIAPATH =\n\n\n\n";

        let config = Config::parse(text).unwrap();

        assert_eq!(config.version, "");
        assert_eq!(config.app_id, "");
        assert_eq!(config.logo, "");
        assert_eq!(config.interfaces, "");
        assert_eq!(config.mediapath, "");
    }

    #[test]
    fn get_config_invalid_tag() {
        assert_eq!(
            Config::parse("FOO = bar\n"),
            Err(InvalidTag("FOO".into()))
        );
    }

    #[test]
    fn get_config_missing_file_uses_defaults() {
        let config = Config::load(Path::new("/nonexistent/recovery/config.txt"));
        assert_eq!(config, Config::default());
    }

    #[test]
    fn parse_line_handles_quotes_and_spaces() {
        assert_eq!(
            parse_line("  APP _ID = \"lorem ipsum \"\"dolor\" \n"),
            Some(("APP_ID".into(), "lorem ipsum dolor".into()))
        );
        assert_eq!(parse_line("VERSION"), Some(("VERSION".into(), "".into())));
        assert_eq!(parse_line("   "), None);
        assert_eq!(parse_line("# comment"), None);
    }

    #[test]
    fn parse_interfaces_list() {
        assert_eq!(parse_interfaces("eth0, br0, eth1"), ["eth0", "br0", "eth1"]);
        assert_eq!(parse_interfaces("abc,pqr,_xyz"), ["abc", "pqr", "_xyz"]);
        assert_eq!(parse_interfaces(""), Vec::<String>::new());

        let many = (0..20).map(|i| format!("eth{i}")).collect::<Vec<_>>().join(",");
        assert_eq!(parse_interfaces(&many).len(), IFACE_COUNT_MAX);
    }
}
