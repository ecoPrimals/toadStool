// SPDX-License-Identifier: AGPL-3.0-or-later
//! USB vendor/product filter discovery (placeholder).

use std::sync::Arc;

use toadstool::error::ToadStoolResult;

use crate::platforms::*;

use super::DiscoveryMethod;

/// USB Device Discovery Method
pub struct USBDiscovery {
    pub(super) vendor_filters: Vec<u16>,
    pub(super) product_filters: Vec<u16>,
}

#[async_trait::async_trait]
impl DiscoveryMethod for USBDiscovery {
    fn get_name(&self) -> &str {
        "USB Discovery"
    }

    async fn discover(&self) -> ToadStoolResult<Vec<Arc<dyn EdgeDevice>>> {
        // USB discovery is largely covered by serial port discovery
        // This could be extended to handle other USB device types
        Ok(Vec::new())
    }

    async fn is_available(&self) -> bool {
        // Check if USB subsystem is available
        true
    }

    fn get_supported_types(&self) -> Vec<String> {
        vec!["USB Device".to_string()]
    }
}
