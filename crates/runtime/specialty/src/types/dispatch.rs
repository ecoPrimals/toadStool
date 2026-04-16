// SPDX-License-Identifier: AGPL-3.0-or-later
//! Concrete dispatch enums for legacy adapters and emulators.

use std::future::Future;
use std::path::Path;
use uuid::Uuid;

use crate::embedded::{Microcontroller8BitAdapter, System16BitAdapter};
use crate::emulation::{Apple2Emulator, PDP11Emulator};
use crate::industrial::{PLCAdapter, SCADAAdapter};
use crate::mainframe::{AS400Adapter, IBMMainframeAdapter, VAXVMSAdapter};
use crate::realtime::{QNXAdapter, VxWorksAdapter};
use crate::types::emulation::{EmulationConfig, EmulationStatus, LegacyEmulator};
use crate::types::jobs::LegacyJob;
use crate::types::traits::{JobOutput, JobStatus, LegacyAdapter, SystemInfo};
use crate::{SpecialtyRuntimeConfig, ToadStoolResult};

/// Enum over all [`LegacyAdapter`] implementations registered by the runtime engine.
#[derive(Debug)]
pub enum LegacyAdapterDispatch {
    /// VxWorks real-time adapter.
    VxWorks(VxWorksAdapter),
    /// QNX real-time adapter.
    Qnx(QNXAdapter),
    /// VAX/VMS mainframe adapter.
    VaxVms(VAXVMSAdapter),
    /// IBM System/360–family mainframe adapter.
    IbmMainframe(IBMMainframeAdapter),
    /// IBM AS/400 adapter.
    As400(AS400Adapter),
    /// PLC industrial adapter.
    Plc(PLCAdapter),
    /// SCADA industrial adapter.
    Scada(SCADAAdapter),
    /// 8-bit embedded microcontroller adapter.
    Microcontroller8Bit(Microcontroller8BitAdapter),
    /// 16-bit embedded system adapter.
    System16Bit(System16BitAdapter),
}

