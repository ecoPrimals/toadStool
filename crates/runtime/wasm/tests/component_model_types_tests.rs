//! Comprehensive tests for WASM component model types

use toadstool_runtime_wasm::component_model::{
    ComponentInterface, ComponentModelConfig, ComponentResourceUsage, ComponentState,
    InterfaceFunction, InterfaceType,
};

// ============================================================================
// ComponentModelConfig Tests
// ============================================================================

#[test]
fn test_component_model_config_default() {
    let config = ComponentModelConfig::default();

    assert!(config.enabled);
    assert_eq!(config.max_instances, 1000);
    assert_eq!(config.linking_timeout_ms, 5000);
    assert!(config.composition_enabled);
    assert!(config.wit_support);
}

#[test]
fn test_component_model_config_custom() {
    let config = ComponentModelConfig {
        enabled: false,
        max_instances: 500,
        linking_timeout_ms: 10000,
        composition_enabled: false,
        wit_support: false,
    };

    assert!(!config.enabled);
    assert_eq!(config.max_instances, 500);
}

#[test]
fn test_component_model_config_serialization() {
    let config = ComponentModelConfig::default();

    let json = serde_json::to_string(&config).unwrap();
    assert!(!json.is_empty());

    let deserialized: ComponentModelConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.max_instances, 1000);
}

#[test]
fn test_component_model_config_zero_instances() {
    let config = ComponentModelConfig {
        enabled: true,
        max_instances: 0,
        linking_timeout_ms: 5000,
        composition_enabled: true,
        wit_support: true,
    };

    assert_eq!(config.max_instances, 0);
}

#[test]
fn test_component_model_config_very_high_timeout() {
    let config = ComponentModelConfig {
        enabled: true,
        max_instances: 1000,
        linking_timeout_ms: 60000,
        composition_enabled: true,
        wit_support: true,
    };

    assert_eq!(config.linking_timeout_ms, 60000);
}

// ============================================================================
// ComponentInterface Tests
// ============================================================================

#[test]
fn test_component_interface_creation() {
    let interface = ComponentInterface {
        name: "wasi:http/handler".to_string(),
        version: "0.2.0".to_string(),
        exports: vec![],
        imports: vec![],
        types: vec![],
    };

    assert_eq!(interface.name, "wasi:http/handler");
    assert_eq!(interface.version, "0.2.0");
}

#[test]
fn test_component_interface_with_exports() {
    let func = InterfaceFunction {
        name: "handle".to_string(),
        params: vec![InterfaceType::String],
        return_type: Some(InterfaceType::String),
        docs: Some("Handle HTTP request".to_string()),
    };

    let interface = ComponentInterface {
        name: "handler".to_string(),
        version: "1.0.0".to_string(),
        exports: vec![func],
        imports: vec![],
        types: vec![],
    };

    assert_eq!(interface.exports.len(), 1);
}

#[test]
fn test_component_interface_with_imports() {
    let func = InterfaceFunction {
        name: "fetch".to_string(),
        params: vec![InterfaceType::String],
        return_type: Some(InterfaceType::String),
        docs: None,
    };

    let interface = ComponentInterface {
        name: "http-client".to_string(),
        version: "1.0.0".to_string(),
        exports: vec![],
        imports: vec![func],
        types: vec![],
    };

    assert_eq!(interface.imports.len(), 1);
}

#[test]
fn test_component_interface_with_types() {
    let interface = ComponentInterface {
        name: "types".to_string(),
        version: "1.0.0".to_string(),
        exports: vec![],
        imports: vec![],
        types: vec![
            InterfaceType::Bool,
            InterfaceType::U32,
            InterfaceType::String,
        ],
    };

    assert_eq!(interface.types.len(), 3);
}

#[test]
fn test_component_interface_serialization() {
    let interface = ComponentInterface {
        name: "test".to_string(),
        version: "0.1.0".to_string(),
        exports: vec![],
        imports: vec![],
        types: vec![],
    };

    let json = serde_json::to_string(&interface).unwrap();
    assert!(!json.is_empty());

    let deserialized: ComponentInterface = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.name, "test");
}

// ============================================================================
// InterfaceFunction Tests
// ============================================================================

#[test]
fn test_interface_function_no_params() {
    let func = InterfaceFunction {
        name: "get_version".to_string(),
        params: vec![],
        return_type: Some(InterfaceType::String),
        docs: None,
    };

    assert!(func.params.is_empty());
    assert!(func.return_type.is_some());
}

