//! Comprehensive tests for WASM component model types

use toadstool_runtime_wasm::*;

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
fn test_component_model_config_enabled() {
    let config = ComponentModelConfig::default();

    assert!(config.enabled);
}

#[test]
fn test_component_model_config_max_instances() {
    let config = ComponentModelConfig::default();

    assert_eq!(config.max_instances, 1000);
}

#[test]
fn test_component_model_config_linking_timeout() {
    let config = ComponentModelConfig::default();

    assert_eq!(config.linking_timeout_ms, 5000);
}

#[test]
fn test_component_model_config_composition() {
    let config = ComponentModelConfig::default();

    assert!(config.composition_enabled);
}

#[test]
fn test_component_model_config_wit_support() {
    let config = ComponentModelConfig::default();

    assert!(config.wit_support);
}

#[test]
fn test_component_model_config_clone() {
    let config1 = ComponentModelConfig::default();
    let config2 = config1.clone();

    assert_eq!(config1.enabled, config2.enabled);
    assert_eq!(config1.max_instances, config2.max_instances);
}

#[test]
fn test_component_model_config_serialization() {
    let config = ComponentModelConfig::default();
    let serialized = serde_json::to_string(&config).unwrap();

    assert!(!serialized.is_empty());
}

// ============================================================================
// ComponentState Tests
// ============================================================================

#[test]
fn test_component_state_initializing() {
    let state = ComponentState::Initializing;
    assert!(matches!(state, ComponentState::Initializing));
}

#[test]
fn test_component_state_ready() {
    let state = ComponentState::Ready;
    assert!(matches!(state, ComponentState::Ready));
}

#[test]
fn test_component_state_running() {
    let state = ComponentState::Running;
    assert!(matches!(state, ComponentState::Running));
}

#[test]
fn test_component_state_failed() {
    let state = ComponentState::Failed {
        error: "test error".to_string(),
    };

    match state {
        ComponentState::Failed { error } => {
            assert_eq!(error, "test error");
        }
        _ => panic!("Expected Failed state"),
    }
}

#[test]
fn test_component_state_terminating() {
    let state = ComponentState::Terminating;
    assert!(matches!(state, ComponentState::Terminating));
}

#[test]
fn test_component_state_clone() {
    let state1 = ComponentState::Ready;
    let state2 = state1.clone();

    match (state1, state2) {
        (ComponentState::Ready, ComponentState::Ready) => {
            // Clone successful
        }
        _ => panic!("Clone failed"),
    }
}

#[test]
fn test_component_state_serialization() {
    let state = ComponentState::Ready;
    let serialized = serde_json::to_string(&state).unwrap();

    assert!(!serialized.is_empty());
}

#[test]
fn test_all_component_states() {
    let states = [
        ComponentState::Initializing,
        ComponentState::Ready,
        ComponentState::Running,
        ComponentState::Failed {
            error: "error".to_string(),
        },
        ComponentState::Terminating,
    ];

    assert_eq!(states.len(), 5);
}

// ============================================================================
// InterfaceType Tests
// ============================================================================

#[test]
fn test_interface_type_bool() {
    let itype = InterfaceType::Bool;
    assert!(matches!(itype, InterfaceType::Bool));
}

#[test]
fn test_interface_type_u8() {
    let itype = InterfaceType::U8;
    assert!(matches!(itype, InterfaceType::U8));
}

#[test]
fn test_interface_type_u16() {
    let itype = InterfaceType::U16;
    assert!(matches!(itype, InterfaceType::U16));
}

#[test]
fn test_interface_type_u32() {
    let itype = InterfaceType::U32;
    assert!(matches!(itype, InterfaceType::U32));
}

#[test]
fn test_interface_type_u64() {
    let itype = InterfaceType::U64;
    assert!(matches!(itype, InterfaceType::U64));
}

#[test]
fn test_interface_type_s8() {
    let itype = InterfaceType::S8;
    assert!(matches!(itype, InterfaceType::S8));
}

#[test]
fn test_interface_type_s16() {
    let itype = InterfaceType::S16;
    assert!(matches!(itype, InterfaceType::S16));
}

#[test]
fn test_interface_type_s32() {
    let itype = InterfaceType::S32;
    assert!(matches!(itype, InterfaceType::S32));
}

#[test]
fn test_interface_type_s64() {
    let itype = InterfaceType::S64;
    assert!(matches!(itype, InterfaceType::S64));
}

#[test]
fn test_interface_type_f32() {
    let itype = InterfaceType::F32;
    assert!(matches!(itype, InterfaceType::F32));
}

#[test]
fn test_interface_type_f64() {
    let itype = InterfaceType::F64;
    assert!(matches!(itype, InterfaceType::F64));
}

#[test]
fn test_interface_type_string() {
    let itype = InterfaceType::String;
    assert!(matches!(itype, InterfaceType::String));
}

