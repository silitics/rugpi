//! Test case representation.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    pub vm: VmConfig,
    pub steps: Vec<TestStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "action")]
pub enum TestStep {
    Reboot,
    Copy {
        src: String,
        dst: String,
    },
    Run {
        script: String,
        stdin: Option<PathBuf>,
        may_fail: Option<bool>,
    },
    Wait {
        duration_secs: f64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct VmConfig {
    pub image: PathBuf,
    pub stdout: Option<PathBuf>,
    pub stderr: Option<PathBuf>,
    pub private_key: PathBuf,
}
