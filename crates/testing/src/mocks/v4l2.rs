// SPDX-License-Identifier: AGPL-3.0-only
//! Mock V4L2 capture device for headless CI testing.
//!
//! Simulates a V4L2 camera device without requiring actual hardware or
//! kernel interfaces. Generates synthetic frames with configurable
//! formats, resolutions, and error injection.
#![allow(clippy::expect_used)]

use std::sync::atomic::{AtomicU64, Ordering};

/// Capture pixel format compatible with V4L2 `CaptureFormat`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptureFormat {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// FourCC pixel format code.
    pub fourcc: u32,
    /// Bytes per line (stride).
    pub bytes_per_line: u32,
    /// Total image size in bytes.
    pub image_size: u32,
}

impl CaptureFormat {
    /// Bytes per pixel (approximate for common formats).
    #[must_use]
    pub fn bytes_per_pixel(&self) -> usize {
        if self.image_size > 0 && self.width > 0 && self.height > 0 {
            (self.image_size / (self.width * self.height)) as usize
        } else {
            // Common YUYV/VYUY: 2 bpp
            match self.fourcc {
                0x56_59_55_59 | 0x32_31_56_59 => 2, // VYUY, YUY2
                _ => 2,
            }
        }
    }
}

/// Pattern for synthetic frame generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FramePattern {
    /// Single solid color (0–255).
    Solid(u8),
    /// Horizontal gradient.
    Gradient,
    /// Frame number encoded in first 8 pixels.
    Counter,
    /// Deterministic pseudo-random noise.
    Random,
}

/// Configuration for the mock V4L2 device.
#[derive(Debug, Clone)]
pub struct MockV4l2Config {
    /// Device name (e.g. "/dev/video0").
    pub device_name: String,
    /// Supported formats.
    pub formats: Vec<CaptureFormat>,
    /// Default width.
    pub default_width: u32,
    /// Default height.
    pub default_height: u32,
    /// Frame rate (for metadata; not used in frame gen).
    pub frame_rate: u32,
    /// Pattern for synthetic frame generation.
    pub generate_pattern: FramePattern,
    /// If set, `open()` fails with this error (for testing).
    pub fail_open: Option<MockV4l2Error>,
}

impl Default for MockV4l2Config {
    fn default() -> Self {
        Self {
            device_name: "/dev/video0".to_string(),
            formats: vec![
                CaptureFormat {
                    width: 640,
                    height: 480,
                    fourcc: 0x56_59_55_59, // VYUY
                    bytes_per_line: 1280,
                    image_size: 614_400,
                },
                CaptureFormat {
                    width: 1920,
                    height: 1080,
                    fourcc: 0x56_59_55_59,
                    bytes_per_line: 3840,
                    image_size: 4_147_200,
                },
            ],
            default_width: 640,
            default_height: 480,
            frame_rate: 30,
            generate_pattern: FramePattern::Gradient,
            fail_open: None,
        }
    }
}

/// Injectable errors for testing error paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MockV4l2Error {
    /// Device is busy.
    DeviceBusy,
    /// Stream is off.
    StreamOff,
    /// Buffer error.
    BufferError,
    /// I/O timeout.
    IoTimeout,
}

/// Mock V4L2 capture device for headless CI testing.
pub struct MockV4l2Device {
    config: MockV4l2Config,
    format: CaptureFormat,
    streaming: bool,
    frame_count: AtomicU64,
    injected_error: std::sync::Mutex<Option<MockV4l2Error>>,
}

impl MockV4l2Device {
    /// Create a new mock device with the given configuration.
    #[must_use]
    pub fn new(config: MockV4l2Config) -> Self {
        let default_fmt = config
            .formats
            .iter()
            .find(|f| f.width == config.default_width && f.height == config.default_height)
            .copied()
            .unwrap_or_else(|| {
                config.formats.first().copied().unwrap_or(CaptureFormat {
                    width: config.default_width,
                    height: config.default_height,
                    fourcc: 0x56_59_55_59,
                    bytes_per_line: config.default_width * 2,
                    image_size: config.default_width * config.default_height * 2,
                })
            });

        Self {
            config: config.clone(),
            format: default_fmt,
            streaming: false,
            frame_count: AtomicU64::new(0),
            injected_error: std::sync::Mutex::new(None),
        }
    }

    /// Open a pre-configured mock device (convenience constructor).
    ///
    /// # Errors
    ///
    /// Returns error if `config.fail_open` is `Some(DeviceBusy)` or similar.
    pub fn open(config: MockV4l2Config) -> Result<Self, String> {
        if let Some(err) = config.fail_open {
            if err == MockV4l2Error::DeviceBusy {
                return Err("device busy".to_string());
            }
        }
        Ok(Self::new(config))
    }

