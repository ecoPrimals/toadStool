// SPDX-License-Identifier: AGPL-3.0-only
//! VFIO ioctl wrappers — thin delegation to `hw-safe::vfio_setup`.
//!
//! All VFIO setup ioctls are handled by the shared `hw-safe::vfio_setup`
//! module. This file is retained for any future device-specific ioctls
//! that don't belong in the shared module.
