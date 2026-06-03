// SPDX-License-Identifier: AGPL-3.0-or-later
//! Strategy-driven replay of captured GR init sequences to ungate engines.

use crate::nv::pri::is_pri_fault;
use crate::vfio::device::MappedBar;

/// PGRAPH status register (GK110+).
pub(crate) const PGRAPH_STATUS: usize = 0x0040_0700;

/// Replay a [`GrInitSequence`] for a named engine and optionally validate status.
///
/// `status_reg` is an optional BAR0 offset; PRI-faulted read-back means failure.
pub(crate) fn engine_ungate(
    bar0: &MappedBar,
    seq: &crate::nv::gr_init::GrInitSequence,
    engine_name: &str,
    status_reg: Option<usize>,
) -> Result<String, String> {
    let applied = seq.apply(bar0)?;

    if let Some(reg) = status_reg {
        let status = bar0.read_u32(reg).unwrap_or(0xDEAD_DEAD);
        if is_pri_fault(status) {
            return Err(format!(
                "{engine_name} still gated after {applied} writes (status=0x{status:08x})"
            ));
        }
        Ok(format!(
            "{engine_name} ungated: {applied} writes applied, status=0x{status:08x}"
        ))
    } else {
        Ok(format!("{engine_name} ungated: {applied} writes applied"))
    }
}
