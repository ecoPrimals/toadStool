// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2025 ecoPrimals

use super::super::capabilities::DiscoverySource;

impl From<&str> for DiscoverySource {
    fn from(source_name: &str) -> Self {
        match source_name {
            "environment" => Self::Environment,
            "mdns" => Self::MDNS,
            "service_mesh" => Self::ServiceMesh("unknown".to_string()),
            "config_file" => Self::ConfigFile,
            _ => Self::Fallback,
        }
    }
}
