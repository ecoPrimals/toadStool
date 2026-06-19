// SPDX-License-Identifier: AGPL-3.0-or-later
//! Page table encoding and population for PFIFO channels.
//!
//! Split by GPU generation:
//! - [`v2`]: Volta+ 5-level MMU (PD3→PD2→PD1→PD0→PT)
//! - [`kepler`]: GK104/GK110 2-level MMU (PD→PT)

mod kepler;
mod v2;

pub(super) use kepler::{
    populate_kepler_instance_block, populate_kepler_page_tables, populate_kepler_runlist,
};
pub(super) use v2::{
    encode_pde, encode_pte, populate_instance_block, populate_instance_block_custom,
    populate_instance_block_static, populate_page_tables, populate_page_tables_custom,
    populate_runlist, populate_runlist_static, write_vram_pte,
};

/// Write a little-endian `u32` into a byte slice at the given byte offset.
pub(super) fn write_u32_le(buf: &mut [u8], offset: usize, value: u32) {
    buf[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::kepler::*;
    use super::v2::*;
    use super::*;
    use crate::vfio::channel::registers::*;

    #[test]
    fn pde_encoding_sys_mem_coherent() {
        let pde = encode_pde(0x6000);
        // (0x6000 >> 4) | (2 << 1) | (1 << 3) = 0x600 | 0xC = 0x60C
        assert_eq!(pde, 0x60C);
        assert_eq!((pde >> 1) & 3, 2, "aperture bits[2:1] = COH(2)");
        assert_eq!((pde >> 3) & 1, 1, "VOL bit 3");
        assert_eq!((pde >> 4) & 1, 0, "bit 4 NOT set for PD3/PD2/PD1");
        let addr = (pde & !0xF) << 4;
        assert_eq!(addr, 0x6000, "GPU decode: (PDE & ~0xF) << 4");
    }

    #[test]
    fn pd0_pde_encoding_has_spt_present() {
        let pd0_pde = encode_pd0_pde(0x9000);
        // (0x9000 >> 4) | (2 << 1) | (1 << 3) | (1 << 4) = 0x900 | 0x1C = 0x91C
        assert_eq!(pd0_pde, 0x91C);
        assert_eq!((pd0_pde >> 1) & 3, 2, "aperture bits[2:1] = COH(2)");
        assert_eq!((pd0_pde >> 3) & 1, 1, "VOL bit 3");
        assert_eq!((pd0_pde >> 4) & 1, 1, "SPT_PRESENT bit 4");
        let addr = (pd0_pde & !0x1F) << 4;
        assert_eq!(addr, 0x9000, "GPU decode: (PDE & ~0x1F) << 4");
    }

    #[test]
    fn pd0_pde_vs_pde_differ_only_in_bit4() {
        let pde = encode_pde(0x9000);
        let pd0 = encode_pd0_pde(0x9000);
        assert_eq!(pd0 ^ pde, 1 << 4, "only bit 4 differs");
    }

    #[test]
    fn pte_encoding_identity_map() {
        let pte = encode_pte(0x1000);
        // (0x1000 >> 4) | 1 | (2 << 1) | (1 << 3) = 0x100 | 0xD = 0x10D
        assert_eq!(pte, 0x10D);
        assert_eq!(pte & 1, 1, "valid bit");
        assert_eq!((pte >> 1) & 3, 2, "aperture bits[2:1] = COH(2)");
        assert_eq!((pte >> 3) & 1, 1, "VOL bit 3");
        let addr = (pte & !0xF) << 4;
        assert_eq!(addr, 0x1000, "GPU decode: (PTE & ~0xF) << 4");
    }

    #[test]
    fn pte_encoding_higher_address() {
        let pte = encode_pte(0x10_0000);
        assert_eq!(pte, 0x1_000D);
        let addr = (pte & !0xF) << 4;
        assert_eq!(addr, 0x10_0000);
    }

    #[test]
    fn write_u32_le_roundtrip() {
        let mut buf = [0u8; 8];
        write_u32_le(&mut buf, 4, 0xDEAD_BEEF);
        let val = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);
        assert_eq!(val, 0xDEAD_BEEF);
    }

    #[test]
    fn ramuserd_offsets_match_nvidia_spec() {
        assert_eq!(ramuserd::GP_GET, 0x88);
        assert_eq!(ramuserd::GP_PUT, 0x8C);
    }

    #[test]
    fn pccsr_register_offsets() {
        assert_eq!(pccsr::inst(0), 0x80_0000);
        assert_eq!(pccsr::channel(0), 0x80_0004);
        assert_eq!(pccsr::inst(1), 0x80_0008);
        assert_eq!(pccsr::channel(1), 0x80_000C);
    }

    #[test]
    fn kepler_pde_encoding() {
        let pde = encode_kepler_pde(0x9000);
        assert_eq!(pde & 0x7, 2, "target bits[2:0] = SYS_COH(2)");
        assert_eq!((pde >> 4) & 1, 1, "present bit 4");
        let addr = pde & 0x000F_FFFF_FF00;
        assert_eq!(addr, 0x9000, "address preserved in bits[35:8]");
    }

    #[test]
    fn kepler_pte_encoding() {
        let pte = encode_kepler_pte(0x2000);
        assert_eq!(pte & 1, 1, "valid bit 0");
        assert_eq!((pte >> 1) & 3, 2, "target bits[2:1] = SYS_COH(2)");
        assert_eq!((pte >> 3) & 1, 1, "VOL bit 3");
        let addr = pte & 0x000F_FFFF_FFF0;
        assert_eq!(addr, 0x2000, "address preserved in bits[35:4]");
    }

    #[test]
    fn kepler_runlist_entry_format() {
        let mut rl = [0u8; 8];
        populate_kepler_runlist(&mut rl, 0x3000, 0);
        let dw0 = u32::from_le_bytes([rl[0], rl[1], rl[2], rl[3]]);
        let dw1 = u32::from_le_bytes([rl[4], rl[5], rl[6], rl[7]]);
        assert_eq!(dw0, 0, "channel_id = 0");
        assert_eq!(dw1, 4, "entry_type = channel (0x04)");

        let mut rl2 = [0u8; 8];
        populate_kepler_runlist(&mut rl2, 0x3000, 7);
        let dw0_2 = u32::from_le_bytes([rl2[0], rl2[1], rl2[2], rl2[3]]);
        assert_eq!(dw0_2, 7, "channel_id = 7");
    }

    #[test]
    fn gk104_runlist_base_value_encoding() {
        let val = pfifo::gk104_runlist_base_value(RUNLIST_IOVA, TARGET_SYS_MEM_COHERENT);
        assert_eq!(val, (0x4000 >> 12) | 2, "RUNLIST_IOVA >> 12 | target=COH");
        assert_eq!(val & 3, 2, "target bits[1:0] = SYS_MEM_COH");
        assert_eq!((val >> 2) << 14, RUNLIST_IOVA as u32, "addr roundtrip");
    }

    #[test]
    fn gk104_runlist_submit_value_encoding() {
        let val = pfifo::gk104_runlist_submit_value(1, 1);
        assert_eq!(val, (1 << 20) | 1, "runlist_id=1, count=1");
        assert_eq!((val >> 20) & 0xFFF, 1, "runlist_id field");
        assert_eq!(val & 0xFFFFF, 1, "entry_count field");
    }

    #[test]
    fn kepler_doorbell_offsets() {
        assert_eq!(
            usermode::gk104_doorbell(0),
            0x3000,
            "ch0 doorbell at 0x3000"
        );
        assert_eq!(
            usermode::gk104_doorbell(1),
            0x3008,
            "ch1 doorbell at 0x3008"
        );
        assert_eq!(
            usermode::gk104_doorbell(127),
            0x3000 + 127 * 8,
            "ch127 doorbell"
        );
    }

    #[test]
    fn kepler_instance_block_ramfc_golden() {
        let mut inst = [0u8; 4096];
        let gpfifo_iova: u64 = 0xC000;
        let gpfifo_entries: u32 = 512;
        let userd_iova: u64 = 0x2000;
        let channel_id: u32 = 0;
        let pd_iova: u64 = PD3_IOVA;

        populate_kepler_instance_block(
            &mut inst,
            gpfifo_iova,
            gpfifo_entries,
            userd_iova,
            channel_id,
            pd_iova,
        );

        let rd = |off: usize| u32::from_le_bytes(inst[off..off + 4].try_into().unwrap());

        // USERD_LO: addr masked | target=COH(2)
        assert_eq!(rd(0x008) & 3, 2, "USERD target = SYS_MEM_COH");
        assert_eq!(rd(0x008) & 0xFFFF_FE00, 0x2000, "USERD addr");
        assert_eq!(rd(0x00C), 0, "USERD_HI = 0 (32-bit IOVA)");

        // SIGNATURE
        assert_eq!(rd(0x010), 0x0000_FACE, "RAMFC signature");

        // ACQUIRE
        assert_eq!(rd(0x030), 0x7FFF_F902, "semaphore acquire config");

        // DMA_LIMIT_REF (0x3C) — the nv50 field that was missing
        assert_eq!(rd(0x03C), 0x003F_6078, "DMA limit/ref from nv50");

        // PB_DMA_SUBROUTINE (0x44) — the nv50 field that was missing
        assert_eq!(rd(0x044), 0x0100_3FFF, "PB DMA subroutine from nv50");

        // GP_BASE
        assert_eq!(rd(0x048), gpfifo_iova as u32, "GP_BASE_LO");
        let limit2 = gpfifo_entries.ilog2();
        assert_eq!(rd(0x04C), limit2 << 16, "GP_BASE_HI has limit");

        // GP_PUT/GET/FETCH all zero
        assert_eq!(rd(0x054), 0, "GP_PUT = 0");
        assert_eq!(rd(0x058), 0, "GP_GET = 0");
        assert_eq!(rd(0x050), 0, "GP_FETCH = 0");

        // PB_HEADER
        assert_eq!(rd(0x084), 0x2040_0000, "PB_HEADER");

        // SUBDEVICE
        assert_eq!(rd(0x094), 0x3000_0FFF, "SUBDEVICE mask");

        // CONFIG (Kepler-specific)
        assert_eq!(rd(0x0A8), 0x0000_0400, "CONFIG = 0x400 (Kepler)");

        // CHANNEL_INFO
        assert_eq!(rd(0x0AC), 0x0300_0000 | channel_id, "CHANNEL_INFO");

        // RAMIN PDB — V1 format
        let pdb_lo = rd(0x200);
        assert_eq!(pdb_lo & 3, 2, "PDB target = SYS_MEM_COH");
        assert_eq!((pdb_lo >> 2) & 1, 1, "PDB VOL = 1");
        let pdb_addr = (pdb_lo & 0xFFFF_F000) as u64;
        assert_eq!(pdb_addr, pd_iova, "PDB address = PD3_IOVA");

        // VA limit — 40-bit (1 TB)
        assert_eq!(rd(0x208), 0xFFFF_FFFF, "ADDR_LIMIT_LO");
        assert_eq!(rd(0x20C), 0x0000_00FF, "ADDR_LIMIT_HI (40-bit)");
    }

    #[test]
    fn iova_layout_non_overlapping() {
        let iovas = [
            ("INSTANCE", INSTANCE_IOVA),
            ("RUNLIST", RUNLIST_IOVA),
            ("PD3", PD3_IOVA),
            ("PD2", PD2_IOVA),
            ("PD1", PD1_IOVA),
            ("PD0", PD0_IOVA),
            ("PT0", PT0_IOVA),
        ];
        for i in 0..iovas.len() {
            for j in (i + 1)..iovas.len() {
                assert_ne!(
                    iovas[i].1, iovas[j].1,
                    "{} and {} overlap at {:#x}",
                    iovas[i].0, iovas[j].0, iovas[i].1
                );
            }
        }
    }

    #[test]
    fn iova_layout_after_userd() {
        const { assert!(INSTANCE_IOVA > 0x2000, "instance after USERD") };
        const {
            assert!(
                PT0_IOVA + 4096 <= 0x10_0000,
                "page tables before USER_IOVA_BASE"
            )
        };
    }

    #[test]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "IOVA addresses are intentionally truncated to 32-bit hardware register fields"
    )]
    fn pccsr_inst_value_channel_zero() {
        let value = (INSTANCE_IOVA >> 12) as u32 | pccsr::INST_TARGET_SYS_MEM_NCOH;
        assert_eq!(value & 0x0FFF_FFFF, 3, "INST_PTR = 3 (0x3000 >> 12)");
        assert_eq!((value >> 28) & 3, 3, "target = SYS_MEM_NCOH");
        assert_eq!((value >> 31) & 1, 0, "BIND not set — implicit via runlist");
    }

    #[test]
    fn runlist_gv100_register_addresses() {
        assert_eq!(pfifo::runlist_base(0), 0x2270, "RL0 base");
        assert_eq!(pfifo::runlist_submit(0), 0x2274, "RL0 submit");
        assert_eq!(pfifo::runlist_base(1), 0x2280, "RL1 base");
        assert_eq!(pfifo::runlist_submit(1), 0x2284, "RL1 submit");
        assert_eq!(pfifo::runlist_base(2), 0x2290, "RL2 base");
    }

    #[test]
    fn runlist_gv100_value_encoding() {
        let base = pfifo::gv100_runlist_base_value(RUNLIST_IOVA);
        assert_eq!(base, 4, "lower_32(0x4000 >> 12) = 4");
        let submit = pfifo::gv100_runlist_submit_value(RUNLIST_IOVA, 2);
        assert_eq!(submit, 2 << 16, "upper_32(0x4000>>12)=0, count=2<<16");
        assert_eq!((submit >> 16) & 0xFFFF, 2, "entry count");
    }

    #[test]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "IOVA addresses truncated to 32-bit registers"
    )]
    fn runlist_chan_entry_encoding() {
        let userd: u64 = 0x2000;
        let dw0 = userd as u32 | (TARGET_SYS_MEM_COHERENT << 2);
        assert_eq!(dw0, 0x2008, "USERD=0x2000, target=COH(2), runq=0");
        assert_eq!((dw0 >> 2) & 3, 2, "USERD_TARGET = SYS_MEM_COH");
        assert_eq!(dw0 & 1, 0, "TYPE = 0 (channel)");

        let dw0_runq1 = userd as u32 | (TARGET_SYS_MEM_COHERENT << 2) | (1 << 1);
        assert_eq!(dw0_runq1, 0x200A, "USERD=0x2000, target=COH(2), runq=1");
        assert_eq!((dw0_runq1 >> 1) & 1, 1, "RUNQUEUE = 1");

        let inst: u64 = 0x3000;
        let chid: u32 = 0;
        let dw2 = inst as u32 | (TARGET_SYS_MEM_NONCOHERENT << 4) | chid;
        assert_eq!(dw2, 0x3030, "INST=0x3000, target=NCOH(3), chid=0");
        assert_eq!((dw2 >> 4) & 3, 3, "INST_TARGET = SYS_MEM_NCOH");
    }
}
