// Test NPU setup

use akida_driver::setup::NpuSetup;

fn main() -> akida_driver::Result<()> {
    tracing_subscriber::fmt::init();

    let mut setup = NpuSetup::new();
    setup.run()?;

    Ok(())
}
