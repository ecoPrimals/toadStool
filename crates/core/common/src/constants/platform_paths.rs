// SPDX-License-Identifier: AGPL-3.0-or-later
//! Well-known platform filesystem paths (`procfs`, `devfs`, `sysfs`, `/etc`, install locations).
//!
//! Linux-oriented constants used by production detection and monitoring code.

/// Paths under `/proc`.
pub mod procfs {
    /// `/proc/cpuinfo`
    pub const CPUINFO: &str = "/proc/cpuinfo";
    /// `/proc/meminfo`
    pub const MEMINFO: &str = "/proc/meminfo";
    /// `/proc/loadavg`
    pub const LOADAVG: &str = "/proc/loadavg";
    /// `/proc/1/cgroup` (init process cgroups)
    pub const PROC_INIT_CGROUP: &str = "/proc/1/cgroup";
    /// `/proc/self/cgroup`
    pub const PROC_SELF_CGROUP: &str = "/proc/self/cgroup";
    /// `/proc/self/loginuid`
    pub const PROC_SELF_LOGINUID: &str = "/proc/self/loginuid";
    /// `/proc/net/dev` (aggregate network stats)
    pub const PROC_NET_DEV: &str = "/proc/net/dev";
    /// `/proc/version` (kernel release string)
    pub const VERSION: &str = "/proc/version";
    /// `/proc/net/if_inet6` (IPv6 addresses per interface)
    pub const NET_IF_INET6: &str = "/proc/net/if_inet6";

    /// `/proc/{pid}/stat`
    #[must_use]
    pub fn proc_pid_stat(pid: u32) -> String {
        format!("/proc/{pid}/stat")
    }

    /// `/proc/{pid}/status`
    #[must_use]
    pub fn proc_pid_status(pid: u32) -> String {
        format!("/proc/{pid}/status")
    }

    /// `/proc/{pid}/io`
    #[must_use]
    pub fn proc_pid_io(pid: u32) -> String {
        format!("/proc/{pid}/io")
    }

    /// `/proc/{pid}/net/dev`
    #[must_use]
    pub fn proc_pid_net_dev(pid: u32) -> String {
        format!("/proc/{pid}/net/dev")
    }
}

/// Device nodes and device filesystem directories.
pub mod devfs {
    /// `/dev/kvm`
    pub const KVM: &str = "/dev/kvm";
    /// `/dev/dri` (DRM render nodes)
    pub const DRI_DIR: &str = "/dev/dri";
    /// VFIO container device (`/dev/vfio/vfio`)
    pub const VFIO_CONTAINER: &str = "/dev/vfio/vfio";
}

/// sysfs hierarchy roots used by discovery.
pub mod sysfs {
    /// PCI devices under sysfs (`/sys/bus/pci/devices`)
    pub const BUS_PCI_DEVICES: &str = "/sys/bus/pci/devices";
    /// `/sys/class/bluetooth`
    pub const CLASS_BLUETOOTH: &str = "/sys/class/bluetooth";
    /// `/sys/block` (block device symlinks)
    pub const BLOCK: &str = "/sys/block";
    /// `/sys/class/net` (network interface sysfs)
    pub const CLASS_NET: &str = "/sys/class/net";
    /// USB devices on the USB bus (`/sys/bus/usb/devices`)
    pub const BUS_USB_DEVICES: &str = "/sys/bus/usb/devices";
    /// Bluetooth devices enumerated by the kernel (`/sys/bus/bluetooth/devices`)
    pub const BUS_BLUETOOTH_DEVICES: &str = "/sys/bus/bluetooth/devices";
    /// DMI product name (`/sys/class/dmi/id/product_name`)
    pub const CLASS_DMI_ID_PRODUCT_NAME: &str = "/sys/class/dmi/id/product_name";
    /// First SATA disk rotational flag (`/sys/block/sda/queue/rotational`)
    pub const BLOCK_SDA_QUEUE_ROTATIONAL: &str = "/sys/block/sda/queue/rotational";
}

/// Common files under `/etc`.
pub mod etc_paths {
    /// `/etc/os-release`
    pub const OS_RELEASE: &str = "/etc/os-release";
    /// Static pod manifests directory on kubelet nodes
    pub const KUBERNETES_MANIFESTS: &str = "/etc/kubernetes/manifests";
    /// System-wide ToadStool config directory
    pub const TOADSTOOL_DIR: &str = "/etc/toadstool";
    /// System-wide services manifest
    pub const TOADSTOOL_SERVICES_TOML: &str = "/etc/toadstool/services.toml";
    /// Resolver configuration
    pub const RESOLV_CONF: &str = "/etc/resolv.conf";
    /// POSIX user database
    pub const PASSWD: &str = "/etc/passwd";
}

/// Install-time and log paths (Linux defaults).
pub mod install_paths {
    /// Default opt install root
    pub const OPT_TOADSTOOL: &str = "/opt/toadstool";
    /// Default audit log file
    pub const VAR_LOG_TOADSTOOL_AUDIT: &str = "/var/log/toadstool/audit.log";
}
