//! Strategy Lab types shared by the service (runner) and the GUI.

use serde::{Deserialize, Serialize};

use crate::model::EngineKind;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LabStrategy {
    pub name: String,
    pub args: String,
    /// "DPIMech standard set" or an online list's label (e.g. "Community list").
    pub source: String,
    /// Where an online strategy was downloaded from; empty for the standard set.
    #[serde(default)]
    pub origin: String,
    /// Preset made for the user's detected ISP.
    #[serde(default)]
    pub recommended: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabRequest {
    pub engine: EngineKind,
    pub strategies: Vec<LabStrategy>,
    pub domains: Vec<String>,
    /// Hosts actually requested during the test (a reachable subset of `domains`).
    #[serde(default)]
    pub probes: Vec<String>,
    /// Requests per domain; more repeats catch flaky strategies.
    pub repeats: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabResult {
    /// `None` is the baseline run without any bypass.
    pub strategy: Option<LabStrategy>,
    pub ok: u32,
    pub total: u32,
    /// Mean time to first response byte of successful requests.
    pub avg_ms: u32,
    pub failed_domains: Vec<String>,
    /// Set when the engine itself could not start with this strategy.
    #[serde(default)]
    pub error: Option<String>,
    /// Passed the extra rounds run for the best candidates (every site, every time); a single
    /// lucky round is not enough to call a strategy working.
    #[serde(default)]
    pub confirmed: bool,
}

impl LabResult {
    /// Higher is better: confirmed first, then success rate, then speed.
    pub fn score(&self) -> (bool, u32, i64) {
        let rate = (self.ok * 1000).checked_div(self.total).unwrap_or(0);
        (self.confirmed, rate, -(self.avg_ms as i64))
    }

    pub fn same_strategy(&self, other: &LabResult) -> bool {
        match (&self.strategy, &other.strategy) {
            (Some(a), Some(b)) => a.args == b.args && a.name == b.name,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IspInfo {
    pub provider: String,
    pub asn: Option<u32>,
    pub country: String,
    /// Name from the built-in ISP table when recognised (e.g. "Türk Telekom").
    pub known: Option<String>,
}
