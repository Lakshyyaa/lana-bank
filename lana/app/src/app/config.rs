use serde::{Deserialize, Serialize};

use std::path::PathBuf;

use crate::{
    access::config::AccessConfig, applicant::SumsubConfig,
    contract_creation::ContractCreationConfig, credit::CreditConfig, custody::CustodyConfig,
    customer_sync::CustomerSyncConfig, job::JobsConfig, notification::NotificationConfig,
    report::ReportConfig, service_account::ServiceAccountConfig, storage::config::StorageConfig,
    user_onboarding::UserOnboardingConfig,
};

#[derive(Clone, Default, Debug, Deserialize, Serialize)]
pub struct AppConfig {
    #[serde(default)]
    pub job_execution: JobsConfig,
    #[serde(default)]
    pub sumsub: SumsubConfig,
    #[serde(default)]
    pub access: AccessConfig,
    #[serde(default)]
    pub credit: CreditConfig,
    #[serde(default)]
    pub service_account: ServiceAccountConfig,
    #[serde(default)]
    pub report: ReportConfig,
    #[serde(default)]
    pub storage: StorageConfig,
    #[serde(default)]
    pub user_onboarding: UserOnboardingConfig,
    #[serde(default)]
    pub customer_sync: CustomerSyncConfig,
    #[serde(default)]
    pub accounting_init: AccountingInitConfig,
    #[serde(default)]
    pub custody: CustodyConfig,
    #[serde(default)]
    pub notification: NotificationConfig,
    #[serde(default)]
    pub contract_creation: ContractCreationConfig,
}

#[derive(Clone, Default, Debug, Deserialize, Serialize)]
pub struct AccountingInitConfig {
    #[serde(default)]
    pub chart_of_accounts_seed_path: Option<PathBuf>,
    #[serde(default)]
    pub deposit_config_path: Option<PathBuf>,
    #[serde(default)]
    pub credit_config_path: Option<PathBuf>,
    #[serde(default)]
    pub balance_sheet_config_path: Option<PathBuf>,
    #[serde(default)]
    pub profit_and_loss_config_path: Option<PathBuf>,
}
