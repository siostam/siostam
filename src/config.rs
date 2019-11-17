use serde_derive::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct SubsystemMapperConfig {
    pub(crate) global: GlobalConfig,
    pub(crate) remotes: Vec<GitRemote>,
}

#[derive(Debug, Deserialize)]
pub struct GlobalConfig {
    pub(crate) path_to_ssh_key: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GitRemote {
    pub(crate) url: String,
    pub(crate) name: String,
    pub(crate) branch: String,
}

pub fn read_config_in_workdir() -> SubsystemMapperConfig {
    let config: String = fs::read_to_string("SubsystemMapper.toml")
        .expect("Something went wrong reading the config file");

    toml::from_str(config.as_str()).expect("Something went wrong parsing the config file")
}