#[test]
fn test_interface_function_multiple_params() {
    let func = InterfaceFunction {
        name: "add".to_string(),
        params: vec![InterfaceType::U32, InterfaceType::U32],
        return_type: Some(InterfaceType::U32),
        docs: Some("Add two numbers".to_string()),
    };

    assert_eq!(func.params.len(), 2);
}

#[test]
fn test_interface_function_no_return_type() {
    let func = InterfaceFunction {
        name: "log".to_string(),
        params: vec![InterfaceType::String],
        return_type: None,
        docs: Some("Log a message".to_string()),
    };

    assert!(func.return_type.is_none());
}

#[test]
fn test_interface_function_with_docs() {
    let func = InterfaceFunction {
        name: "process".to_string(),
        params: vec![InterfaceType::String],
        return_type: Some(InterfaceType::Bool),
        docs: Some("Process the input and return success status".to_string()),
    };

    assert!(func.docs.is_some());
    assert!(func.docs.unwrap().contains("Process"));
}

// ============================================================================
// InterfaceType Tests
// ============================================================================

#[test]
fn test_interface_type_bool() {
    let t = InterfaceType::Bool;
    let json = serde_json::to_string(&t).unwrap();
    let deserialized: InterfaceType = serde_json::from_str(&json).unwrap();

    match deserialized {
        InterfaceType::Bool => (),
        _ => panic!("Expected Bool"),
    }
}

#[test]
fn test_interface_type_integers() {
    let types = vec![
        InterfaceType::U8,
        InterfaceType::U16,
        InterfaceType::U32,
        InterfaceType::U64,
        InterfaceType::S8,
        InterfaceType::S16,
        InterfaceType::S32,
        InterfaceType::S64,
    ];

    assert_eq!(types.len(), 8);
}

#[test]
fn test_interface_type_floats() {
    let f32_type = InterfaceType::F32;
    let f64_type = InterfaceType::F64;

    let json1 = serde_json::to_string(&f32_type).unwrap();
    let json2 = serde_json::to_string(&f64_type).unwrap();

    assert_ne!(json1, json2);
}

#[test]
fn test_interface_type_string() {
    let t = InterfaceType::String;
    let json = serde_json::to_string(&t).unwrap();
    assert!(!json.is_empty());
}

#[test]
fn test_interface_type_list() {
    let t = InterfaceType::List(Box::new(InterfaceType::U32));
    let json = serde_json::to_string(&t).unwrap();
    let deserialized: InterfaceType = serde_json::from_str(&json).unwrap();

    match deserialized {
        InterfaceType::List(_) => (),
        _ => panic!("Expected List"),
    }
}

#[test]
fn test_interface_type_record() {
    let t = InterfaceType::Record(vec![
        ("name".to_string(), InterfaceType::String),
        ("age".to_string(), InterfaceType::U32),
    ]);

    match &t {
        InterfaceType::Record(fields) => {
            assert_eq!(fields.len(), 2);
        }
        _ => panic!("Expected Record"),
    }
}

#[test]
fn test_interface_type_variant() {
    let t = InterfaceType::Variant(vec![
        ("Success".to_string(), Some(InterfaceType::String)),
        ("Error".to_string(), Some(InterfaceType::String)),
    ]);

    match &t {
        InterfaceType::Variant(cases) => {
            assert_eq!(cases.len(), 2);
        }
        _ => panic!("Expected Variant"),
    }
}

#[test]
fn test_interface_type_option() {
    let t = InterfaceType::Option(Box::new(InterfaceType::U32));

    match &t {
        InterfaceType::Option(inner) => match **inner {
            InterfaceType::U32 => (),
            _ => panic!("Expected U32 inner type"),
        },
        _ => panic!("Expected Option"),
    }
}

#[test]
fn test_interface_type_result() {
    let t = InterfaceType::Result(
        Box::new(InterfaceType::String),
        Box::new(InterfaceType::String),
    );

    match &t {
        InterfaceType::Result(_, _) => (),
        _ => panic!("Expected Result"),
    }
}

#[test]
fn test_interface_type_custom() {
    let t = InterfaceType::Custom("MyCustomType".to_string());

    match &t {
        InterfaceType::Custom(name) => {
            assert_eq!(name, "MyCustomType");
        }
        _ => panic!("Expected Custom"),
    }
}

