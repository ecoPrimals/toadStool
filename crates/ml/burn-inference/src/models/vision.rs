//! Vision models (YOLO, ResNet, etc.)
//!
//! Type-safe API surface for computer vision inference.
//! Inference methods return `NotImplemented` until a model backend
//! (burn, onnxruntime, or custom WGSL) is integrated.

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
    pub fn new(config: YoloConfig) -> Self {
        Self { config }
    }

    /// Load from pretrained model weights
    pub fn from_pretrained(model_id: &str) -> Result<Self> {
        Err(crate::Error::NotImplemented(format!(
            "YOLO model loading not yet integrated (requested: {model_id})"
        )))
    }

    /// Detect objects in image
    pub fn detect(&self, _image: &[u8], _width: usize, _height: usize) -> Result<Vec<Detection>> {
        Err(crate::Error::NotImplemented(
            "YOLO inference requires a model backend (burn/onnx/wgsl)".into(),
        ))
    }

    /// Get number of parameters
    pub fn num_parameters(&self) -> usize {
        match self.config.version {
            YoloVersion::V8Nano => 3_200_000,
            YoloVersion::V8Small => 11_200_000,
            YoloVersion::V8Medium => 25_900_000,
            YoloVersion::V8Large => 43_700_000,
        }
    }
}

/// ResNet configuration
#[derive(Debug, Clone, Copy)]
pub enum ResNetVariant {
    ResNet18,
    ResNet34,
    ResNet50,
    ResNet101,
    ResNet152,
}

/// ResNet model
#[derive(Debug)]
pub struct ResNet {
    variant: ResNetVariant,
}

impl ResNet {
    /// Create new ResNet model
    pub fn new(variant: ResNetVariant) -> Self {
        Self { variant }
    }

    /// Classify image
    pub fn classify(&self, _image: &[u8]) -> Result<Vec<(usize, f32)>> {
        Err(crate::Error::NotImplemented(
            "ResNet inference requires a model backend (burn/onnx/wgsl)".into(),
        ))
    }

    /// Get number of parameters
    pub fn num_parameters(&self) -> usize {
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
    fn test_yolo_from_pretrained_not_implemented() {
        let result = Yolo::from_pretrained("yolov8n");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("not yet integrated"));
    }

    #[test]
    fn test_yolo_detect_not_implemented() {
        let yolo = Yolo::new(YoloConfig::default());
        let result = yolo.detect(&[], 0, 0);
        assert!(result.is_err());
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
    fn test_resnet_classify_not_implemented() {
        let model = ResNet::new(ResNetVariant::ResNet101);
        let result = model.classify(&[]);
        assert!(result.is_err());
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
