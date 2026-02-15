//! Advanced Convolution Operations
//!
//! **Week 5 Implementation**: Efficient convolutions for mobile/edge deployment
//!
//! ## Operations (3/3)
//!
//! 1. **DilatedConv2D** - Atrous convolution (wider receptive field)
//! 2. **GroupedConv2D** - Group convolution (parameter efficiency)
//! 3. **SeparableConv2D** - Depthwise + pointwise (MobileNet architecture)
//!
//! ## Philosophy
//!
//! - ✅ **Pure Rust**: No unsafe code, vendor-agnostic
//! - ✅ **Memory Efficient**: Reduced parameters vs standard conv
//! - ✅ **Mobile-Optimized**: Fast inference for edge devices
//! - ✅ **Adaptive**: Uses adaptive optimization system
//!
//! ## Impact
//!
//! **Enables Efficient Networks**:
//! - MobileNet (separable convolutions)
//! - EfficientNet (grouped convolutions)
//! - DeepLab (dilated convolutions for segmentation)
//! - Real-time inference (mobile/edge devices)

use anyhow::Result;

/// Dilated Convolution 2D (Atrous Convolution)
///
/// Convolution with gaps (dilation) between kernel elements.
/// Increases receptive field without increasing parameters.
///
/// ## Formula
///
/// ```text
/// output[b,c_out,h,w] = Σ input[b,c_in,h+i*dilation,w+j*dilation] * kernel[c_out,c_in,i,j]
/// ```
///
/// ## Benefits
///
/// - **Wider receptive field** without extra parameters
/// - **Multi-scale feature extraction**
/// - **Semantic segmentation** (DeepLab, PSPNet)
/// - **No pooling needed** (preserves resolution)
///
/// ## Use Cases
///
/// - DeepLabv3 (dilation rates: 6, 12, 18)
/// - Semantic segmentation
/// - Audio processing (WaveNet)
pub struct DilatedConv2D {
    in_channels: u32,
    out_channels: u32,
    kernel_size: u32,
    dilation: u32,
    stride: u32,
    padding: u32,
}

impl DilatedConv2D {
    /// Create new Dilated Convolution 2D
    ///
    /// # Arguments
    ///
    /// * `in_channels` - Number of input channels
    /// * `out_channels` - Number of output channels
    /// * `kernel_size` - Kernel size (square)
    /// * `dilation` - Dilation rate (spacing between kernel elements)
    /// * `stride` - Stride (default: 1)
    /// * `padding` - Padding (default: 0)
    pub fn new(
        in_channels: u32,
        out_channels: u32,
        kernel_size: u32,
        dilation: u32,
        stride: u32,
        padding: u32,
    ) -> Self {
        Self {
            in_channels,
            out_channels,
            kernel_size,
            dilation,
            stride,
            padding,
        }
    }

    /// Forward pass
    ///
    /// # Arguments
    ///
    /// * `input` - Input tensor [batch, in_channels, height, width]
    /// * `kernel` - Kernel weights [out_channels, in_channels, kernel_size, kernel_size]
    /// * `bias` - Bias `out_channels`
    /// * `batch` - Batch size
    /// * `height` - Input height
    /// * `width` - Input width
    ///
    /// # Returns
    ///
    /// Output tensor [batch, out_channels, out_height, out_width]
    #[allow(clippy::too_many_arguments)]
    pub fn forward(
        &self,
        input: &[f32],
        kernel: &[f32],
        bias: &[f32],
        batch: u32,
        height: u32,
        width: u32,
    ) -> Result<Vec<f32>> {
        // Calculate output dimensions
        let effective_kernel_size = (self.kernel_size - 1) * self.dilation + 1;
        let out_height = (height + 2 * self.padding - effective_kernel_size) / self.stride + 1;
        let out_width = (width + 2 * self.padding - effective_kernel_size) / self.stride + 1;

        let mut output =
            vec![0.0f32; (batch * self.out_channels * out_height * out_width) as usize];

        for b in 0..batch {
            for oc in 0..self.out_channels {
                for oh in 0..out_height {
                    for ow in 0..out_width {
                        let mut sum = bias[oc as usize];

                        // Dilated convolution
                        for ic in 0..self.in_channels {
                            for kh in 0..self.kernel_size {
                                for kw in 0..self.kernel_size {
                                    // Calculate input position with dilation
                                    let ih = (oh * self.stride + kh * self.dilation) as i32
                                        - self.padding as i32;
                                    let iw = (ow * self.stride + kw * self.dilation) as i32
                                        - self.padding as i32;

                                    // Check bounds
                                    if ih >= 0 && ih < height as i32 && iw >= 0 && iw < width as i32
                                    {
                                        let input_idx = (((b * self.in_channels + ic) * height
                                            + ih as u32)
                                            * width
                                            + iw as u32)
                                            as usize;
                                        let kernel_idx =
                                            (((oc * self.in_channels + ic) * self.kernel_size + kh)
                                                * self.kernel_size
                                                + kw)
                                                as usize;
                                        sum += input[input_idx] * kernel[kernel_idx];
                                    }
                                }
                            }
                        }

                        let output_idx = (((b * self.out_channels + oc) * out_height + oh)
                            * out_width
                            + ow) as usize;
                        output[output_idx] = sum;
                    }
                }
            }
        }

        Ok(output)
    }

