use async_graphql::*;

use crate::primitives::*;
pub use lana_app::primitives::CollateralAction;

#[derive(async_graphql::Union)]
pub enum CreditFacilityHistoryEntry {
    Payment(CreditFacilityIncrementalPayment),
    Collateral(CreditFacilityCollateralUpdated),
    Approved(CreditFacilityApproved),
    Collateralization(CreditFacilityCollateralizationUpdated),
    Disbursal(CreditFacilityDisbursalExecuted),
    Interest(CreditFacilityInterestAccrued),
    ReservedForLiquidation(CreditFacilityLiquidationAmountReserved),
}

#[derive(SimpleObject)]
pub struct CreditFacilityIncrementalPayment {
    pub cents: UsdCents,
    pub recorded_at: Timestamp,
    pub effective: Date,
    pub tx_id: UUID,
}

#[derive(SimpleObject)]
pub struct CreditFacilityCollateralUpdated {
    pub satoshis: Satoshis,
    pub recorded_at: Timestamp,
    pub effective: Date,
    pub action: CollateralAction,
    pub tx_id: UUID,
}

#[derive(SimpleObject)]
pub struct CreditFacilityApproved {
    pub cents: UsdCents,
    pub recorded_at: Timestamp,
    pub effective: Date,
    pub tx_id: UUID,
}

#[derive(SimpleObject)]
pub struct CreditFacilityCollateralizationUpdated {
    pub state: CollateralizationState,
    pub collateral: Satoshis,
    pub outstanding_interest: UsdCents,
    pub outstanding_disbursal: UsdCents,
    pub recorded_at: Timestamp,
    pub effective: Date,
    pub price: UsdCents,
}

#[derive(SimpleObject)]
pub struct CreditFacilityDisbursalExecuted {
    pub cents: UsdCents,
    pub recorded_at: Timestamp,
    pub effective: Date,
    pub tx_id: UUID,
}

#[derive(SimpleObject)]
pub struct CreditFacilityInterestAccrued {
    pub cents: UsdCents,
    pub recorded_at: Timestamp,
    pub effective: Date,
    pub tx_id: UUID,
    pub days: u32,
}

#[derive(SimpleObject)]
pub struct CreditFacilityLiquidationAmountReserved {
    pub cents: UsdCents,
    pub recorded_at: Timestamp,
    pub effective: Date,
    pub tx_id: UUID,
}

impl From<lana_app::credit::CreditFacilityHistoryEntry> for CreditFacilityHistoryEntry {
    fn from(transaction: lana_app::credit::CreditFacilityHistoryEntry) -> Self {
        match transaction {
            lana_app::credit::CreditFacilityHistoryEntry::Payment(payment) => {
                CreditFacilityHistoryEntry::Payment(payment.into())
            }
            lana_app::credit::CreditFacilityHistoryEntry::Collateral(collateral) => {
                CreditFacilityHistoryEntry::Collateral(collateral.into())
            }
            lana_app::credit::CreditFacilityHistoryEntry::Approved(approved) => {
                CreditFacilityHistoryEntry::Approved(approved.into())
            }
            lana_app::credit::CreditFacilityHistoryEntry::Collateralization(collateralization) => {
                CreditFacilityHistoryEntry::Collateralization(collateralization.into())
            }
            lana_app::credit::CreditFacilityHistoryEntry::Disbursal(disbursal) => {
                CreditFacilityHistoryEntry::Disbursal(disbursal.into())
            }
            lana_app::credit::CreditFacilityHistoryEntry::Interest(interest) => {
                CreditFacilityHistoryEntry::Interest(interest.into())
            }
            lana_app::credit::CreditFacilityHistoryEntry::ReservedForLiquidation(liquidation) => {
                CreditFacilityHistoryEntry::ReservedForLiquidation(liquidation.into())
            }
        }
    }
}

impl From<lana_app::credit::IncrementalPayment> for CreditFacilityIncrementalPayment {
    fn from(payment: lana_app::credit::IncrementalPayment) -> Self {
        Self {
            cents: payment.cents,
            recorded_at: payment.recorded_at.into(),
            effective: payment.effective.into(),
            tx_id: UUID::from(payment.payment_id),
        }
    }
}

impl From<lana_app::credit::CollateralUpdated> for CreditFacilityCollateralUpdated {
    fn from(collateral: lana_app::credit::CollateralUpdated) -> Self {
        Self {
            satoshis: collateral.satoshis,
            recorded_at: collateral.recorded_at.into(),
            effective: collateral.effective.into(),
            action: collateral.action,
            tx_id: UUID::from(collateral.tx_id),
        }
    }
}

impl From<lana_app::credit::CreditFacilityApproved> for CreditFacilityApproved {
    fn from(origination: lana_app::credit::CreditFacilityApproved) -> Self {
        Self {
            cents: origination.cents,
            recorded_at: origination.recorded_at.into(),
            effective: origination.effective.into(),
            tx_id: UUID::from(origination.tx_id),
        }
    }
}

impl From<lana_app::credit::CollateralizationUpdated> for CreditFacilityCollateralizationUpdated {
    fn from(collateralization: lana_app::credit::CollateralizationUpdated) -> Self {
        Self {
            state: collateralization.state,
            collateral: collateralization.collateral,
            outstanding_interest: collateralization.outstanding_interest,
            outstanding_disbursal: collateralization.outstanding_disbursal,
            recorded_at: collateralization.recorded_at.into(),
            effective: collateralization.effective.into(),
            price: collateralization.price.into_inner(),
        }
    }
}

impl From<lana_app::credit::DisbursalExecuted> for CreditFacilityDisbursalExecuted {
    fn from(disbursal: lana_app::credit::DisbursalExecuted) -> Self {
        Self {
            cents: disbursal.cents,
            recorded_at: disbursal.recorded_at.into(),
            effective: disbursal.effective.into(),
            tx_id: UUID::from(disbursal.tx_id),
        }
    }
}

impl From<lana_app::credit::InterestAccrualsPosted> for CreditFacilityInterestAccrued {
    fn from(interest: lana_app::credit::InterestAccrualsPosted) -> Self {
        Self {
            cents: interest.cents,
            recorded_at: interest.recorded_at.into(),
            effective: interest.effective.into(),
            tx_id: UUID::from(interest.tx_id),
            days: interest.days,
        }
    }
}

impl From<lana_app::credit::ObligationMovedToLiquidation>
    for CreditFacilityLiquidationAmountReserved
{
    fn from(liquidation: lana_app::credit::ObligationMovedToLiquidation) -> Self {
        Self {
            cents: liquidation.cents,
            recorded_at: liquidation.recorded_at.into(),
            effective: liquidation.effective.into(),
            tx_id: UUID::from(liquidation.tx_id),
        }
    }
}