#[test]
fn test_interface_type_list() {
    let itype = InterfaceType::List(Box::new(InterfaceType::U32));

    match itype {
        InterfaceType::List(inner) => {
            assert!(matches!(*inner, InterfaceType::U32));
        }
        _ => panic!("Expected List type"),
    }
}

#[test]
fn test_interface_type_option() {
    let itype = InterfaceType::Option(Box::new(InterfaceType::String));

    match itype {
        InterfaceType::Option(inner) => {
            assert!(matches!(*inner, InterfaceType::String));
        }
        _ => panic!("Expected Option type"),
    }
}

#[test]
fn test_interface_type_result() {
    let itype = InterfaceType::Result(
        Box::new(InterfaceType::U32),
        Box::new(InterfaceType::String),
    );

    match itype {
        InterfaceType::Result(ok, err) => {
            assert!(matches!(*ok, InterfaceType::U32));
            assert!(matches!(*err, InterfaceType::String));
        }
        _ => panic!("Expected Result type"),
    }
}

#[test]
fn test_interface_type_custom() {
    let itype = InterfaceType::Custom("MyType".to_string());

    match itype {
        InterfaceType::Custom(name) => {
            assert_eq!(name, "MyType");
        }
        _ => panic!("Expected Custom type"),
    }
}

#[test]
fn test_interface_type_clone() {
    let itype1 = InterfaceType::Bool;
    let itype2 = itype1.clone();

    match (itype1, itype2) {
        (InterfaceType::Bool, InterfaceType::Bool) => {
            // Clone successful
        }
        _ => panic!("Clone failed"),
    }
}

#[test]
fn test_interface_type_serialization() {
    let itype = InterfaceType::U32;
    let serialized = serde_json::to_string(&itype).unwrap();

    assert!(!serialized.is_empty());
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
fn test_component_resource_usage_clone() {
    let usage1 = ComponentResourceUsage {
        memory_bytes: 1024,
        cpu_time_ms: 100,
        function_calls: 50,
        imports_count: 5,
        exports_count: 10,
    };

    let usage2 = usage1.clone();

    assert_eq!(usage1.memory_bytes, usage2.memory_bytes);
    assert_eq!(usage1.cpu_time_ms, usage2.cpu_time_ms);
    assert_eq!(usage1.function_calls, usage2.function_calls);
}

// ============================================================================
// InterfaceFunction Tests
// ============================================================================

#[test]
fn test_interface_function_creation() {
    let func = InterfaceFunction {
        name: "add".to_string(),
        params: vec![InterfaceType::U32, InterfaceType::U32],
        return_type: Some(InterfaceType::U32),
        docs: Some("Add two numbers".to_string()),
    };

    assert_eq!(func.name, "add");
    assert_eq!(func.params.len(), 2);
    assert!(func.return_type.is_some());
    assert!(func.docs.is_some());
}

#[test]
fn test_interface_function_no_return() {
    let func = InterfaceFunction {
        name: "log".to_string(),
        params: vec![InterfaceType::String],
        return_type: None,
        docs: None,
    };

    assert!(func.return_type.is_none());
    assert!(func.docs.is_none());
}

#[test]
fn test_interface_function_clone() {
    let func1 = InterfaceFunction {
        name: "test".to_string(),
        params: vec![],
        return_type: None,
        docs: None,
    };

    let func2 = func1.clone();

    assert_eq!(func1.name, func2.name);
}

#[test]
fn test_interface_function_serialization() {
    let func = InterfaceFunction {
        name: "test".to_string(),
        params: vec![],
        return_type: None,
        docs: None,
    };

    let serialized = serde_json::to_string(&func).unwrap();
    assert!(!serialized.is_empty());
}

// ============================================================================
// ComponentInterface Tests
// ============================================================================

#[test]
fn test_component_interface_creation() {
    let interface = ComponentInterface {
        name: "calculator".to_string(),
        version: "1.0.0".to_string(),
        exports: vec![],
        imports: vec![],
        types: vec![],
    };

    assert_eq!(interface.name, "calculator");
    assert_eq!(interface.version, "1.0.0");
}

#[test]
fn test_component_interface_clone() {
    let interface1 = ComponentInterface {
        name: "test".to_string(),
        version: "0.1.0".to_string(),
        exports: vec![],
        imports: vec![],
        types: vec![],
    };

    let interface2 = interface1.clone();

    assert_eq!(interface1.name, interface2.name);
    assert_eq!(interface1.version, interface2.version);
}

#[test]
fn test_component_interface_serialization() {
    let interface = ComponentInterface {
        name: "test".to_string(),
        version: "1.0.0".to_string(),
        exports: vec![],
        imports: vec![],
        types: vec![],
    };

    let serialized = serde_json::to_string(&interface).unwrap();
    assert!(!serialized.is_empty());
}