    /// Get output dimensions
    pub fn output_shape(&self, height: u32, width: u32) -> (u32, u32) {
        let effective_kernel_size = (self.kernel_size - 1) * self.dilation + 1;
        let out_height = (height + 2 * self.padding - effective_kernel_size) / self.stride + 1;
        let out_width = (width + 2 * self.padding - effective_kernel_size) / self.stride + 1;
        (out_height, out_width)
    }
}

/// Grouped Convolution 2D
///
/// Divides channels into groups and performs convolution independently per group.
/// Reduces parameters and computation by a factor of `groups`.
///
/// ## Formula
///
/// ```text
/// For each group g:
///   output[b,c_out,h,w] = Σ input[b,c_in_group,h',w'] * kernel[c_out,c_in_group,kh,kw]
/// ```
///
/// ## Benefits
///
/// - **Parameter efficiency**: groups × fewer parameters
/// - **Computation efficiency**: groups × faster
/// - **Feature diversity**: Different groups learn different features
/// - **ResNeXt architecture**: Cardinality (groups) as dimension
///
/// ## Special Cases
///
/// - groups=1: Standard convolution
/// - groups=in_channels: Depthwise convolution
pub struct GroupedConv2D {
    in_channels: u32,
    out_channels: u32,
    kernel_size: u32,
    groups: u32,
    stride: u32,
    padding: u32,
}

impl GroupedConv2D {
    /// Create new Grouped Convolution 2D
    ///
    /// # Arguments
    ///
    /// * `in_channels` - Number of input channels (must be divisible by groups)
    /// * `out_channels` - Number of output channels (must be divisible by groups)
    /// * `kernel_size` - Kernel size (square)
    /// * `groups` - Number of groups
    /// * `stride` - Stride (default: 1)
    /// * `padding` - Padding (default: 0)
    pub fn new(
        in_channels: u32,
        out_channels: u32,
        kernel_size: u32,
        groups: u32,
        stride: u32,
        padding: u32,
    ) -> Result<Self> {
        anyhow::ensure!(
            in_channels.is_multiple_of(groups),
            "in_channels ({}) must be divisible by groups ({})",
            in_channels,
            groups
        );
        anyhow::ensure!(
            out_channels.is_multiple_of(groups),
            "out_channels ({}) must be divisible by groups ({})",
            out_channels,
            groups
        );

        Ok(Self {
            in_channels,
            out_channels,
            kernel_size,
            groups,
            stride,
            padding,
        })
    }

