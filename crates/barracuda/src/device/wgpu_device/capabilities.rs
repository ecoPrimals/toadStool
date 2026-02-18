//! Driver capability detection — f64 workarounds, NVK/RADV detection

use super::WgpuDevice;

impl WgpuDevice {
    /// Check if this device uses the NVK (nouveau) Vulkan driver
    ///
    /// NVK is the open-source Vulkan driver for NVIDIA GPUs built on Mesa.
    /// Some f64 builtins (particularly `exp()`) crash the NAK compiler on NVK.
    pub fn is_nvk(&self) -> bool {
        let driver = self.adapter_info.driver.to_lowercase();
        let driver_info = self.adapter_info.driver_info.to_lowercase();
        driver.contains("nvk")
            || driver.contains("nouveau")
            || driver.contains("mesa")
            || driver_info.contains("nvk")
            || driver_info.contains("nouveau")
    }

    /// Check if this device uses AMD's RADV Vulkan driver
    ///
    /// RADV is the open-source Vulkan driver for AMD GPUs built on Mesa.
    pub fn is_radv(&self) -> bool {
        let driver = self.adapter_info.driver.to_lowercase();
        let driver_info = self.adapter_info.driver_info.to_lowercase();
        driver.contains("radv") || driver_info.contains("radv")
    }

    /// Whether this device needs software workarounds for f64 exp/log builtins.
    ///
    /// Known broken drivers:
    /// - NVK/NAK: crashes on native exp(f64), log(f64)
    /// - RADV/ACO (AMD open-source): `fexp2` unimplemented for f64
    ///
    /// Proprietary NVIDIA and AMD drivers handle f64 exp/log natively.
    pub fn needs_f64_exp_log_workaround(&self) -> bool {
        self.is_nvk() || self.is_radv()
    }

    /// Check if this device uses a proprietary NVIDIA driver
    pub fn is_nvidia_proprietary(&self) -> bool {
        let name = self.adapter_info.name.to_lowercase();
        let driver = self.adapter_info.driver.to_lowercase();
        (name.contains("nvidia")
            || name.contains("geforce")
            || name.contains("rtx")
            || name.contains("gtx"))
            && !self.is_nvk()
            && !driver.contains("mesa")
    }
}
