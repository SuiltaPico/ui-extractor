use ort::session::builder::SessionBuilder;
use oar_ocr::core::config::{OrtExecutionProvider, OrtSessionConfig};

use crate::error::{ExtractError, Result};

fn directml_available() -> bool {
    #[cfg(target_os = "windows")]
    {
        use ort::ep::{DirectML, ExecutionProvider};

        DirectML::default().is_available().unwrap_or(false)
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

fn log_cpu_fallback(component: &str) {
    eprintln!(
        "{component}: using CPU{}",
        if cfg!(target_os = "windows") {
            " (DirectML unavailable)"
        } else {
            ""
        }
    );
}

/// Apply the best available ONNX Runtime execution provider for direct `ort` sessions.
pub fn apply_session_builder(builder: SessionBuilder, component: &str) -> Result<SessionBuilder> {
    if directml_available() {
        #[cfg(target_os = "windows")]
        {
            use ort::ep::DirectML;

            eprintln!("{component}: using DirectML (GPU)");
            return builder
                .with_execution_providers([DirectML::default().build()])
                .map_err(|e| ExtractError::Image(e.to_string()));
        }
    }

    log_cpu_fallback(component);
    Ok(builder)
}

/// ONNX Runtime session config for `oar-ocr` pipelines (det/rec models).
pub fn oar_session_config(component: &str) -> Option<OrtSessionConfig> {
    if !directml_available() {
        log_cpu_fallback(component);
        return None;
    }

    eprintln!("{component}: using DirectML (GPU)");
    Some(
        OrtSessionConfig::new().with_execution_providers(vec![
            OrtExecutionProvider::DirectML { device_id: None },
            OrtExecutionProvider::CPU,
        ]),
    )
}

/// GPU execution providers should share one session; CPU can use one session per worker.
pub fn prefer_gpu_single_session() -> bool {
    directml_available()
}
