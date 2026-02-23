//! Cast operations: type conversions

pub struct Cast;

impl Cast {
    /// Cast f32 to i8 (quantized)
    pub fn f32_to_i8(data: &[f32], scale: f32, zero_point: i8) -> Vec<i8> {
        data.iter()
            .map(|&val| {
                let scaled = (val / scale) + zero_point as f32;
                scaled.round().clamp(-128.0, 127.0) as i8
            })
            .collect()
    }

    /// Cast i8 to f32 (dequantized)
    pub fn i8_to_f32(data: &[i8], scale: f32, zero_point: i8) -> Vec<f32> {
        data.iter()
            .map(|&val| (val - zero_point) as f32 * scale)
            .collect()
    }

    /// Cast f32 to u8 (normalized [0, 255])
    pub fn f32_to_u8_normalized(data: &[f32]) -> Vec<u8> {
        data.iter()
            .map(|&val| (val * 255.0).round().clamp(0.0, 255.0) as u8)
            .collect()
    }

    /// Cast u8 to f32 (denormalized [0, 1])
    pub fn u8_to_f32_normalized(data: &[u8]) -> Vec<f32> {
        data.iter().map(|&val| val as f32 / 255.0).collect()
    }
}
