//! Test case representation.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestWorkflow {
    pub systems: Vec<TestSystemConfig>,
    pub steps: Vec<TestStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct TestSystemConfig {
    pub disk_image: String,
    pub disk_size: Option<String>,
    pub ssh: SshConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SshConfig {
    pub private_key: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "action")]
pub enum TestStep {
    #[serde(rename_all = "kebab-case")]
    Run {
        script: String,
        stdin: Option<PathBuf>,
        may_fail: Option<bool>,
        #[serde(default)]
        description: String,
    },
    #[serde(rename_all = "kebab-case")]
    Wait {
        #[serde(default)]
        description: String,
        duration: f64,
    },
}
