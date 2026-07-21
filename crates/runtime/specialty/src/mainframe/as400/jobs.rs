// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;
use std::time::Duration;

use super::super::types::JCLGenerator;

use crate::{JCLSettings, LegacyJob, ToadStoolError, ToadStoolResult};

impl Default for JCLGenerator {
    fn default() -> Self {
        Self {
            templates: HashMap::new(),
            settings: JCLSettings {
                job_class: "A".to_string(),
                message_class: "A".to_string(),
                priority: 1,
                time_limit: Duration::from_secs(3600),
                region_size: 1024 * 1024,
            },
        }
    }
}

impl JCLGenerator {
    /// Creates a new JCL generator with default templates.
    pub fn new() -> Self {
        Self::default()
    }

    /// Initializes the generator with JCL settings and loads templates.
    ///
    /// # Errors
    ///
    /// Reserved for future I/O failures when loading templates.
    pub async fn initialize(&mut self, settings: &JCLSettings) -> ToadStoolResult<()> {
        self.settings = settings.clone();

        // Load JCL templates
        self.templates.insert(
            "COBOL_COMPILE".to_string(),
            "//COBOLJOB JOB (ACCT),CLASS={job_class},MSGCLASS={message_class}\n\
             //COMPILE  EXEC PGM=IGYCRCTL\n\
             //STEPLIB  DD  DSN=IGY.SIGYCOMP,DISP=SHR\n\
             //SYSPRINT DD  SYSOUT=*\n\
             //SYSLIN   DD  DSN=&&LOADSET,DISP=(MOD,PASS),\n\
             //             UNIT=SYSDA,SPACE=(CYL,(1,1))\n\
             //SYSIN    DD  DSN={source_dataset},DISP=SHR\n"
                .to_string(),
        );

        Ok(())
    }

    /// Generates JCL for the given legacy job.
    ///
    /// # Errors
    ///
    /// Returns when a required JCL template is missing.
    pub async fn generate_jcl(&self, _job: &LegacyJob) -> ToadStoolResult<String> {
        // Generate JCL based on job type
        let template = self
            .templates
            .get("COBOL_COMPILE")
            .ok_or_else(|| ToadStoolError::runtime("JCL template not found"))?;

        let jcl = template
            .replace("{job_class}", &self.settings.job_class)
            .replace("{message_class}", &self.settings.message_class)
            .replace("{source_dataset}", "USER.SOURCE(HELLO)");

        Ok(jcl)
    }
}