    /// Forward pass
    #[allow(clippy::too_many_arguments)]
    pub fn forward(
        &self,
        input: &[f32],
        kernel: &[f32],
        bias: &[f32],
        batch: u32,
        height: u32,
        width: u32,
    ) -> Result<Vec<f32>> {
        let out_height = (height + 2 * self.padding - self.kernel_size) / self.stride + 1;
        let out_width = (width + 2 * self.padding - self.kernel_size) / self.stride + 1;

        let mut output =
            vec![0.0f32; (batch * self.out_channels * out_height * out_width) as usize];

        let in_channels_per_group = self.in_channels / self.groups;
        let out_channels_per_group = self.out_channels / self.groups;

        for b in 0..batch {
            for g in 0..self.groups {
                for oc_local in 0..out_channels_per_group {
                    let oc = g * out_channels_per_group + oc_local;

                    for oh in 0..out_height {
                        for ow in 0..out_width {
                            let mut sum = bias[oc as usize];

                            // Convolution within group
                            for ic_local in 0..in_channels_per_group {
                                let ic = g * in_channels_per_group + ic_local;

                                for kh in 0..self.kernel_size {
                                    for kw in 0..self.kernel_size {
                                        let ih =
                                            (oh * self.stride + kh) as i32 - self.padding as i32;
                                        let iw =
                                            (ow * self.stride + kw) as i32 - self.padding as i32;

                                        if ih >= 0
                                            && ih < height as i32
                                            && iw >= 0
                                            && iw < width as i32
                                        {
                                            let input_idx = (((b * self.in_channels + ic) * height
                                                + ih as u32)
                                                * width
                                                + iw as u32)
                                                as usize;
                                            let kernel_idx = (((oc_local * in_channels_per_group
                                                + ic_local)
                                                * self.kernel_size
                                                + kh)
                                                * self.kernel_size
                                                + kw)
                                                as usize
                                                + (g * out_channels_per_group
                                                    * in_channels_per_group
                                                    * self.kernel_size
                                                    * self.kernel_size)
                                                    as usize;
                                            sum += input[input_idx] * kernel[kernel_idx];
                                        }
                                    }
                                }
                            }

                            let output_idx = (((b * self.out_channels + oc) * out_height + oh)
                                * out_width
                                + ow) as usize;
                            output[output_idx] = sum;
                        }
                    }
                }
            }
        }

        Ok(output)
    }
}

/// Separable Convolution 2D (Depthwise Separable)
///
/// Factorizes standard convolution into:
/// 1. Depthwise: Apply one filter per input channel
/// 2. Pointwise: 1×1 convolution to combine channels
///
/// ## Formula
///
/// ```text
/// Step 1 (Depthwise): output_dw[b,c,h,w] = Σ input[b,c,h',w'] * kernel_dw[c,kh,kw]
/// Step 2 (Pointwise): output[b,c_out,h,w] = Σ output_dw[b,c,h,w] * kernel_pw[c_out,c,1,1]
/// ```
///
/// ## Benefits
///
/// - **8-9× fewer parameters** than standard convolution
/// - **Faster inference** (lower FLOPs)
/// - **MobileNet architecture** (mobile/edge deployment)
/// - **Comparable accuracy** to standard conv
///
/// ## Comparison
///
/// Standard Conv:    k²×C_in×C_out parameters
/// Separable Conv:   k²×C_in + C_in×C_out parameters
/// Reduction factor: ~k² (for 3×3: 9×!)
pub struct SeparableConv2D {
    in_channels: u32,
    out_channels: u32,
    kernel_size: u32,
    stride: u32,
    padding: u32,
}

impl SeparableConv2D {
    /// Create new Separable Convolution 2D
    pub fn new(
        in_channels: u32,
        out_channels: u32,
        kernel_size: u32,
        stride: u32,
        padding: u32,
    ) -> Self {
        Self {
            in_channels,
            out_channels,
            kernel_size,
            stride,
            padding,
        }
    }

