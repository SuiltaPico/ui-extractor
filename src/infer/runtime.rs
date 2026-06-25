use crate::infer::error::Result;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct RuntimeConfig {
    #[serde(default)]
    pub onnx: Option<OnnxConfig>,
    #[serde(default)]
    pub mnn: Option<MnnConfig>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MnnConfig {
    #[serde(default = "default_mnn_backend")]
    pub backend: String,
    #[serde(default)]
    pub num_thread: Option<u32>,
    #[serde(default = "default_mnn_precision")]
    pub precision: String,
}

fn default_mnn_backend() -> String {
    "cpu".into()
}

fn default_mnn_precision() -> String {
    "normal".into()
}

impl Default for MnnConfig {
    fn default() -> Self {
        Self {
            backend: default_mnn_backend(),
            num_thread: None,
            precision: default_mnn_precision(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OnnxConfig {
    #[serde(default)]
    pub execution_providers: Vec<String>,
    #[serde(default)]
    pub intra_threads: Option<u32>,
    #[serde(default)]
    pub inter_threads: Option<u32>,
    #[serde(default = "default_true")]
    pub append_cpu_fallback: bool,
    #[serde(default = "default_true")]
    pub gpu_single_session: bool,
}

fn default_true() -> bool {
    true
}

impl Default for OnnxConfig {
    fn default() -> Self {
        Self {
            execution_providers: vec!["auto".into()],
            intra_threads: None,
            inter_threads: None,
            append_cpu_fallback: true,
            gpu_single_session: true,
        }
    }
}

impl RuntimeConfig {
    pub fn from_json(text: &str) -> Result<Self> {
        Ok(serde_json::from_str(text)?)
    }

    pub fn from_env_or_default() -> Self {
        if let Ok(text) = std::env::var("LOCAL_INFER_RUNTIME_CONFIG") {
            if let Ok(cfg) = Self::from_json(&text) {
                return cfg;
            }
        }
        Self::default()
    }

    pub fn onnx_config(&self) -> OnnxConfig {
        self.onnx.clone().unwrap_or_default()
    }

    pub fn mnn_config(&self) -> MnnConfig {
        self.mnn.clone().unwrap_or_default()
    }

    pub fn resolved_eps(&self) -> Vec<String> {
        let onnx = self.onnx_config();
        if !onnx.execution_providers.is_empty()
            && !onnx.execution_providers.iter().any(|ep| ep == "auto")
        {
            return maybe_append_cpu(onnx.execution_providers, onnx.append_cpu_fallback);
        }

        if let Ok(raw) = std::env::var("LOCAL_INFER_ORT_EP") {
            let eps: Vec<String> = raw
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if !eps.is_empty() {
                return maybe_append_cpu(eps, onnx.append_cpu_fallback);
            }
        }

        maybe_append_cpu(vec![], onnx.append_cpu_fallback)
    }

    pub fn prefer_gpu_single_session(&self) -> bool {
        self.onnx_config().gpu_single_session && resolved_eps_has_gpu(&self.resolved_eps())
    }
}

fn maybe_append_cpu(mut eps: Vec<String>, append: bool) -> Vec<String> {
    if append && !eps.iter().any(|ep| ep == "cpu") {
        eps.push("cpu".into());
    }
    if eps.is_empty() {
        eps.push("cpu".into());
    }
    eps
}

fn resolved_eps_has_gpu(eps: &[String]) -> bool {
    eps.iter()
        .any(|ep| matches!(ep.as_str(), "directml" | "coreml" | "cuda"))
}