impl LegacyAdapter for LegacyAdapterDispatch {
    fn name(&self) -> &'static str {
        match self {
            LegacyAdapterDispatch::VxWorks(a) => a.name(),
            LegacyAdapterDispatch::Qnx(a) => a.name(),
            LegacyAdapterDispatch::VaxVms(a) => a.name(),
            LegacyAdapterDispatch::IbmMainframe(a) => a.name(),
            LegacyAdapterDispatch::As400(a) => a.name(),
            LegacyAdapterDispatch::Plc(a) => a.name(),
            LegacyAdapterDispatch::Scada(a) => a.name(),
            LegacyAdapterDispatch::Microcontroller8Bit(a) => a.name(),
            LegacyAdapterDispatch::System16Bit(a) => a.name(),
        }
    }

    fn supported_systems(&self) -> Vec<crate::LegacySystemType> {
        match self {
            LegacyAdapterDispatch::VxWorks(a) => a.supported_systems(),
            LegacyAdapterDispatch::Qnx(a) => a.supported_systems(),
            LegacyAdapterDispatch::VaxVms(a) => a.supported_systems(),
            LegacyAdapterDispatch::IbmMainframe(a) => a.supported_systems(),
            LegacyAdapterDispatch::As400(a) => a.supported_systems(),
            LegacyAdapterDispatch::Plc(a) => a.supported_systems(),
            LegacyAdapterDispatch::Scada(a) => a.supported_systems(),
            LegacyAdapterDispatch::Microcontroller8Bit(a) => a.supported_systems(),
            LegacyAdapterDispatch::System16Bit(a) => a.supported_systems(),
        }
    }

    fn initialize<'a>(
        &'a mut self,
        config: &'a SpecialtyRuntimeConfig,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a {
        async move {
            match self {
                LegacyAdapterDispatch::VxWorks(a) => a.initialize(config).await,
                LegacyAdapterDispatch::Qnx(a) => a.initialize(config).await,
                LegacyAdapterDispatch::VaxVms(a) => a.initialize(config).await,
                LegacyAdapterDispatch::IbmMainframe(a) => a.initialize(config).await,
                LegacyAdapterDispatch::As400(a) => a.initialize(config).await,
                LegacyAdapterDispatch::Plc(a) => a.initialize(config).await,
                LegacyAdapterDispatch::Scada(a) => a.initialize(config).await,
                LegacyAdapterDispatch::Microcontroller8Bit(a) => a.initialize(config).await,
                LegacyAdapterDispatch::System16Bit(a) => a.initialize(config).await,
            }
        }
    }

    fn shutdown<'a>(&'a mut self) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a {
        async move {
            match self {
                LegacyAdapterDispatch::VxWorks(a) => a.shutdown().await,
                LegacyAdapterDispatch::Qnx(a) => a.shutdown().await,
                LegacyAdapterDispatch::VaxVms(a) => a.shutdown().await,
                LegacyAdapterDispatch::IbmMainframe(a) => a.shutdown().await,
                LegacyAdapterDispatch::As400(a) => a.shutdown().await,
                LegacyAdapterDispatch::Plc(a) => a.shutdown().await,
                LegacyAdapterDispatch::Scada(a) => a.shutdown().await,
                LegacyAdapterDispatch::Microcontroller8Bit(a) => a.shutdown().await,
                LegacyAdapterDispatch::System16Bit(a) => a.shutdown().await,
            }
        }
    }

    fn submit_job(
        &self,
        job: LegacyJob,
    ) -> impl Future<Output = ToadStoolResult<Uuid>> + Send + '_ {
        async move {
            match self {
                LegacyAdapterDispatch::VxWorks(a) => a.submit_job(job).await,
                LegacyAdapterDispatch::Qnx(a) => a.submit_job(job).await,
                LegacyAdapterDispatch::VaxVms(a) => a.submit_job(job).await,
                LegacyAdapterDispatch::IbmMainframe(a) => a.submit_job(job).await,
                LegacyAdapterDispatch::As400(a) => a.submit_job(job).await,
                LegacyAdapterDispatch::Plc(a) => a.submit_job(job).await,
                LegacyAdapterDispatch::Scada(a) => a.submit_job(job).await,
                LegacyAdapterDispatch::Microcontroller8Bit(a) => a.submit_job(job).await,
                LegacyAdapterDispatch::System16Bit(a) => a.submit_job(job).await,
            }
        }
    }

    fn get_job_status(
        &self,
        job_id: Uuid,
    ) -> impl Future<Output = ToadStoolResult<JobStatus>> + Send + '_ {
        async move {
            match self {
                LegacyAdapterDispatch::VxWorks(a) => a.get_job_status(job_id).await,
                LegacyAdapterDispatch::Qnx(a) => a.get_job_status(job_id).await,
                LegacyAdapterDispatch::VaxVms(a) => a.get_job_status(job_id).await,
                LegacyAdapterDispatch::IbmMainframe(a) => a.get_job_status(job_id).await,
                LegacyAdapterDispatch::As400(a) => a.get_job_status(job_id).await,
                LegacyAdapterDispatch::Plc(a) => a.get_job_status(job_id).await,
                LegacyAdapterDispatch::Scada(a) => a.get_job_status(job_id).await,
                LegacyAdapterDispatch::Microcontroller8Bit(a) => a.get_job_status(job_id).await,
                LegacyAdapterDispatch::System16Bit(a) => a.get_job_status(job_id).await,
            }
        }
    }

    fn cancel_job(&self, job_id: Uuid) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        async move {
            match self {
                LegacyAdapterDispatch::VxWorks(a) => a.cancel_job(job_id).await,
                LegacyAdapterDispatch::Qnx(a) => a.cancel_job(job_id).await,
                LegacyAdapterDispatch::VaxVms(a) => a.cancel_job(job_id).await,
                LegacyAdapterDispatch::IbmMainframe(a) => a.cancel_job(job_id).await,
                LegacyAdapterDispatch::As400(a) => a.cancel_job(job_id).await,
                LegacyAdapterDispatch::Plc(a) => a.cancel_job(job_id).await,
                LegacyAdapterDispatch::Scada(a) => a.cancel_job(job_id).await,
                LegacyAdapterDispatch::Microcontroller8Bit(a) => a.cancel_job(job_id).await,
                LegacyAdapterDispatch::System16Bit(a) => a.cancel_job(job_id).await,
            }
        }
    }

    fn get_job_output(
        &self,
        job_id: Uuid,
    ) -> impl Future<Output = ToadStoolResult<JobOutput>> + Send + '_ {
        async move {
            match self {
                LegacyAdapterDispatch::VxWorks(a) => a.get_job_output(job_id).await,
                LegacyAdapterDispatch::Qnx(a) => a.get_job_output(job_id).await,
                LegacyAdapterDispatch::VaxVms(a) => a.get_job_output(job_id).await,
                LegacyAdapterDispatch::IbmMainframe(a) => a.get_job_output(job_id).await,
                LegacyAdapterDispatch::As400(a) => a.get_job_output(job_id).await,
                LegacyAdapterDispatch::Plc(a) => a.get_job_output(job_id).await,
                LegacyAdapterDispatch::Scada(a) => a.get_job_output(job_id).await,
                LegacyAdapterDispatch::Microcontroller8Bit(a) => a.get_job_output(job_id).await,
                LegacyAdapterDispatch::System16Bit(a) => a.get_job_output(job_id).await,
            }
        }
    }

    fn get_system_info(&self) -> impl Future<Output = ToadStoolResult<SystemInfo>> + Send + '_ {
        async move {
            match self {
                LegacyAdapterDispatch::VxWorks(a) => a.get_system_info().await,
                LegacyAdapterDispatch::Qnx(a) => a.get_system_info().await,
                LegacyAdapterDispatch::VaxVms(a) => a.get_system_info().await,
                LegacyAdapterDispatch::IbmMainframe(a) => a.get_system_info().await,
                LegacyAdapterDispatch::As400(a) => a.get_system_info().await,
                LegacyAdapterDispatch::Plc(a) => a.get_system_info().await,
                LegacyAdapterDispatch::Scada(a) => a.get_system_info().await,
                LegacyAdapterDispatch::Microcontroller8Bit(a) => a.get_system_info().await,
                LegacyAdapterDispatch::System16Bit(a) => a.get_system_info().await,
            }
        }
    }

    fn test_connectivity(&self) -> impl Future<Output = ToadStoolResult<bool>> + Send + '_ {
        async move {
            match self {
                LegacyAdapterDispatch::VxWorks(a) => a.test_connectivity().await,
                LegacyAdapterDispatch::Qnx(a) => a.test_connectivity().await,
                LegacyAdapterDispatch::VaxVms(a) => a.test_connectivity().await,
                LegacyAdapterDispatch::IbmMainframe(a) => a.test_connectivity().await,
                LegacyAdapterDispatch::As400(a) => a.test_connectivity().await,
                LegacyAdapterDispatch::Plc(a) => a.test_connectivity().await,
                LegacyAdapterDispatch::Scada(a) => a.test_connectivity().await,
                LegacyAdapterDispatch::Microcontroller8Bit(a) => a.test_connectivity().await,
                LegacyAdapterDispatch::System16Bit(a) => a.test_connectivity().await,
            }
        }
    }
}

