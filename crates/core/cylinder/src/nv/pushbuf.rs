// SPDX-License-Identifier: AGPL-3.0-or-later
//! Push buffer construction for NVIDIA GPU command submission.
//!
//! Provides helpers that build small inline push buffers for compute init,
//! per-dispatch, and GR context init workloads. Each push buffer is a
//! sequence of `u32` words containing method headers and data.

/// Push buffer builder.
///
/// Wraps a growable `Vec<u32>` of GPU method+data words.
pub struct PushBuf {
    data: Vec<u32>,
}

impl PushBuf {
    /// Create an empty push buffer.
    #[must_use]
    pub fn new() -> Self {
        Self {
            data: Vec::with_capacity(64),
        }
    }

    /// Encode and push a single-word method write.
    ///
    /// Uses the SEC_OP=1 (INC_METHOD) encoding with count=1:
    /// - Bits [31:29] = 001 (INC_METHOD)
    /// - Bits [28:16] = count (1)
    /// - Bits [15:13] = subchannel
    /// - Bits [12:0]  = method address >> 2
    pub fn push_1(&mut self, subchannel: u32, method_addr: u32, value: u32) {
        let hdr =
            (1u32 << 29) | (1 << 16) | ((subchannel & 0x7) << 13) | ((method_addr >> 2) & 0x1FFF);
        self.data.push(hdr);
        self.data.push(value);
    }

    /// Append another push buffer's words to this one.
    pub fn append(&mut self, other: &PushBuf) {
        self.data.extend_from_slice(&other.data);
    }

    /// Return the push buffer data as a slice of words.
    #[must_use]
    pub fn as_words(&self) -> &[u32] {
        &self.data
    }

    /// Return the push buffer data as a byte slice.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.data)
    }
}

/// Compute class identifiers.
pub mod class {
    /// Volta compute class.
    pub const VOLTA_COMPUTE_A: u32 = 0xC3C0;
    /// Turing compute class.
    pub const TURING_COMPUTE_A: u32 = 0xC5C0;
    /// Ampere compute class.
    pub const AMPERE_COMPUTE_A: u32 = 0xC6C0;
}

/// Copy Engine (CE / DMA_COPY) class identifiers and method offsets.
pub mod ce {
    /// Volta DMA copy class (VOLTA_DMA_COPY_A).
    pub const VOLTA_DMA_COPY_A: u32 = 0xC3B5;

    /// Method offsets for the DMA copy engine class.
    pub mod method {
        pub const SET_OBJECT: u32 = 0x0000;
        /// Source address upper 8 bits.
        pub const OFFSET_IN_UPPER: u32 = 0x0400;
        /// Source address lower 32 bits.
        pub const OFFSET_IN_LOWER: u32 = 0x0404;
        /// Dest address upper 8 bits.
        pub const OFFSET_OUT_UPPER: u32 = 0x0408;
        /// Dest address lower 32 bits.
        pub const OFFSET_OUT_LOWER: u32 = 0x040C;
        /// Pitch-in (bytes per row, source).
        pub const PITCH_IN: u32 = 0x0410;
        /// Pitch-out (bytes per row, dest).
        pub const PITCH_OUT: u32 = 0x0414;
        /// Line length in bytes.
        pub const LINE_LENGTH_IN: u32 = 0x0418;
        /// Line count.
        pub const LINE_COUNT: u32 = 0x041C;
        /// Launch DMA transfer.
        /// Bits: [1:0] = data_transfer_type (0=NONE, 1=PIPELINED, 2=NON_PIPELINED)
        ///       [2]   = flush_enable
        ///       [8]   = src_memory_layout (0=BLOCKLINEAR, 1=PITCH)
        ///       [12]  = dst_memory_layout (0=BLOCKLINEAR, 1=PITCH)
        ///       [20]  = src_type (0=VIRTUAL, 1=PHYSICAL)
        ///       [24]  = dst_type (0=VIRTUAL, 1=PHYSICAL)
        pub const LAUNCH_DMA: u32 = 0x0300;
        /// LAUNCH_DMA value: pipelined, pitch src+dst, virtual addressing.
        pub const LAUNCH_PIPELINED_PITCH: u32 = 0x0000_1101;
        /// Semaphore address upper (CE semaphore, not compute).
        pub const SET_SEMAPHORE_A: u32 = 0x0240;
        /// Semaphore address lower.
        pub const SET_SEMAPHORE_B: u32 = 0x0244;
        /// Semaphore payload.
        pub const SET_SEMAPHORE_PAYLOAD: u32 = 0x0248;
        /// Semaphore control: bit [0] = release after copy.
        pub const SEMAPHORE_CTRL: u32 = 0x024C;
    }
}

