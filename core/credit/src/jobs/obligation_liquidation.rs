use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use audit::AuditSvc;
use authz::PermissionCheck;
use job::*;
use outbox::OutboxEventMarker;

use crate::{event::CoreCreditEvent, ledger::CreditLedger, obligation::Obligations, primitives::*};

use super::obligation_defaulted;

#[derive(Clone, Serialize, Deserialize)]
pub struct ObligationLiquidationJobConfig<Perms, E> {
    pub obligation_id: ObligationId,
    pub effective: chrono::NaiveDate,
    pub _phantom: std::marker::PhantomData<(Perms, E)>,
}
impl<Perms, E> JobConfig for ObligationLiquidationJobConfig<Perms, E>
where
    Perms: PermissionCheck,
    <<Perms as PermissionCheck>::Audit as AuditSvc>::Action: From<CoreCreditAction>,
    <<Perms as PermissionCheck>::Audit as AuditSvc>::Object: From<CoreCreditObject>,
    E: OutboxEventMarker<CoreCreditEvent>,
{
    type Initializer = ObligationLiquidationInit<Perms, E>;
}
pub struct ObligationLiquidationInit<Perms, E>
where
    Perms: PermissionCheck,
    E: OutboxEventMarker<CoreCreditEvent>,
{
    obligations: Obligations<Perms, E>,
    ledger: CreditLedger,
    jobs: Jobs,
}

impl<Perms, E> ObligationLiquidationInit<Perms, E>
where
    Perms: PermissionCheck,
    <<Perms as PermissionCheck>::Audit as AuditSvc>::Action: From<CoreCreditAction>,
    <<Perms as PermissionCheck>::Audit as AuditSvc>::Object: From<CoreCreditObject>,
    E: OutboxEventMarker<CoreCreditEvent>,
{
    pub fn new(ledger: &CreditLedger, obligations: &Obligations<Perms, E>, jobs: &Jobs) -> Self {
        Self {
            ledger: ledger.clone(),
            obligations: obligations.clone(),
            jobs: jobs.clone(),
        }
    }
}

const OBLIGATION_LIQUIDATION_PROCESSING_JOB: JobType =
    JobType::new("obligation-liquidation-processing");
impl<Perms, E> JobInitializer for ObligationLiquidationInit<Perms, E>
where
    Perms: PermissionCheck,
    <<Perms as PermissionCheck>::Audit as AuditSvc>::Action: From<CoreCreditAction>,
    <<Perms as PermissionCheck>::Audit as AuditSvc>::Object: From<CoreCreditObject>,
    E: OutboxEventMarker<CoreCreditEvent>,
{
    fn job_type() -> JobType
    where
        Self: Sized,
    {
        OBLIGATION_LIQUIDATION_PROCESSING_JOB
    }

    fn init(&self, job: &Job) -> Result<Box<dyn JobRunner>, Box<dyn std::error::Error>> {
        Ok(Box::new(ObligationLiquidationJobRunner::<Perms, E> {
            config: job.config()?,
            obligations: self.obligations.clone(),
            ledger: self.ledger.clone(),
            jobs: self.jobs.clone(),
        }))
    }
}

pub struct ObligationLiquidationJobRunner<Perms, E>
where
    Perms: PermissionCheck,
    E: OutboxEventMarker<CoreCreditEvent>,
{
    config: ObligationLiquidationJobConfig<Perms, E>,
    obligations: Obligations<Perms, E>,
    ledger: CreditLedger,
    jobs: Jobs,
}

#[async_trait]
impl<Perms, E> JobRunner for ObligationLiquidationJobRunner<Perms, E>
where
    Perms: PermissionCheck,
    <<Perms as PermissionCheck>::Audit as AuditSvc>::Action: From<CoreCreditAction>,
    <<Perms as PermissionCheck>::Audit as AuditSvc>::Object: From<CoreCreditObject>,
    E: OutboxEventMarker<CoreCreditEvent>,
{
    async fn run(
        &self,
        _current_job: CurrentJob,
    ) -> Result<JobCompletion, Box<dyn std::error::Error>> {
        let mut db = self.obligations.begin_op().await?;

        let (obligation, liquidation_process) = self
            .obligations
            .start_liquidation_process_in_op(
                &mut db,
                self.config.obligation_id,
                self.config.effective,
            )
            .await?;

        let liquidation_process = if let Some(liquidation_process) = liquidation_process {
            liquidation_process
        } else {
            return Ok(JobCompletion::Complete);
        };

        if let Some(defaulted_at) = obligation.defaulted_at() {
            self.jobs
                .create_and_spawn_at_in_op(
                    &mut db,
                    JobId::new(),
                    obligation_defaulted::ObligationDefaultedJobConfig::<Perms, E> {
                        obligation_id: obligation.id,
                        effective: defaulted_at.date_naive(),
                        _phantom: std::marker::PhantomData,
                    },
                    defaulted_at,
                )
                .await?;
        }

        self.ledger
            .reserve_for_liquidation(db, liquidation_process)
            .await?;

        Ok(JobCompletion::Complete)
    }
}
