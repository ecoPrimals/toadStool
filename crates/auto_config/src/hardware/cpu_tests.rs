// SPDX-License-Identifier: AGPL-3.0-or-later


use super::*;

#[test]
fn test_cpu_info_default() {
    let info = CpuInfo::default();
    assert_eq!(info.model_name, "Unknown CPU");
    assert_eq!(info.physical_cores, 4);
    assert_eq!(info.logical_cores, 4);
    assert_eq!(info.family, 0);
    assert!((info.base_frequency_mhz - 2000.0).abs() < f64::EPSILON);
    assert!((info.max_frequency_mhz - 3000.0).abs() < f64::EPSILON);
    assert_eq!(info.cache_size_kb, 8192);
    assert!(info.instruction_sets.is_empty());
}

#[test]
fn test_cpu_features_default() {
    let features = CpuFeatures::default();
    assert!(!features.supports_avx);
    assert!(!features.supports_avx2);
    assert!(!features.supports_sse4_1);
    assert!(!features.supports_sse4_2);
    assert!(!features.supports_neon);
    assert!(!features.supports_riscv_v);
}

#[test]
fn test_cpu_info_serialization() {
    let info = CpuInfo {
        model_name: "Intel Core i7-9700K".to_string(),
        physical_cores: 8,
        logical_cores: 16,
        family: 6,
        base_frequency_mhz: 3600.0,
        max_frequency_mhz: 4900.0,
        cache_size_kb: 12288,
        instruction_sets: vec!["avx2".to_string(), "sse4_2".to_string()],
        features: CpuFeatures {
            supports_avx: true,
            supports_avx2: true,
            supports_sse4_1: true,
            supports_sse4_2: true,
            supports_neon: false,
            supports_riscv_v: false,
        },
    };

    let json = serde_json::to_string(&info).unwrap();
    let deserialized: CpuInfo = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.model_name, info.model_name);
    assert_eq!(deserialized.physical_cores, info.physical_cores);
    assert_eq!(deserialized.logical_cores, info.logical_cores);
    assert_eq!(deserialized.family, info.family);
}

#[test]
fn test_parse_linux_cpuinfo_full() {
    let cpuinfo = r"processor	: 0
vendor_id	: GenuineIntel
cpu family	: 6
model name	: Intel(R) Core(TM) i7-9700K CPU @ 3.60GHz
cpu MHz		: 3600.000
cache size	: 12288 KB
flags		: fpu vme de pse tsc msr pae mce cx8 apic sep mtrr pge mca cmov pat pse36 clflush mmx fxsr sse sse2 ss ht syscall nx rdtscp lm constant_tsc rep_good nopl xtopology nonstop_tsc cpuid pni pclmulqdq ssse3 fma cx16 sse4_1 sse4_2 movbe popcnt aes xsave avx f16c rdrand lahf_lm abm 3dnowprefetch invpcid_single fsgsbase tsc_adjust bmi1 avx2 smep bmi2 invpcid mpx rdseed adx smap clflushopt xsaveopt xsavec xgetbv1 xsaves
processor	: 1
model name	: Intel(R) Core(TM) i7-9700K CPU @ 3.60GHz
";

    let result = super::parse_linux_cpuinfo(cpuinfo);
    assert_eq!(
        result.model_name,
        "Intel(R) Core(TM) i7-9700K CPU @ 3.60GHz"
    );
    assert_eq!(result.physical_cores, 2);
    assert_eq!(result.logical_cores, 2);
    assert_eq!(result.family, 6);
    assert!((result.base_frequency_mhz - 3600.0).abs() < f64::EPSILON);
    assert_eq!(result.cache_size_kb, 12288);
    assert!(result.instruction_sets.contains(&"avx2".to_string()));
    assert!(result.instruction_sets.contains(&"sse4_1".to_string()));
}

#[test]
fn test_parse_linux_cpuinfo_empty() {
    let result = super::parse_linux_cpuinfo("");
    assert_eq!(result.model_name, "Unknown CPU");
    assert_eq!(result.physical_cores, 0);
    assert_eq!(result.logical_cores, 0);
}

#[test]
fn test_parse_linux_cpuinfo_arm_features() {
    let cpuinfo = r"processor	: 0
Features	: fp asimd evtstrm aes pmull sha1 sha2 crc32 atomics fphp asimdhp
model name	: ARMv8 Processor
";

    let result = super::parse_linux_cpuinfo(cpuinfo);
    assert_eq!(result.model_name, "ARMv8 Processor");
    assert_eq!(result.logical_cores, 1);
    assert!(result.instruction_sets.contains(&"asimd".to_string()));
}

#[test]
fn test_parse_linux_cpuinfo_malformed_values() {
    let cpuinfo = r"processor	: 0
cpu family	: not_a_number
cpu MHz		: invalid
cache size	: 8192 KB
model name	: Test CPU
";

    let result = super::parse_linux_cpuinfo(cpuinfo);
    assert_eq!(result.model_name, "Test CPU");
    assert_eq!(result.family, 0);
    assert_eq!(result.cache_size_kb, 8192);
}

#[test]
fn test_parse_linux_cpuinfo_cache_without_kb() {
    let cpuinfo = r"processor	: 0
cache size	: 8192 MB
model name	: Test CPU
";

    let result = super::parse_linux_cpuinfo(cpuinfo);
    assert_eq!(result.cache_size_kb, 8192);
}

#[test]
fn test_calculate_cpu_score_basic() {
    let info = CpuInfo::default();
    let score = calculate_cpu_score(&info);
    assert!(score > 0.0);
    assert!(score <= 100.0);
}

#[test]
fn test_calculate_cpu_score_high_end() {
    let info = CpuInfo {
        model_name: "High-end CPU".to_string(),
        physical_cores: 32,
        logical_cores: 64,
        family: 6,
        base_frequency_mhz: 4000.0,
        max_frequency_mhz: 5000.0,
        cache_size_kb: 32768,
        instruction_sets: Vec::new(),
        features: CpuFeatures {
            supports_avx: true,
            supports_avx2: true,
            supports_sse4_1: true,
            supports_sse4_2: true,
            supports_neon: false,
            supports_riscv_v: false,
        },
    };
    let score = calculate_cpu_score(&info);
    assert!(score >= 80.0);
}

#[test]
fn test_calculate_cpu_score_low_end() {
    let info = CpuInfo {
        model_name: "Low-end CPU".to_string(),
        physical_cores: 2,
        logical_cores: 2,
        family: 0,
        base_frequency_mhz: 1000.0,
        max_frequency_mhz: 1500.0,
        cache_size_kb: 1024,
        instruction_sets: Vec::new(),
        features: CpuFeatures::default(),
    };
    let score = calculate_cpu_score(&info);
    assert!(score < 50.0);
}