    /// Start capture streaming.
    ///
    /// # Errors
    ///
    /// Returns error if already streaming or error injected.
    pub fn start_capture(&mut self) -> Result<(), String> {
        if let Some(err) = self
            .injected_error
            .lock()
            .map_err(|e| format!("lock poisoned: {e}"))?
            .as_ref()
        {
            return Err(format!("{err:?}"));
        }
        if self.streaming {
            return Err("already streaming".to_string());
        }
        self.streaming = true;
        Ok(())
    }

    /// Read a frame (generates synthetic data).
    ///
    /// # Errors
    ///
    /// Returns error if not streaming or error injected.
    pub fn read_frame(&self) -> Result<Vec<u8>, String> {
        if let Some(err) = self
            .injected_error
            .lock()
            .map_err(|e| format!("lock poisoned: {e}"))?
            .as_ref()
        {
            match err {
                MockV4l2Error::StreamOff => return Err("stream off".to_string()),
                MockV4l2Error::BufferError => return Err("buffer error".to_string()),
                MockV4l2Error::IoTimeout => return Err("I/O timeout".to_string()),
                MockV4l2Error::DeviceBusy => return Err("device busy".to_string()),
            }
        }
        if !self.streaming {
            return Err("not streaming".to_string());
        }
        let count = self.frame_count.fetch_add(1, Ordering::Relaxed);
        let frame = self.generate_frame(count);
        Ok(frame)
    }

    /// Stop capture streaming.
    ///
    /// # Errors
    ///
    /// Returns error if error injected.
    pub fn stop_capture(&mut self) -> Result<(), String> {
        if let Some(err) = self
            .injected_error
            .lock()
            .map_err(|e| format!("lock poisoned: {e}"))?
            .as_ref()
        {
            return Err(format!("{err:?}"));
        }
        self.streaming = false;
        Ok(())
    }

    /// Set capture format.
    ///
    /// # Errors
    ///
    /// Returns error if format not supported or error injected.
    pub fn set_format(&mut self, width: u32, height: u32, pixfmt: u32) -> Result<(), String> {
        if let Some(err) = self
            .injected_error
            .lock()
            .map_err(|e| format!("lock poisoned: {e}"))?
            .as_ref()
        {
            return Err(format!("{err:?}"));
        }
        let fmt = self
            .config
            .formats
            .iter()
            .find(|f| f.width == width && f.height == height && f.fourcc == pixfmt)
            .copied()
            .or_else(|| {
                self.config
                    .formats
                    .iter()
                    .find(|f| f.width == width && f.height == height)
                    .copied()
            })
            .ok_or_else(|| "unsupported format".to_string())?;

        self.format = fmt;
        Ok(())
    }

    /// Get current capture format.
    #[must_use]
    pub fn get_format(&self) -> CaptureFormat {
        self.format
    }

    /// Enumerate supported formats.
    #[must_use]
    pub fn enumerate_formats(&self) -> Vec<CaptureFormat> {
        self.config.formats.clone()
    }

    /// Inject an error for the next operation.
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    pub fn inject_error(&self, error: MockV4l2Error) {
        *self.injected_error.lock().expect("mock mutex poisoned") = Some(error);
    }

