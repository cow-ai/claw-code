#![allow(clippy::module_inception)]

pub mod budget;
pub mod config;
pub mod events;
pub mod gates;
pub mod hivemind;
pub mod memory;
pub mod mcp_stdio;
pub mod oracle;
pub mod planner;
pub mod worker;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io: {0}")] Io(#[from] std::io::Error),
    #[error("sql: {0}")] Sql(#[from] rusqlite::Error),
    #[error("serde: {0}")] Serde(#[from] serde_json::Error),
    #[error("toml: {0}")] Toml(#[from] toml::de::Error),
    #[error("git: {0}")] Git(#[from] git2::Error),
    #[error("xml: {0}")] Xml(#[from] quick_xml::DeError),
    #[error("other: {0}")] Other(String),
}
