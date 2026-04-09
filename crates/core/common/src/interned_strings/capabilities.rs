// SPDX-License-Identifier: AGPL-3.0-or-later
//! Capability type constants (Deep Debt compliant).
//!
//! These represent WHAT services do, not WHO provides them.
//! Use these for capability-based discovery! Never match on primal names.

/// Security capabilities (encryption, signing, key management)
pub const SECURITY: &str = "security";

/// Cryptographic capabilities (encryption, signing, key management, PKI).
/// Use for discovery: `discover_capability(capabilities::CRYPTO)`.
pub const CRYPTO: &str = "crypto";

/// Storage capabilities (persistence, compression, versioning)
pub const STORAGE: &str = "storage";

/// Coordination capabilities (service mesh, discovery, orchestration)
pub const COORDINATION: &str = "coordination";

/// Workload routing / MCP-style agent IPC
pub const ROUTING: &str = "routing";

/// AI/ML capabilities (inference, training, natural language)
pub const INTELLIGENCE: &str = "intelligence";

/// Compute capabilities (CPU, GPU, specialized hardware)
pub const COMPUTE: &str = "compute";

/// Monitoring capabilities (metrics, logging, tracing)
pub const MONITORING: &str = "monitoring";

/// Networking capabilities (routing, tunneling, VPN)
pub const NETWORKING: &str = "networking";

// Specific capability features

/// Encryption capability
pub const ENCRYPTION: &str = "encryption";

/// Digital signing capability
pub const SIGNING: &str = "signing";

/// Key management capability
pub const KEY_MANAGEMENT: &str = "key-management";

/// Public Key Infrastructure
pub const PKI: &str = "pki";

/// Audit logging capability
pub const AUDIT: &str = "audit";

/// Data persistence capability
pub const PERSISTENCE: &str = "persistence";

/// Data compression capability
pub const COMPRESSION: &str = "compression";

/// Version control capability
pub const VERSIONING: &str = "versioning";

/// GPU dispatch capability (native shader / compute pipeline)
pub const GPU_DISPATCH: &str = "gpu.dispatch";

/// Science GPU dispatch (JSON-RPC method family)
pub const SCIENCE_GPU_DISPATCH: &str = "science.gpu.dispatch";

/// Shader compilation capability (sovereign pipeline)
pub const SHADER_COMPILE: &str = "shader.compile";

/// Native shader compilation pipeline
pub const SHADER_COMPILE_NATIVE: &str = "shader.compile.native";

/// GPU hardware calibration (NVVM safety, precision tier probing).
pub const GPU_CALIBRATION: &str = "gpu.calibration";

/// Workload routing (substrate selection based on problem size).
pub const WORKLOAD_ROUTING: &str = "workload.routing";

/// Orchestration capability
pub const ORCHESTRATION: &str = "orchestration";

/// Ecology domain capability (airSpring)
pub const ECOLOGY: &str = "ecology";

/// Science domain capability
pub const SCIENCE: &str = "science";

/// Activation function capabilities (science GPU stack)
pub const ACTIVATIONS: &str = "science.activations";

/// RNG capabilities
pub const RNG: &str = "science.rng";

/// Special math functions
pub const SPECIAL_FUNCTIONS: &str = "science.special";

/// Biology domain capability (wetSpring — metagenomics, phylogenetics, mass spec)
pub const BIOLOGY: &str = "biology";

/// Health domain capability (healthSpring — PK/PD, NLME, biosignal)
pub const HEALTH: &str = "health";

/// Measurement/uncertainty domain capability (groundSpring — UQ, validation)
pub const MEASUREMENT: &str = "measurement";

/// Optimization domain capability (neuralSpring — ML, evolutionary computation)
pub const OPTIMIZATION: &str = "optimization";

/// Visualization / streaming pipeline capability (petalTongue)
pub const VISUALIZATION: &str = "visualization";
