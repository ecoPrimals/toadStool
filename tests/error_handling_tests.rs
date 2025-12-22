//! Error handling and recovery tests
//!
//! Tests error propagation, error recovery, and graceful degradation.

use toadstool::{ToadStoolError, ToadStoolResult};

#[test]
fn test_error_creation() {
    let error = ToadStoolError::runtime("test error");
    assert!(matches!(error, ToadStoolError::Runtime(_)));
}

#[test]
fn test_error_context() {
    let error = ToadStoolError::execution("failed to execute");
    let error_msg = error.to_string();
    assert!(error_msg.contains("failed to execute"));
}

#[test]
fn test_error_propagation() {
    fn returns_error() -> ToadStoolResult<()> {
        Err(ToadStoolError::runtime("propagated error"))
    }
    
    let result = returns_error();
    assert!(result.is_err());
}

#[test]
fn test_error_types() {
    let errors = vec![
        ToadStoolError::runtime("runtime error"),
        ToadStoolError::execution("execution error"),
        ToadStoolError::not_found("not found"),
        ToadStoolError::validation("validation error"),
    ];
    
    for error in errors {
        assert!(!error.to_string().is_empty());
    }
}

#[test]
fn test_result_ok_handling() {
    fn returns_ok() -> ToadStoolResult<String> {
        Ok("success".to_string())
    }
    
    let result = returns_ok();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "success");
}

#[test]
fn test_result_error_handling() {
    fn returns_err() -> ToadStoolResult<String> {
        Err(ToadStoolError::runtime("failed"))
    }
    
    let result = returns_err();
    assert!(result.is_err());
}

#[test]
fn test_error_recovery() {
    fn fallible_operation() -> ToadStoolResult<i32> {
        Err(ToadStoolError::runtime("operation failed"))
    }
    
    let result = fallible_operation().unwrap_or(42);
    assert_eq!(result, 42);
}

#[test]
fn test_error_chaining() {
    fn inner_operation() -> ToadStoolResult<()> {
        Err(ToadStoolError::runtime("inner error"))
    }
    
    fn outer_operation() -> ToadStoolResult<()> {
        inner_operation()?;
        Ok(())
    }
    
    let result = outer_operation();
    assert!(result.is_err());
}

#[test]
fn test_error_matching() {
    let error = ToadStoolError::not_found("resource missing");
    
    match error {
        ToadStoolError::NotFound(_) => {
            // Expected
        }
        _ => panic!("Wrong error type"),
    }
}

#[test]
fn test_multiple_error_sources() {
    fn operation_a() -> ToadStoolResult<()> {
        Err(ToadStoolError::runtime("A failed"))
    }
    
    fn operation_b() -> ToadStoolResult<()> {
        Err(ToadStoolError::execution("B failed"))
    }
    
    let result_a = operation_a();
    let result_b = operation_b();
    
    assert!(result_a.is_err());
    assert!(result_b.is_err());
}

