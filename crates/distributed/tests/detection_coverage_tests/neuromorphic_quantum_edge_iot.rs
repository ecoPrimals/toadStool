// SPDX-License-Identifier: AGPL-3.0-or-later
use super::common::serde_json_roundtrip;
use toadstool_distributed::substrate::{EdgeIoTPlatform, NeuromorphicPlatform, QuantumPlatform};

#[test]
fn neuromorphic_platform_variants_roundtrip() {
    let samples = [
        NeuromorphicPlatform::SpikingNeuralNetwork {
            platform: "p".into(),
            neuron_model: "LIF".into(),
            synapse_model: "STDP".into(),
            neuron_count: 1,
            connectivity_pattern: "c".into(),
        },
        NeuromorphicPlatform::LiquidStateMachine {
            platform: "p".into(),
            liquid_neuron_count: 1,
            readout_neuron_count: 1,
            temporal_dynamics: "t".into(),
        },
        NeuromorphicPlatform::OpticalNeuralNetwork {
            platform: "p".into(),
            wavelength_channels: 1,
            photonic_neurons: 1,
            optical_switches: 1,
        },
        NeuromorphicPlatform::AnalogNeuralNetwork {
            platform: "p".into(),
            analog_neurons: 1,
            precision_bits: 8,
            noise_characteristics: "n".into(),
        },
    ];
    for p in samples {
        let q = serde_json_roundtrip(&p);
        assert_eq!(p, q);
    }
}

#[test]
fn quantum_platform_variants_roundtrip() {
    let samples = [
        QuantumPlatform::GateBasedQuantum {
            platform: "p".into(),
            qubit_count: 2,
            gate_fidelity: 0.9,
            connectivity_graph: "g".into(),
            error_correction: false,
        },
        QuantumPlatform::PhotonicQuantum {
            platform: "p".into(),
            photon_sources: 1,
            beam_splitters: 1,
            detectors: 1,
            squeezing_level_db: 1.0,
        },
        QuantumPlatform::TrappedIonQuantum {
            platform: "p".into(),
            ion_species: "Yb".into(),
            trap_frequency_mhz: 1.0,
            laser_cooling: true,
        },
        QuantumPlatform::SuperconductingQuantum {
            platform: "p".into(),
            qubit_type: "transmon".into(),
            operating_temperature_mk: 15.0,
            coherence_time_us: 10.0,
        },
    ];
    for p in samples {
        let q = serde_json_roundtrip(&p);
        assert_eq!(p, q);
    }
}

#[test]
fn edge_iot_platform_variants_roundtrip() {
    let samples = [
        EdgeIoTPlatform::IoTSensor {
            sensor_type: "t".into(),
            measurement_range: "r".into(),
            power_consumption_uw: 500.0,
            communication_protocol: "I2C".into(),
        },
        EdgeIoTPlatform::FPGA {
            family: "f".into(),
            logic_elements: 1,
            ram_blocks: 1,
            dsp_blocks: 1,
            io_pins: 1,
        },
        EdgeIoTPlatform::NPU {
            chip: "c".into(),
            tops_performance: 1.0,
            power_efficiency_tops_per_watt: 1.0,
            supported_frameworks: vec![],
        },
    ];
    for p in samples {
        let q = serde_json_roundtrip(&p);
        assert_eq!(p, q);
    }
}

#[test]
fn neuromorphic_memristive_and_echo_state_roundtrip() {
    let m = NeuromorphicPlatform::MemristiveComputing {
        platform: "p".into(),
        memristor_technology: "t".into(),
        crossbar_size: (2, 2),
        resistance_levels: 4,
    };
    assert_eq!(m, serde_json_roundtrip(&m));
    let e = NeuromorphicPlatform::EchoStateNetwork {
        platform: "p".into(),
        reservoir_size: 10,
        connectivity_density: 0.1,
        spectral_radius: 0.9,
        input_scaling: 1.0,
        leak_rate: 0.1,
    };
    assert_eq!(e, serde_json_roundtrip(&e));
}

#[test]
fn edge_iot_microcontroller_and_sbc_roundtrip() {
    let m = EdgeIoTPlatform::Microcontroller {
        chip: "c".into(),
        architecture: "a".into(),
        flash_kb: 1,
        ram_kb: 2,
        clock_speed_mhz: 3,
        gpio_pins: 4,
    };
    assert_eq!(m, serde_json_roundtrip(&m));
    let s = EdgeIoTPlatform::SingleBoardComputer {
        board: "b".into(),
        soc: "s".into(),
        ram_mb: 512,
        storage_type: "sd".into(),
        connectivity: vec!["GPIO".into()],
    };
    assert_eq!(s, serde_json_roundtrip(&s));
}

#[test]
fn quantum_annealing_roundtrip() {
    let p = QuantumPlatform::QuantumAnnealing {
        platform: "p".into(),
        qubit_count: 8,
        coupling_strength: 1.0,
        annealing_time_us: 5.0,
    };
    assert_eq!(p, serde_json_roundtrip(&p));
}
