// SPDX-License-Identifier: AGPL-3.0-or-later
//! Fuzz [`toadstool_runtime_gpu::unified_memory::UnifiedBuffer`] CPU slice views (`NonNull::as_mut` /
//! `as_ref` in `buffer/access.rs`) via the CPU unified-memory backend (no GPU hardware).
#![no_main]

use libfuzzer_sys::fuzz_target;
use tokio::runtime::Builder;
use toadstool_runtime_gpu::unified_memory::{
    BackendStrategy, BackendType, UniversalUnifiedMemory,
};

fn usize_from_bytes(data: &[u8]) -> usize {
    data.iter().take(8).fold(0usize, |acc, &b| {
        acc.wrapping_shl(8) | usize::from(b)
    })
}

fuzz_target!(|data: &[u8]| {
    let rt = Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(async {
        let Ok(memory) = UniversalUnifiedMemory::with_strategy(BackendStrategy::Specific(
            BackendType::Cpu,
        ))
        .await
        else {
            return;
        };

        let u = usize_from_bytes(data);
        // Exercise invalid and edge sizes (allocator rejects 0 / oversize before slice construction).
        let candidates = [
            0usize,
            1usize,
            data.len().max(1),
            u % 4097,
            u % (1024 * 1024 + 1),
            u % (4 * 1024 * 1024 * 1024 + 7),
        ];

        for size in candidates {
            let Ok(mut buf) = memory.allocate(size).await else {
                continue;
            };

            let _ = buf.fuzz_exercise_cpu_slice_views();

            let sz = buf.size();
            if sz == 0 {
                continue;
            }

            let off = u % sz;
            let avail = sz.saturating_sub(off);
            let write_len = (data.len() % 256).min(avail).max(1).min(avail);
            let chunk: Vec<u8> = data.iter().cycle().take(write_len).copied().collect();
            let _ = buf.write_async(off, &chunk).await;

            let read_len = (u % (avail + 1)).min(avail);
            let _ = buf.read_async(off, read_len).await;
        }
    });
});