// ============================================================================
// ComponentState Tests
// ============================================================================

#[test]
fn test_component_state_initializing() {
    let state = ComponentState::Initializing;
    let json = serde_json::to_string(&state).unwrap();
    let deserialized: ComponentState = serde_json::from_str(&json).unwrap();

    match deserialized {
        ComponentState::Initializing => (),
        _ => panic!("Expected Initializing"),
    }
}

#[test]
fn test_component_state_ready() {
    let state = ComponentState::Ready;
    let json = serde_json::to_string(&state).unwrap();
    assert!(!json.is_empty());
}

#[test]
fn test_component_state_running() {
    let state = ComponentState::Running;
    let json = serde_json::to_string(&state).unwrap();
    let deserialized: ComponentState = serde_json::from_str(&json).unwrap();

    match deserialized {
        ComponentState::Running => (),
        _ => panic!("Expected Running"),
    }
}

#[test]
fn test_component_state_failed() {
    let state = ComponentState::Failed {
        error: "Component crashed".to_string(),
    };

    match &state {
        ComponentState::Failed { error } => {
            assert_eq!(error, "Component crashed");
        }
        _ => panic!("Expected Failed"),
    }
}

#[test]
fn test_component_state_terminating() {
    let state = ComponentState::Terminating;
    let json = serde_json::to_string(&state).unwrap();
    let deserialized: ComponentState = serde_json::from_str(&json).unwrap();

    match deserialized {
        ComponentState::Terminating => (),
        _ => panic!("Expected Terminating"),
    }
}

// ============================================================================
// ComponentResourceUsage Tests
// ============================================================================

#[test]
fn test_component_resource_usage_default() {
    let usage = ComponentResourceUsage::default();

    assert_eq!(usage.memory_bytes, 0);
    assert_eq!(usage.cpu_time_ms, 0);
    assert_eq!(usage.function_calls, 0);
    assert_eq!(usage.imports_count, 0);
    assert_eq!(usage.exports_count, 0);
}

#[test]
fn test_component_resource_usage_custom() {
    let usage = ComponentResourceUsage {
        memory_bytes: 1024000,
        cpu_time_ms: 500,
        function_calls: 100,
        imports_count: 5,
        exports_count: 10,
    };

    assert_eq!(usage.memory_bytes, 1024000);
    assert_eq!(usage.function_calls, 100);
}

#[test]
fn test_component_resource_usage_clone() {
    let usage = ComponentResourceUsage {
        memory_bytes: 2048,
        cpu_time_ms: 100,
        function_calls: 50,
        imports_count: 2,
        exports_count: 3,
    };

    let cloned = usage.clone();
    assert_eq!(cloned.memory_bytes, 2048);
    assert_eq!(cloned.function_calls, 50);
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_complete_component_interface() {
    let func = InterfaceFunction {
        name: "process".to_string(),
        params: vec![InterfaceType::String, InterfaceType::U32],
        return_type: Some(InterfaceType::Bool),
        docs: Some("Process data".to_string()),
    };

    let interface = ComponentInterface {
        name: "processor".to_string(),
        version: "2.0.0".to_string(),
        exports: vec![func],
        imports: vec![],
        types: vec![
            InterfaceType::String,
            InterfaceType::U32,
            InterfaceType::Bool,
        ],
    };

    assert_eq!(interface.exports.len(), 1);
    assert_eq!(interface.types.len(), 3);
}

#[test]
fn test_complex_interface_types() {
    let record_type = InterfaceType::Record(vec![
        ("id".to_string(), InterfaceType::U64),
        ("name".to_string(), InterfaceType::String),
    ]);

    let list_type = InterfaceType::List(Box::new(record_type.clone()));
    let option_type = InterfaceType::Option(Box::new(list_type.clone()));

    match &option_type {
        InterfaceType::Option(inner) => match **inner {
            InterfaceType::List(_) => (),
            _ => panic!("Expected List inside Option"),
        },
        _ => panic!("Expected Option"),
    }
}

#[test]
fn test_component_lifecycle_states() {
    let states = [
        ComponentState::Initializing,
        ComponentState::Ready,
        ComponentState::Running,
        ComponentState::Terminating,
    ];

    assert_eq!(states.len(), 4);
}
