use serde::{Serialize, Deserialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProfileId { P1, P2, P3, P4, P5, P6, P7, P8, P9 }

impl ProfileId {
    #[must_use]
    pub fn is_inline(&self) -> bool { matches!(self, ProfileId::P1 | ProfileId::P2) }

    /// Minimum of P6 escalation
    #[must_use]
    pub fn escalate_to_p6(self) -> ProfileId {
        match self {
            ProfileId::P1 | ProfileId::P2 | ProfileId::P3 |
            ProfileId::P4 | ProfileId::P5 => ProfileId::P6,
            other => other,
        }
    }
}

impl std::fmt::Display for ProfileId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub swarm: bool,
    #[serde(default)] pub gates: Vec<String>,
    #[serde(default = "default_one")] pub main_cap: f32,
    #[serde(default = "default_one")] pub worker_cap: f32,
    #[serde(default)] pub retro: bool,
    #[serde(default)] pub brainstorming_pre_phase: bool,
    #[serde(default)] pub oracle_cadence: Option<String>,
}

fn default_one() -> f32 { 1.0 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileTable {
    pub profiles: BTreeMap<ProfileId, Profile>,
}

impl ProfileTable {
    #[must_use]
    pub fn get(&self, id: ProfileId) -> Option<&Profile> {
        self.profiles.get(&id)
    }

    pub fn from_toml(toml_str: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_str)
    }
}