/// Enum over all [`LegacyEmulator`] implementations registered by the runtime engine.
#[derive(Debug)]
pub enum LegacyEmulatorDispatch {
    /// DEC PDP-11 emulator.
    Pdp11(PDP11Emulator),
    /// Apple II emulator.
    Apple2(Apple2Emulator),
}

impl LegacyEmulator for LegacyEmulatorDispatch {
    fn name(&self) -> &'static str {
        match self {
            LegacyEmulatorDispatch::Pdp11(e) => e.name(),
            LegacyEmulatorDispatch::Apple2(e) => e.name(),
        }
    }

    fn supported_systems(&self) -> Vec<crate::LegacySystemType> {
        match self {
            LegacyEmulatorDispatch::Pdp11(e) => e.supported_systems(),
            LegacyEmulatorDispatch::Apple2(e) => e.supported_systems(),
        }
    }

    fn initialize<'a>(
        &'a mut self,
        config: &'a EmulationConfig,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a {
        async move {
            match self {
                LegacyEmulatorDispatch::Pdp11(e) => e.initialize(config).await,
                LegacyEmulatorDispatch::Apple2(e) => e.initialize(config).await,
            }
        }
    }

    fn start<'a>(&'a mut self) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a {
        async move {
            match self {
                LegacyEmulatorDispatch::Pdp11(e) => e.start().await,
                LegacyEmulatorDispatch::Apple2(e) => e.start().await,
            }
        }
    }

    fn stop<'a>(&'a mut self) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a {
        async move {
            match self {
                LegacyEmulatorDispatch::Pdp11(e) => e.stop().await,
                LegacyEmulatorDispatch::Apple2(e) => e.stop().await,
            }
        }
    }

    fn reset<'a>(&'a mut self) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a {
        async move {
            match self {
                LegacyEmulatorDispatch::Pdp11(e) => e.reset().await,
                LegacyEmulatorDispatch::Apple2(e) => e.reset().await,
            }
        }
    }

    fn load_image<'a>(
        &'a mut self,
        image: &'a Path,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a {
        async move {
            match self {
                LegacyEmulatorDispatch::Pdp11(e) => e.load_image(image).await,
                LegacyEmulatorDispatch::Apple2(e) => e.load_image(image).await,
            }
        }
    }

    fn save_state<'a>(
        &'a mut self,
        path: &'a Path,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a {
        async move {
            match self {
                LegacyEmulatorDispatch::Pdp11(e) => e.save_state(path).await,
                LegacyEmulatorDispatch::Apple2(e) => e.save_state(path).await,
            }
        }
    }

    fn load_state<'a>(
        &'a mut self,
        path: &'a Path,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a {
        async move {
            match self {
                LegacyEmulatorDispatch::Pdp11(e) => e.load_state(path).await,
                LegacyEmulatorDispatch::Apple2(e) => e.load_state(path).await,
            }
        }
    }

    fn get_status<'a>(
        &'a self,
    ) -> impl Future<Output = ToadStoolResult<EmulationStatus>> + Send + 'a {
        async move {
            match self {
                LegacyEmulatorDispatch::Pdp11(e) => e.get_status().await,
                LegacyEmulatorDispatch::Apple2(e) => e.get_status().await,
            }
        }
    }
}
