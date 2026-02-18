//! Debug PPPM CPU vs GPU - run: cargo run -p barracuda --no-default-features --example pppm_debug

use barracuda::ops::md::electrostatics::{
    compute_short_range, dipole_correction, self_energy_correction, spread_charges_with_coeffs,
    GreensFunction, Pppm, PppmCpuFft, PppmParams,
};

fn main() {
    let params = PppmParams::custom(2, [10.0, 10.0, 10.0], [8, 8, 8], 2.0, 3.0, 4);
    let positions = vec![[4.0, 5.0, 5.0], [6.0, 5.0, 5.0]];
    let charges = vec![1.0, -1.0];

    let pppm = Pppm::new(params.clone());
    let (forces, energy) = pppm.compute(&positions, &charges).unwrap();

    let (_e_short_forces, e_short) = compute_short_range(&positions, &charges, &params);
    let e_self = self_energy_correction(&charges, params.alpha, params.coulomb_constant);
    let e_dipole = dipole_correction(&positions, &charges, params.box_dims, params.coulomb_constant);

    let (charge_mesh, _) = spread_charges_with_coeffs(&positions, &charges, &params);
    let rho_k = PppmCpuFft::forward_3d(&charge_mesh.values, 8, 8, 8);
    let greens = GreensFunction::new(&params);
    let e_kspace = greens.kspace_energy(&rho_k, 1000.0);

    println!("CPU PPPM (opposite charges):");
    println!("  e_kspace={} e_short={} e_self={} e_dipole={}", e_kspace, e_short, e_self, e_dipole);
    println!("  energy = {} (sum = {})", energy, e_kspace + e_short + e_self + e_dipole);
    println!("  forces[0] = {:?}", forces[0]);
    println!("  forces[1] = {:?}", forces[1]);
}