/// Method offsets and constants for NVIDIA compute class push buffers.
pub mod method {
    /// Bind an engine class to a subchannel.
    pub const SET_OBJECT: u32 = 0x0000;
    /// Cache invalidation method (common across compute classes).
    pub const INVALIDATE_SHADER_CACHES: u32 = 0x021C;
    /// Data value: invalidate both instruction and data caches.
    pub const INVALIDATE_INSTR_AND_DATA: u32 = 0x13;
    /// Set shared memory window base (upper bits).
    pub const SET_SHADER_SHARED_MEMORY_WINDOW_A: u32 = 0x02A0;
    /// Set shared memory window base (lower 32 bits).
    pub const SET_SHADER_SHARED_MEMORY_WINDOW_B: u32 = 0x02A4;
    /// SLM non-throttled per-TPC limit (upper 8 bits).
    pub const SET_SHADER_LOCAL_MEMORY_NON_THROTTLED_A: u32 = 0x02E4;
    /// SLM non-throttled per-TPC limit (lower 32 bits).
    pub const SET_SHADER_LOCAL_MEMORY_NON_THROTTLED_B: u32 = 0x02E8;
    /// SLM non-throttled max SM count (9 bits).
    pub const SET_SHADER_LOCAL_MEMORY_NON_THROTTLED_C: u32 = 0x02EC;
    /// SLM base GPU VA (upper 8 bits).
    pub const SET_SHADER_LOCAL_MEMORY_A: u32 = 0x0790;
    /// SLM base GPU VA (lower 32 bits).
    pub const SET_SHADER_LOCAL_MEMORY_B: u32 = 0x0794;
    /// Set local memory window base (upper 17 bits).
    pub const SET_SHADER_LOCAL_MEMORY_WINDOW_A: u32 = 0x07B0;
    /// Set local memory window base (lower 32 bits).
    pub const SET_SHADER_LOCAL_MEMORY_WINDOW_B: u32 = 0x07B4;
    /// Launch compute: QMD address >> 8 (256-byte aligned).
    pub const SEND_PCAS_A: u32 = 0x02B4;
    /// Launch compute trigger (<= Turing): bit 0 = invalidate, bit 1 = schedule.
    pub const SEND_SIGNALING_PCAS_B: u32 = 0x02BC;
    /// Launch compute trigger (Ampere+): enum, see `PCAS_ACTION_*`.
    pub const SEND_SIGNALING_PCAS2_B: u32 = 0x02C0;
    /// Turing compute class value — used to determine launch path.
    pub const TURING_COMPUTE_A: u32 = 0xC5C0;
    /// PCAS2 action (Ampere+): invalidate QMD cache, copy from memory, schedule.
    /// 4-bit field [3:0] per clcec0.h NVCEC0_SEND_SIGNALING_PCAS2_B_PCAS_ACTION.
    pub const PCAS_ACTION_INVALIDATE_COPY_SCHEDULE: u32 = 3;

    /// Semaphore address upper 8 bits (method 0x06C0).
    pub const SEMAPHORE_ADDR_UPPER: u32 = 0x06C0;
    /// Semaphore address lower 32 bits (method 0x06C4).
    pub const SEMAPHORE_ADDR_LOWER: u32 = 0x06C4;
    /// Semaphore payload value to write on release (method 0x06C8).
    pub const SEMAPHORE_PAYLOAD: u32 = 0x06C8;
    /// Semaphore control: operation mode (method 0x06CC).
    /// Bit [0] = RELEASE (write payload to addr), Bit [2:1] = ACQUIRE mode.
    pub const SEMAPHORE_CTRL: u32 = 0x06CC;
    /// Semaphore control value: release (write payload, no acquire).
    pub const SEMAPHORE_CTRL_RELEASE: u32 = 0x1;
}

