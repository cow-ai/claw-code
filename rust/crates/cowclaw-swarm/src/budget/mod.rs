pub mod linter;

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Tier { Xl, Large, Default, Tight }

impl Tier {
    #[must_use]
    pub fn max_lines(self) -> usize {
        match self {
            Tier::Xl => 1600, Tier::Large => 1000,
            Tier::Default => 500, Tier::Tight => 200,
        }
    }
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "xl" => Some(Self::Xl), "large" => Some(Self::Large),
            "default" => Some(Self::Default), "tight" => Some(Self::Tight),
            _ => None,
        }
    }
}
