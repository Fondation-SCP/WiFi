use crate::errors::ApiError;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;
use std::{fs, io};

lazy_static!(
    static ref CONFIG_PATHS: Box<[PathBuf]> = [
        Some(PathBuf::new().join("etc")),
        std::env::home_dir().map(|home| home.join(".config")),
        std::env::current_dir().ok()
    ].into_iter()
    .filter_map(|x| x.map(|dir| dir.join("wikidot_fi.toml")))
    .collect();
);

impl Config {
    #[inline(always)]
    const fn default_parallel_tasks() -> usize {16}
    #[inline(always)]
    const fn default_port() -> u16 {2012}
}


#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct Config {
    #[serde(default)]
    pub database: DatabaseData,
    #[serde(default)]
    pub write_tokens: Box<[u64]>,
    #[serde(default = "Config::default_parallel_tasks")]
    pub parallel_tasks: usize,
    #[serde(default = "Config::default_port")]
    pub port: u16,
}

impl DatabaseData {
    #[inline(always)]
    fn default_address() -> IpAddr {IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))}
    #[inline(always)]
    fn default_port() -> u16 {3306}
    #[inline(always)]
    fn default_username() -> String {whoami::username().unwrap_or_default()}
    #[inline(always)]
    fn default_name() -> String {"wifi_test".to_string()}
}



#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct DatabaseData {
    #[serde(default = "DatabaseData::default_address")]
    pub address: IpAddr,
    #[serde(default = "DatabaseData::default_port")]
    pub port: u16,
    #[serde(default = "DatabaseData::default_username")]
    pub username: String,
    #[serde(default)]
    pub passwd: String,
    #[serde(default = "DatabaseData::default_name")]
    pub name: String
}

impl DatabaseData {
    pub fn get_url(&self) -> String {
        format!("mariadb://{}:{}@{}:{}/{}", self.username, self.passwd, self.address, self.port, self.name)
    }
}

impl Default for DatabaseData {
    fn default() -> Self {
        Self {
            address: Self::default_address(),
            port: Self::default_port(),
            username: Self::default_username(),
            passwd: String::default(),
            name: Self::default_name()
        }
    }
}

enum CfgFile {
    Existing(String),
    New(File),
    None
}

impl CfgFile {

    fn is_some(&self) -> bool {
        !matches!(self, Self::None)
    }

    fn try_load() -> Result<Self, io::Error> {
        CONFIG_PATHS.iter()
            .try_fold(Self::None, |file, path| {
                if file.is_some() {
                    return Ok(file);
                }
                match fs::read_to_string(path) {
                    Ok(file) => Ok(Self::Existing(file)),
                    Err(e) if e.kind() == io::ErrorKind::NotFound => match File::create_new(path) {
                        Ok(file) => Ok(Self::New(file)),
                        Err(e) if e.kind() == io::ErrorKind::PermissionDenied || e.kind() == io::ErrorKind::NotFound => Ok(Self::None),
                        Err(e) => Err(e)
                    },
                    Err(e) => Err(e)
                }
            })
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            database: DatabaseData::default(),
            parallel_tasks: Self::default_parallel_tasks(),
            port: Self::default_port(),
            write_tokens: Box::default()
        }
    }
}

impl Config {
    pub(crate) fn try_new() -> Result<Self, Box<dyn Error>> {
        match CfgFile::try_load()? {
            CfgFile::New(mut file) => {
                let config = Self::default();
                let toml = toml::to_string_pretty(&config)?;
                file.write_all(toml.as_bytes())?;
                Ok(config)
            },
            CfgFile::Existing(str_data) => toml::from_str::<Self>(str_data.as_str()).map_err(Into::into),
            CfgFile::None => Err("Could not find a suitable directory to create the configuration file.".into())
        }
    }

    pub(crate) fn validate_token(&self, token: impl Into<u64>) -> Result<(), ApiError> {
        if self.write_tokens.contains(&token.into()) {
            Ok(())
        } else {
            Err(ApiError::AccessForbidden)
        }
    }

    pub(crate) fn get_bind_addr(&self) -> String {
        format!("0.0.0.0:{}", self.port)
    }
}