impl PushBuf {
    /// Build a one-time compute init push buffer.
    ///
    /// When `skip_set_object` is false (pre-Blackwell / nouveau), pushes
    /// `SET_OBJECT` on subchannel 0 to program the PBDMA subchannel table.
    ///
    /// When `skip_set_object` is true (Blackwell proprietary), the subchannel
    /// is already bound via RM control `NV906F_CTRL_CMD_BIND` (0x906f0101).
    /// Pushing `SET_OBJECT` again causes Xid 13 "Class Mismatch".
    #[must_use]
    pub fn compute_init(
        compute_class: u32,
        _local_mem_window: u64,
        slm_base_addr: u64,
        slm_per_tpc_bytes: u64,
    ) -> Self {
        Self::compute_init_inner(
            compute_class,
            _local_mem_window,
            slm_base_addr,
            slm_per_tpc_bytes,
            false,
        )
    }

    /// Like [`compute_init`](Self::compute_init) but skips the push buffer
    /// `SET_OBJECT`, relying on the RM bind for subchannel setup.
    #[must_use]
    pub fn compute_init_rm_bound(
        _local_mem_window: u64,
        slm_base_addr: u64,
        slm_per_tpc_bytes: u64,
    ) -> Self {
        Self::compute_init_inner(0, _local_mem_window, slm_base_addr, slm_per_tpc_bytes, true)
    }

