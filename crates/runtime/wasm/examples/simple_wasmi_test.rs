//! Simple wasmi test to understand the API

use wasmi::{Engine, Linker, Module, Store};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Simple add function
    let wasm_bytes = wat::parse_str(r#"
        (module
            (func (export "add") (param i32 i32) (result i32)
                local.get 0
                local.get 1
                i32.add
            )
        )
    "#)?;
    
    let engine = Engine::default();
    let module = Module::new(&engine, &wasm_bytes[..])?;
    
    // Create store with () as host data (no WASI)
    let mut store = Store::new(&engine, ());
    
    // Create linker
    let linker = <Linker<()>>::new(&engine);
    
    // Try to instantiate - what's the correct API?
    println!("Linker methods: instantiate? define? ...");
    
    Ok(())
}
