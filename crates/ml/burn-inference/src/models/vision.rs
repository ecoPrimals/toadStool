//! Vision models (YOLO, `ResNet`, etc.)
//!
//! Type-safe API surface for computer vision inference.
//!
//! # Requirements
//!
//! Vision inference requires:
//! - **Model weights**: Load from HuggingFace/Ultralytics or local safetensors/ONNX
//! - **Burn backend**: Enable `burn` with wgpu or ndarray; or use ONNX runtime
//! - **Image** (optional): Enable `vision` feature for `image` crate preprocessing
//!
//! # Example
//!
//! ```ignore
//! let yolo = Yolo::from_pretrained("yolov8n")?;
//! let detections = yolo.detect(&image_bytes, width, height)?;
//! ```

use crate::Error::ModelBackendRequired;
use crate::Error::ModelNotLoaded;
use crate::Result;

/// YOLO configuration
#[derive(Debug, Clone)]
pub struct YoloConfig {
    pub version: YoloVersion,
    pub input_size: (usize, usize),
    pub num_classes: usize,
}

/// YOLO version variants
#[derive(Debug, Clone, Copy)]
pub enum YoloVersion {
    V8Nano,
    V8Small,
    V8Medium,
    V8Large,
}

impl Default for YoloConfig {
    fn default() -> Self {
        Self {
            version: YoloVersion::V8Nano,
            input_size: (640, 640),
            num_classes: 80, // COCO classes
        }
    }
}

/// Detected object bounding box
#[derive(Debug, Clone)]
pub struct Detection {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    pub confidence: f32,
    pub class_id: usize,
}

/// YOLO model
#[derive(Debug)]
pub struct Yolo {
    config: YoloConfig,
}

impl Yolo {
    /// Create new YOLO model
    #[must_use]
    pub const fn new(config: YoloConfig) -> Self {
        Self { config }
    }

    /// Load from pretrained model weights (HuggingFace/Ultralytics or local path).
    ///
    /// **Requires**: Model weights. Load with `YoloModel::from_safetensors(path)` or
    /// `YoloModel::from_onnx(path)` once burn/ONNX backend is integrated.
    #[cfg_attr(
        feature = "vision",
        doc = "The `vision` feature enables image preprocessing."
    )]
    pub fn from_pretrained(model_id: &str) -> Result<Self> {
        Err(ModelNotLoaded(format!(
            "YOLO model weights required. Requested: {model_id}. \
             Load with YoloModel::from_safetensors(path) or from_onnx(path) once backend is integrated."
        )))
    }

    /// Detect objects in image.
    ///
    /// **Requires**: Model weights loaded via `Yolo::from_pretrained` or
    /// `YoloModel::from_safetensors(path)`, plus burn backend (wgpu/ndarray).
    pub fn detect(&self, _image: &[u8], _width: usize, _height: usize) -> Result<Vec<Detection>> {
        Err(ModelBackendRequired(
            "YOLO inference requires model weights. Load with YoloModel::from_safetensors(path). \
             Ensure burn backend (wgpu/ndarray) is enabled in Cargo.toml."
                .into(),
        ))
    }

    /// Get number of parameters
    #[must_use]
    pub const fn num_parameters(&self) -> usize {
        match self.config.version {
            YoloVersion::V8Nano => 3_200_000,
            YoloVersion::V8Small => 11_200_000,
            YoloVersion::V8Medium => 25_900_000,
            YoloVersion::V8Large => 43_700_000,
        }
    }
}

/// `ResNet` configuration
#[derive(Debug, Clone, Copy)]
pub enum ResNetVariant {
    ResNet18,
    ResNet34,
    ResNet50,
    ResNet101,
    ResNet152,
}

/// `ResNet` model
#[derive(Debug)]
pub struct ResNet {
    variant: ResNetVariant,
}

impl ResNet {
    /// Create new `ResNet` model
    #[must_use]
    pub const fn new(variant: ResNetVariant) -> Self {
        Self { variant }
    }

    /// Classify image.
    ///
    /// **Requires**: Model weights loaded via `ResNetModel::from_safetensors(path)`, plus burn backend.
    pub fn classify(&self, _image: &[u8]) -> Result<Vec<(usize, f32)>> {
        Err(ModelBackendRequired(
            "ResNet inference requires model weights. Load with ResNetModel::from_safetensors(path). \
             Ensure burn backend (wgpu/ndarray) is enabled in Cargo.toml.".into(),
        ))
    }

    /// Get number of parameters
    #[must_use]
    pub const fn num_parameters(&self) -> usize {
        match self.variant {
            ResNetVariant::ResNet18 => 11_700_000,
            ResNetVariant::ResNet34 => 21_800_000,
            ResNetVariant::ResNet50 => 25_600_000,
            ResNetVariant::ResNet101 => 44_500_000,
            ResNetVariant::ResNet152 => 60_200_000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yolo_config_default() {
        let cfg = YoloConfig::default();
        assert_eq!(cfg.num_classes, 80);
        assert_eq!(cfg.input_size, (640, 640));
    }

    #[test]
    fn test_yolo_num_parameters() {
        assert_eq!(
            Yolo::new(YoloConfig {
                version: YoloVersion::V8Nano,
                ..YoloConfig::default()
            })
            .num_parameters(),
            3_200_000
        );
        assert_eq!(
            Yolo::new(YoloConfig {
                version: YoloVersion::V8Small,
                ..YoloConfig::default()
            })
            .num_parameters(),
            11_200_000
        );
        assert_eq!(
            Yolo::new(YoloConfig {
                version: YoloVersion::V8Medium,
                ..YoloConfig::default()
            })
            .num_parameters(),
            25_900_000
        );
        assert_eq!(
            Yolo::new(YoloConfig {
                version: YoloVersion::V8Large,
                ..YoloConfig::default()
            })
            .num_parameters(),
            43_700_000
        );
    }

    #[test]
    fn test_yolo_from_pretrained_requires_weights() {
        let result = Yolo::from_pretrained("yolov8n");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("model weights required"));
        assert!(err.contains("from_safetensors"));
    }

    #[test]
    fn test_yolo_detect_requires_backend() {
        let yolo = Yolo::new(YoloConfig::default());
        let result = yolo.detect(&[], 0, 0);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("model weights"));
        assert!(err.contains("from_safetensors"));
    }

    #[test]
    fn test_resnet_num_parameters() {
        assert_eq!(
            ResNet::new(ResNetVariant::ResNet18).num_parameters(),
            11_700_000
        );
        assert_eq!(
            ResNet::new(ResNetVariant::ResNet50).num_parameters(),
            25_600_000
        );
        assert_eq!(
            ResNet::new(ResNetVariant::ResNet152).num_parameters(),
            60_200_000
        );
    }

    #[test]
    fn test_resnet_classify_requires_backend() {
        let model = ResNet::new(ResNetVariant::ResNet101);
        let result = model.classify(&[]);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("model weights"));
        assert!(err.contains("from_safetensors"));
    }

    #[test]
    fn test_detection_fields() {
        let det = Detection {
            x1: 10.0,
            y1: 20.0,
            x2: 100.0,
            y2: 200.0,
            confidence: 0.95,
            class_id: 0,
        };
        assert!((det.confidence - 0.95).abs() < 1e-5);
        assert_eq!(det.class_id, 0);
    }
}
