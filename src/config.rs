use crate::error::CustomError;
use serde_derive::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct SubsystemMapperConfig {
    pub(crate) suffix: String,
    pub(crate) targets: Vec<Target>,
}

#[derive(Debug, Deserialize)]
pub struct Target {
    pub(crate) url: Option<String>,
    pub(crate) branch: Option<String>,
    pub(crate) folder: Option<String>,
}

pub fn read_config_in_workdir(path: &str) -> Result<SubsystemMapperConfig, CustomError> {
    let config: String = fs::read_to_string(path).map_err(|err| {
        CustomError::new(format!("While reading config file `{}`: {}", path, err))
    })?;

    let toml = toml::from_str(config.as_str()).map_err(|err| {
        CustomError::new(format!(
            "While parsing config file `{}` as TOML: {}",
            path, err
        ))
    })?;

    Ok(toml)
}