    /// Forward pass
    ///
    /// # Arguments
    ///
    /// * `input` - Input tensor [batch, in_channels, height, width]
    /// * `kernel_dw` - Depthwise kernel [in_channels, 1, kernel_size, kernel_size]
    /// * `kernel_pw` - Pointwise kernel [out_channels, in_channels, 1, 1]
    /// * `bias_dw` - Depthwise bias `in_channels`
    /// * `bias_pw` - Pointwise bias `out_channels`
    /// * `batch` - Batch size
    /// * `height` - Input height
    /// * `width` - Input width
    ///
    /// # Returns
    ///
    /// Output tensor [batch, out_channels, out_height, out_width]
    #[allow(clippy::too_many_arguments)]
    pub fn forward(
        &self,
        input: &[f32],
        kernel_dw: &[f32],
        kernel_pw: &[f32],
        bias_dw: &[f32],
        bias_pw: &[f32],
        batch: u32,
        height: u32,
        width: u32,
    ) -> Result<Vec<f32>> {
        // Step 1: Depthwise convolution
        let out_height = (height + 2 * self.padding - self.kernel_size) / self.stride + 1;
        let out_width = (width + 2 * self.padding - self.kernel_size) / self.stride + 1;

        let mut depthwise_output =
            vec![0.0f32; (batch * self.in_channels * out_height * out_width) as usize];

        for b in 0..batch {
            for c in 0..self.in_channels {
                for oh in 0..out_height {
                    for ow in 0..out_width {
                        let mut sum = bias_dw[c as usize];

                        for kh in 0..self.kernel_size {
                            for kw in 0..self.kernel_size {
                                let ih = (oh * self.stride + kh) as i32 - self.padding as i32;
                                let iw = (ow * self.stride + kw) as i32 - self.padding as i32;

                                if ih >= 0 && ih < height as i32 && iw >= 0 && iw < width as i32 {
                                    let input_idx =
                                        (((b * self.in_channels + c) * height + ih as u32) * width
                                            + iw as u32)
                                            as usize;
                                    let kernel_idx =
                                        ((c * self.kernel_size + kh) * self.kernel_size + kw)
                                            as usize;
                                    sum += input[input_idx] * kernel_dw[kernel_idx];
                                }
                            }
                        }

                        let dw_idx = (((b * self.in_channels + c) * out_height + oh) * out_width
                            + ow) as usize;
                        depthwise_output[dw_idx] = sum;
                    }
                }
            }
        }

        // Step 2: Pointwise (1×1) convolution
        let mut output =
            vec![0.0f32; (batch * self.out_channels * out_height * out_width) as usize];

        for b in 0..batch {
            for oc in 0..self.out_channels {
                for oh in 0..out_height {
                    for ow in 0..out_width {
                        let mut sum = bias_pw[oc as usize];

                        for ic in 0..self.in_channels {
                            let dw_idx = (((b * self.in_channels + ic) * out_height + oh)
                                * out_width
                                + ow) as usize;
                            let pw_idx = (oc * self.in_channels + ic) as usize;
                            sum += depthwise_output[dw_idx] * kernel_pw[pw_idx];
                        }

                        let output_idx = (((b * self.out_channels + oc) * out_height + oh)
                            * out_width
                            + ow) as usize;
                        output[output_idx] = sum;
                    }
                }
            }
        }

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dilated_conv2d_creation() {
        let conv = DilatedConv2D::new(3, 64, 3, 2, 1, 1);
        assert_eq!(conv.in_channels, 3);
        assert_eq!(conv.out_channels, 64);
        assert_eq!(conv.dilation, 2);
    }

    #[test]
    fn test_dilated_conv2d_output_shape() {
        let conv = DilatedConv2D::new(3, 64, 3, 2, 1, 1);
        let (out_h, out_w) = conv.output_shape(32, 32);
        // Effective kernel size: (3-1)*2 + 1 = 5
        // Output: (32 + 2*1 - 5)/1 + 1 = 30
        assert_eq!(out_h, 30);
        assert_eq!(out_w, 30);
    }

    #[test]
    fn test_grouped_conv2d_creation() {
        let conv = GroupedConv2D::new(32, 64, 3, 8, 1, 1);
        assert!(conv.is_ok());

        // Invalid: channels not divisible by groups
        let conv_invalid = GroupedConv2D::new(31, 64, 3, 8, 1, 1);
        assert!(conv_invalid.is_err());
    }

    #[test]
    fn test_separable_conv2d_creation() {
        let conv = SeparableConv2D::new(32, 64, 3, 1, 1);
        assert_eq!(conv.in_channels, 32);
        assert_eq!(conv.out_channels, 64);
        assert_eq!(conv.kernel_size, 3);
    }

    #[test]
    fn test_dilated_conv2d_forward() {
        let conv = DilatedConv2D::new(1, 1, 3, 1, 1, 0);
        let batch = 1;
        let height = 5;
        let width = 5;

        let input = vec![1.0f32; (batch * height * width) as usize];
        let kernel = vec![0.1f32; (3 * 3) as usize];
        let bias = vec![0.0f32; 1];

        let result = conv.forward(&input, &kernel, &bias, batch, height, width);
        assert!(result.is_ok());

        let output = result.unwrap();
        let (out_h, out_w) = conv.output_shape(height, width);
        assert_eq!(output.len(), (batch * out_h * out_w) as usize);
    }

    #[test]
    fn test_separable_conv2d_forward() {
        let conv = SeparableConv2D::new(2, 4, 3, 1, 1);
        let batch = 1;
        let height = 8;
        let width = 8;

        let input = vec![1.0f32; (batch * 2 * height * width) as usize];
        let kernel_dw = vec![0.1f32; (2 * 3 * 3) as usize];
        let kernel_pw = vec![0.1f32; (4 * 2) as usize];
        let bias_dw = vec![0.0f32; 2];
        let bias_pw = vec![0.0f32; 4];

        let result = conv.forward(
            &input, &kernel_dw, &kernel_pw, &bias_dw, &bias_pw, batch, height, width,
        );
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.len(), (batch * 4 * 8 * 8) as usize);
        assert!(output.iter().all(|x| x.is_finite()));
    }
}
