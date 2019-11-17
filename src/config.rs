use serde_derive::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct SubsystemMapperConfig {
    pub(crate) suffix: String,
    pub(crate) auth: Option<AuthConfig>,
    pub(crate) targets: Vec<Target>,
}

#[derive(Debug, Deserialize)]
pub struct AuthConfig {
    // HTTPS
    pub(crate) username: Option<String>,
    pub(crate) password: Option<String>,

    // SSH
    pub(crate) public_key: Option<String>,
    pub(crate) private_key: Option<String>,
    pub(crate) passphrase: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Target {
    pub(crate) url: String,
    pub(crate) branch: String,
}

pub fn read_config_in_workdir() -> SubsystemMapperConfig {
    let config: String = fs::read_to_string("SubsystemMapper.toml")
        .expect("Something went wrong reading the config file");

    toml::from_str(config.as_str()).expect("Something went wrong parsing the config file")
}