    /// Blackwell compute init on a specific subchannel with memory windows.
    #[must_use]
    pub fn compute_init_subchannel(
        compute_class: u32,
        local_mem_window: u64,
        shared_mem_window: u64,
        slm_base_addr: u64,
        slm_per_tpc_bytes: u64,
        subchannel: u32,
    ) -> Self {
        let mut pb = Self::compute_init_on_subchannel(
            compute_class,
            local_mem_window,
            slm_base_addr,
            slm_per_tpc_bytes,
            false,
            subchannel,
        );
        if shared_mem_window != 0 {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "deliberate split into 32-bit halves"
            )]
            {
                pb.push_1(
                    subchannel,
                    method::SET_SHADER_SHARED_MEMORY_WINDOW_A,
                    (shared_mem_window >> 32) as u32,
                );
                pb.push_1(
                    subchannel,
                    method::SET_SHADER_SHARED_MEMORY_WINDOW_B,
                    shared_mem_window as u32,
                );
            }
        }
        pb
    }

    fn compute_init_inner(
        compute_class: u32,
        _local_mem_window: u64,
        slm_base_addr: u64,
        slm_per_tpc_bytes: u64,
        skip_set_object: bool,
    ) -> Self {
        Self::compute_init_on_subchannel(
            compute_class,
            _local_mem_window,
            slm_base_addr,
            slm_per_tpc_bytes,
            skip_set_object,
            0,
        )
    }

    fn compute_init_on_subchannel(
        compute_class: u32,
        local_mem_window: u64,
        slm_base_addr: u64,
        slm_per_tpc_bytes: u64,
        skip_set_object: bool,
        sub: u32,
    ) -> Self {
        let mut pb = Self::new();

        if !skip_set_object {
            pb.push_1(sub, method::SET_OBJECT, compute_class);
        }

        #[expect(
            clippy::cast_possible_truncation,
            reason = "deliberate split into 32-bit halves"
        )]
        {
            // Window addresses: the SM uses these as the VA base for
            // LD.LOCAL / ST.LOCAL and shared-memory accesses. Without
            // valid windows the GPU faults with "Invalid Address Space"
            // during warp setup (even for an EXIT-only shader).
            if local_mem_window != 0 {
                pb.push_1(
                    sub,
                    method::SET_SHADER_LOCAL_MEMORY_WINDOW_A,
                    (local_mem_window >> 32) as u32,
                );
                pb.push_1(
                    sub,
                    method::SET_SHADER_LOCAL_MEMORY_WINDOW_B,
                    local_mem_window as u32,
                );
            }

            pb.push_1(
                sub,
                method::SET_SHADER_LOCAL_MEMORY_A,
                (slm_base_addr >> 32) as u32,
            );
            pb.push_1(sub, method::SET_SHADER_LOCAL_MEMORY_B, slm_base_addr as u32);

            pb.push_1(
                sub,
                method::SET_SHADER_LOCAL_MEMORY_NON_THROTTLED_A,
                (slm_per_tpc_bytes >> 32) as u32,
            );
            pb.push_1(
                sub,
                method::SET_SHADER_LOCAL_MEMORY_NON_THROTTLED_B,
                slm_per_tpc_bytes as u32,
            );
            pb.push_1(sub, method::SET_SHADER_LOCAL_MEMORY_NON_THROTTLED_C, 0xFF);
        }

        pb
    }

    /// Build a per-dispatch push buffer using the compute class to infer
    /// the launch method (PCAS vs PCAS2).
    ///
    /// Prefer [`compute_dispatch_with_launch`] when a
    /// [`GenerationProfile`](super::generation::GenerationProfile) is available.
    #[must_use]
    pub fn compute_dispatch(compute_class: u32, qmd_addr: u64) -> Self {
        use super::generation::LaunchMethod;
        let launch = if compute_class > method::TURING_COMPUTE_A {
            LaunchMethod::Pcas2
        } else {
            LaunchMethod::Pcas
        };
        Self::compute_dispatch_with_launch(launch, qmd_addr)
    }

    /// Build a per-dispatch push buffer using an explicit launch method
    /// from the generation profile.
    ///
    /// Invalidates caches and launches via `SEND_PCAS_A` + the appropriate
    /// signaling method. The compute class must already be bound to the
    /// target subchannel via a prior init or RM bind.
    #[must_use]
    pub fn compute_dispatch_with_launch(
        launch: super::generation::LaunchMethod,
        qmd_addr: u64,
    ) -> Self {
        Self::compute_dispatch_on_subchannel(launch, qmd_addr, 0)
    }

    /// Dispatch on a specific subchannel.
    #[must_use]
    pub fn compute_dispatch_on_subchannel(
        launch: super::generation::LaunchMethod,
        qmd_addr: u64,
        sub: u32,
    ) -> Self {
        use super::generation::LaunchMethod;
        let mut pb = Self::new();

        pb.push_1(
            sub,
            method::INVALIDATE_SHADER_CACHES,
            method::INVALIDATE_INSTR_AND_DATA,
        );

        #[expect(
            clippy::cast_possible_truncation,
            reason = "deliberate split into 32-bit halves"
        )]
        {
            pb.push_1(sub, method::SEND_PCAS_A, (qmd_addr >> 8) as u32);

            match launch {
                LaunchMethod::Pcas2 => {
                    pb.push_1(
                        sub,
                        method::SEND_SIGNALING_PCAS2_B,
                        method::PCAS_ACTION_INVALIDATE_COPY_SCHEDULE,
                    );
                }
                LaunchMethod::Pcas => {
                    pb.push_1(sub, method::SEND_SIGNALING_PCAS_B, 0x3);
                }
            }
        }

        pb
    }

    /// Build a semaphore release push buffer for completion signaling.
    ///
    /// On Blackwell+, USERD no longer contains GP_GET, so the host cannot
    /// poll for GPFIFO consumption. Instead, the compute engine writes a
    /// known value to a DMA-mapped semaphore address after dispatch completes.
    ///
    /// Uses `RELEASE_MEMBAR_REDUCTION` (0x06C0/0x06C4/0x06C8/0x06CC) methods:
    /// - `ADDR_UPPER` = high 8 bits of semaphore GPU VA
    /// - `ADDR_LOWER` = low 32 bits (must be 4-byte aligned)
    /// - `PAYLOAD` = value to write on completion
    /// - `CTRL` = release mode (0x5 = ACQUIRE_TERNARY + RELEASE_TRUE)
    #[must_use]
    pub fn semaphore_release(sem_iova: u64, payload: u32, subchannel: u32) -> Self {
        let mut pb = Self::new();
        #[expect(
            clippy::cast_possible_truncation,
            reason = "deliberate split into 32-bit halves"
        )]
        {
            pb.push_1(
                subchannel,
                method::SEMAPHORE_ADDR_UPPER,
                (sem_iova >> 32) as u32,
            );
            pb.push_1(subchannel, method::SEMAPHORE_ADDR_LOWER, sem_iova as u32);
        }
        pb.push_1(subchannel, method::SEMAPHORE_PAYLOAD, payload);
        pb.push_1(
            subchannel,
            method::SEMAPHORE_CTRL,
            method::SEMAPHORE_CTRL_RELEASE,
        );
        pb
    }

    /// Build a GR context init push buffer from FECS method entries.
    ///
    /// Submits the method init entries from firmware blobs as class
    /// method writes on subchannel 0. This initializes the GR engine
    /// context so that subsequent compute dispatches have a valid
    /// context (prevents CTXNOTVALID from PBDMA).
    ///
    /// Each method entry is a `(addr, value)` pair where `addr` is a
    /// GR class method offset and `value` is the data to write.
    ///
    /// Callers must ensure all addresses fit in the 13-bit push buffer
    /// method encoding (<= 0x7FFC). Use [`crate::gsp::split_for_application`]
    /// to separate BAR0 from channel-submittable entries.
    #[must_use]
    pub fn gr_context_init(compute_class: u32, method_entries: &[(u32, u32)]) -> Self {
        let mut pb = Self::new();
        let sub = 0_u32;

        pb.push_1(sub, method::SET_OBJECT, compute_class);

        for &(addr, value) in method_entries {
            debug_assert!(
                addr <= 0x7FFC,
                "method addr {addr:#x} exceeds 13-bit push buffer encoding limit"
            );
            pb.push_1(sub, addr, value);
        }

        pb
    }

    /// Build a CE (Copy Engine) init pushbuffer — binds the CE class on subchannel 0.
    #[must_use]
    pub fn ce_init(ce_class: u32) -> Self {
        let mut pb = Self::new();
        pb.push_1(0, ce::method::SET_OBJECT, ce_class);
        pb
    }

    /// Build a CE DMA copy pushbuffer.
    ///
    /// Copies `byte_count` bytes from `src_iova` to `dst_iova` using the CE
    /// LAUNCH_DMA method. Both addresses must be GPU-virtual (IOVA) from the
    /// same DMA domain.
    #[must_use]
    pub fn ce_dma_copy(src_iova: u64, dst_iova: u64, byte_count: u32) -> Self {
        let mut pb = Self::new();
        let sc = 0_u32;

        pb.push_1(sc, ce::method::OFFSET_IN_UPPER, (src_iova >> 32) as u32);
        pb.push_1(sc, ce::method::OFFSET_IN_LOWER, src_iova as u32);
        pb.push_1(sc, ce::method::OFFSET_OUT_UPPER, (dst_iova >> 32) as u32);
        pb.push_1(sc, ce::method::OFFSET_OUT_LOWER, dst_iova as u32);
        pb.push_1(sc, ce::method::PITCH_IN, byte_count);
        pb.push_1(sc, ce::method::PITCH_OUT, byte_count);
        pb.push_1(sc, ce::method::LINE_LENGTH_IN, byte_count);
        pb.push_1(sc, ce::method::LINE_COUNT, 1);
        pb.push_1(
            sc,
            ce::method::LAUNCH_DMA,
            ce::method::LAUNCH_PIPELINED_PITCH,
        );
        pb
    }

    /// Build a CE semaphore release pushbuffer.
    ///
    /// After the preceding copy completes, writes `payload` to `sem_iova`.
    #[must_use]
    pub fn ce_semaphore_release(sem_iova: u64, payload: u32) -> Self {
        let mut pb = Self::new();
        let sc = 0_u32;
        pb.push_1(sc, ce::method::SET_SEMAPHORE_A, (sem_iova >> 32) as u32);
        pb.push_1(sc, ce::method::SET_SEMAPHORE_B, sem_iova as u32);
        pb.push_1(sc, ce::method::SET_SEMAPHORE_PAYLOAD, payload);
        pb.push_1(sc, ce::method::SEMAPHORE_CTRL, 0x1);
        pb
    }
}

impl Default for PushBuf {
    fn default() -> Self {
        Self::new()
    }
}
