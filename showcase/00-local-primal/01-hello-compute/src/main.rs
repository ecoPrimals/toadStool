// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(deprecated)] // primals module is deprecated; we demonstrate it for showcase

use colored::Colorize;
use toadstool_common::constants::PRIMAL_NAME;
use toadstool_common::format_bytes;
use toadstool_common::interned_strings::capabilities;
use toadstool_common::interned_strings::primals;
use toadstool_sysmon::{cpu_brand, cpu_count, load_average, memory_info};

fn main() {
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
    println!("{}", "  ToadStool Showcase: Hello Compute".cyan().bold());
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
    println!();

    // Primal identity
    println!("{}", "► Primal Identity".cyan());
    println!("  Name:    {}", PRIMAL_NAME);
    println!("  Version: {}", env!("CARGO_PKG_VERSION"));
    println!();

    // Interned capabilities
    println!("{}", "► Interned Capabilities".cyan());
    let caps = [
        capabilities::SECURITY,
        capabilities::CRYPTO,
        capabilities::STORAGE,
        capabilities::COORDINATION,
        capabilities::INTELLIGENCE,
        capabilities::COMPUTE,
        capabilities::MONITORING,
        capabilities::NETWORKING,
        capabilities::ENCRYPTION,
        capabilities::SIGNING,
        capabilities::KEY_MANAGEMENT,
        capabilities::PKI,
        capabilities::AUDIT,
        capabilities::PERSISTENCE,
        capabilities::COMPRESSION,
        capabilities::VERSIONING,
    ];
    for cap in caps {
        println!("  {} {}", "✓".green(), cap);
    }
    println!();

    // Interned primal names (legacy, for demonstration)
    println!("{}", "► Interned Primal Names".cyan());
    let primals_list = [
        primals::BEARDOG,
        primals::SONGBIRD,
        primals::NESTGATE,
        primals::SQUIRREL,
        primals::TOADSTOOL,
    ];
    for p in primals_list {
        println!("  {} {}", "✓".green(), p);
    }
    println!();

    // CPU info
    println!("{}", "► CPU Info".cyan());
    println!("  Cores: {}", cpu_count());
    match cpu_brand() {
        Ok(brand) => println!("  Brand:  {}", brand),
        Err(e) => println!("  Brand:  (error: {})", e),
    }
    println!();

    // Memory info
    println!("{}", "► Memory Info".cyan());
    match memory_info() {
        Ok(mem) => {
            println!("  Total:     {}", format_bytes(mem.total));
            println!("  Available: {}", format_bytes(mem.available));
            println!("  Used:      {}", format_bytes(mem.used));
        }
        Err(e) => println!("  (error: {})", e),
    }
    println!();

    // Load average
    println!("{}", "► Load Average".cyan());
    match load_average() {
        Ok(la) => {
            println!("  1 min:  {:.2}", la.one);
            println!("  5 min:  {:.2}", la.five);
            println!("  15 min: {:.2}", la.fifteen);
        }
        Err(e) => println!("  (error: {})", e),
    }
    println!();

    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
    println!("  {} toadStool is ready for compute orchestration", "✓".green());
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
}
