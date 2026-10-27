use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    High,
    Medium,
    Low,
    Info,
}

impl Confidence {
    pub fn as_str(self) -> &'static str {
        match self {
            Confidence::High => "high",
            Confidence::Medium => "medium",
            Confidence::Low => "low",
            Confidence::Info => "info",
        }
    }

    pub fn rank(self) -> u8 {
        match self {
            Confidence::High => 3,
            Confidence::Medium => 2,
            Confidence::Low => 1,
            Confidence::Info => 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub engine: String,
    pub subject: String,
    pub title: String,
    pub detail: String,
    pub url: Option<String>,
    pub confidence: Confidence,
    pub reason: String,
}

impl Finding {
    pub fn passes_strict(&self) -> bool {
        matches!(self.confidence, Confidence::High)
    }
}