    /// Clear injected error.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    pub fn clear_error(&self) {
        *self.injected_error.lock().expect("mock mutex poisoned") = None;
    }

    /// Current frame count (for Counter pattern verification).
    #[must_use]
    pub fn frame_count(&self) -> u64 {
        self.frame_count.load(Ordering::Relaxed)
    }

    fn generate_frame(&self, frame_num: u64) -> Vec<u8> {
        let bpp = self.format.bytes_per_pixel();
        let size = self.format.width as usize * self.format.height as usize * bpp;

        match self.config.generate_pattern {
            FramePattern::Solid(val) => vec![val; size],
            FramePattern::Gradient => {
                let mut buf = Vec::with_capacity(size);
                for _y in 0..self.format.height {
                    for x in 0..self.format.width {
                        let byte_val = (x * 255 / self.format.width.max(1)) as u8;
                        for _ in 0..bpp {
                            buf.push(byte_val);
                        }
                    }
                }
                buf
            }
            FramePattern::Counter => {
                let mut buf = vec![0u8; size];
                for (i, byte) in buf.iter_mut().enumerate().take(8.min(size)) {
                    *byte = ((frame_num >> (i * 8)) & 0xFF) as u8;
                }
                buf
            }
            FramePattern::Random => {
                let mut buf = vec![0u8; size];
                let mut state = (frame_num as u32).wrapping_add(1) | 1;
                for byte in &mut buf {
                    state ^= state << 13;
                    state ^= state >> 17;
                    state ^= state << 5;
                    *byte = state as u8;
                }
                buf
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_pattern_solid() {
        let config = MockV4l2Config {
            generate_pattern: FramePattern::Solid(0x42),
            ..MockV4l2Config::default()
        };
        let mut dev = MockV4l2Device::new(config);
        dev.start_capture().expect("start");
        let frame = dev.read_frame().expect("read");
        assert!(!frame.is_empty());
        assert!(frame.iter().all(|&b| b == 0x42));
    }

    #[test]
    fn frame_pattern_gradient() {
        let config = MockV4l2Config {
            default_width: 64,
            default_height: 64,
            generate_pattern: FramePattern::Gradient,
            formats: vec![CaptureFormat {
                width: 64,
                height: 64,
                fourcc: 0x56_59_55_59,
                bytes_per_line: 128,
                image_size: 64 * 64 * 2,
            }],
            ..MockV4l2Config::default()
        };
        let mut dev = MockV4l2Device::new(config);
        dev.start_capture().expect("start");
        let frame = dev.read_frame().expect("read");
        assert_eq!(frame.len(), 64 * 64 * 2);
        assert_eq!(frame[0], 0);
        assert!(frame[frame.len() - 1] > 0);
    }

    #[test]
    fn frame_pattern_counter() {
        let config = MockV4l2Config {
            default_width: 8,
            default_height: 8,
            generate_pattern: FramePattern::Counter,
            formats: vec![CaptureFormat {
                width: 8,
                height: 8,
                fourcc: 0x56_59_55_59,
                bytes_per_line: 16,
                image_size: 8 * 8 * 2,
            }],
            ..MockV4l2Config::default()
        };
        let mut dev = MockV4l2Device::new(config);
        dev.start_capture().expect("start");
        let f0 = dev.read_frame().expect("read");
        let f1 = dev.read_frame().expect("read");
        assert_eq!(f0[0], 0);
        assert_eq!(f1[0], 1);
    }

    #[test]
    fn frame_pattern_random() {
        let config = MockV4l2Config {
            default_width: 32,
            default_height: 32,
            generate_pattern: FramePattern::Random,
            formats: vec![CaptureFormat {
                width: 32,
                height: 32,
                fourcc: 0x56_59_55_59,
                bytes_per_line: 64,
                image_size: 32 * 32 * 2,
            }],
            ..MockV4l2Config::default()
        };
        let mut dev = MockV4l2Device::new(config);
        dev.start_capture().expect("start");
        let f0 = dev.read_frame().expect("read");
        let f1 = dev.read_frame().expect("read");
        assert_ne!(f0, f1);
    }

    #[test]
    fn error_injection_device_busy() {
        let config = MockV4l2Config {
            fail_open: Some(MockV4l2Error::DeviceBusy),
            ..MockV4l2Config::default()
        };
        let result = MockV4l2Device::open(config);
        assert!(result.is_err());
    }

    #[test]
    fn error_injection_stream_off() {
        let mut dev = MockV4l2Device::new(MockV4l2Config::default());
        dev.start_capture().expect("start");
        dev.inject_error(MockV4l2Error::StreamOff);
        let result = dev.read_frame();
        assert!(result.is_err());
    }

    #[test]
    fn format_switching() {
        let mut dev = MockV4l2Device::new(MockV4l2Config::default());
        dev.set_format(1920, 1080, 0x56_59_55_59)
            .expect("set format");
        let fmt = dev.get_format();
        assert_eq!(fmt.width, 1920);
        assert_eq!(fmt.height, 1080);
    }

    #[test]
    fn capture_lifecycle() {
        let mut dev = MockV4l2Device::new(MockV4l2Config::default());
        dev.start_capture().expect("start");
        let frame = dev.read_frame().expect("read");
        assert!(!frame.is_empty());
        dev.stop_capture().expect("stop");
        let result = dev.read_frame();
        assert!(result.is_err());
    }

    #[test]
    fn enumerate_formats() {
        let dev = MockV4l2Device::new(MockV4l2Config::default());
        let formats = dev.enumerate_formats();
        assert!(!formats.is_empty());
        assert!(formats.iter().any(|f| f.width == 640 && f.height == 480));
    }
}
