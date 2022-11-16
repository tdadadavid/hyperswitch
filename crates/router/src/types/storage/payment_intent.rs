use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{schema::payment_intent, types::enums, utils::date_time};

#[derive(Clone, Debug, Eq, PartialEq, Identifiable, Queryable, Serialize, Deserialize)]
#[diesel(table_name = payment_intent)]
pub struct PaymentIntent {
    pub id: i32,
    pub payment_id: String,
    pub merchant_id: String,
    pub status: enums::IntentStatus,
    pub amount: i32,
    pub currency: Option<enums::Currency>,
    pub amount_captured: Option<i32>,
    pub customer_id: Option<String>,
    pub description: Option<String>,
    pub return_url: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub connector_id: Option<String>,
    pub shipping_address_id: Option<String>,
    pub billing_address_id: Option<String>,
    pub statement_descriptor_name: Option<String>,
    pub statement_descriptor_suffix: Option<String>,
    pub created_at: PrimitiveDateTime,
    pub modified_at: PrimitiveDateTime,
    pub last_synced: Option<PrimitiveDateTime>, // FIXME: this is optional
    pub setup_future_usage: Option<enums::FutureUsage>,
    pub off_session: Option<bool>,
    pub client_secret: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Insertable, router_derive::DebugAsDisplay)]
#[diesel(table_name = payment_intent)]
pub struct PaymentIntentNew {
    pub payment_id: String,
    pub merchant_id: String,
    pub status: enums::IntentStatus,
    pub amount: i32,
    pub currency: Option<enums::Currency>,
    pub amount_captured: Option<i32>,
    pub customer_id: Option<String>,
    pub description: Option<String>,
    pub return_url: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub connector_id: Option<String>,
    pub shipping_address_id: Option<String>,
    pub billing_address_id: Option<String>,
    pub statement_descriptor_name: Option<String>,
    pub statement_descriptor_suffix: Option<String>,
    pub created_at: Option<PrimitiveDateTime>,
    pub modified_at: Option<PrimitiveDateTime>,
    pub last_synced: Option<PrimitiveDateTime>,
    pub client_secret: Option<String>,
    pub setup_future_usage: Option<enums::FutureUsage>,
    pub off_session: Option<bool>,
}

#[derive(Debug)]
pub enum PaymentIntentUpdate {
    ResponseUpdate {
        status: enums::IntentStatus,
        amount_captured: Option<i32>,
        return_url: Option<String>,
    },
    MetadataUpdate {
        metadata: serde_json::Value,
    },
    ReturnUrlUpdate {
        return_url: Option<String>,
        status: Option<enums::IntentStatus>,
        customer_id: Option<String>,
        shipping_address_id: Option<String>,
        billing_address_id: Option<String>,
    },
    MerchantStatusUpdate {
        status: enums::IntentStatus,
        shipping_address_id: Option<String>,
        billing_address_id: Option<String>,
    },
    PGStatusUpdate {
        status: enums::IntentStatus,
    },
    Update {
        amount: i32,
        currency: enums::Currency,
        status: enums::IntentStatus,
        customer_id: Option<String>,
        shipping_address_id: Option<String>,
        billing_address_id: Option<String>,
    },
}

#[derive(Clone, Debug, Default, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = payment_intent)]
pub(super) struct PaymentIntentUpdateInternal {
    amount: Option<i32>,
    currency: Option<enums::Currency>,
    status: Option<enums::IntentStatus>,
    amount_captured: Option<i32>,
    customer_id: Option<String>,
    return_url: Option<String>,
    setup_future_usage: Option<enums::FutureUsage>,
    off_session: Option<bool>,
    metadata: Option<serde_json::Value>,
    client_secret: Option<Option<String>>,
    billing_address_id: Option<String>,
    shipping_address_id: Option<String>,
    modified_at: Option<PrimitiveDateTime>,
}

impl PaymentIntentUpdate {
    pub fn apply_changeset(self, source: PaymentIntent) -> PaymentIntent {
        let internal_update: PaymentIntentUpdateInternal = self.into();
        PaymentIntent {
            amount: internal_update.amount.unwrap_or(source.amount),
            currency: internal_update.currency.or(source.currency),
            status: internal_update.status.unwrap_or(source.status),
            amount_captured: internal_update.amount_captured.or(source.amount_captured),
            customer_id: internal_update.customer_id.or(source.customer_id),
            return_url: internal_update.return_url.or(source.return_url),
            setup_future_usage: internal_update
                .setup_future_usage
                .or(source.setup_future_usage),
            off_session: internal_update.off_session.or(source.off_session),
            metadata: internal_update.metadata.or(source.metadata),
            client_secret: internal_update
                .client_secret
                .unwrap_or(source.client_secret),
            billing_address_id: internal_update
                .billing_address_id
                .or(source.billing_address_id),
            shipping_address_id: internal_update
                .shipping_address_id
                .or(source.shipping_address_id),
            modified_at: date_time::now(),
            ..source
        }
    }
}

impl From<PaymentIntentUpdate> for PaymentIntentUpdateInternal {
    fn from(payment_intent_update: PaymentIntentUpdate) -> Self {
        match payment_intent_update {
            PaymentIntentUpdate::Update {
                amount,
                currency,
                status,
                customer_id,
                shipping_address_id,
                billing_address_id,
            } => Self {
                amount: Some(amount),
                currency: Some(currency),
                status: Some(status),
                customer_id,
                client_secret: make_client_secret_null_if_success(Some(status)),
                shipping_address_id,
                billing_address_id,
                modified_at: Some(crate::utils::date_time::now()),
                ..Default::default()
            },
            PaymentIntentUpdate::MetadataUpdate { metadata } => Self {
                metadata: Some(metadata),
                modified_at: Some(crate::utils::date_time::now()),
                ..Default::default()
            },
            PaymentIntentUpdate::ReturnUrlUpdate {
                return_url,
                status,
                customer_id,
                shipping_address_id,
                billing_address_id,
            } => Self {
                return_url,
                status,
                client_secret: make_client_secret_null_if_success(status),
                customer_id,
                shipping_address_id,
                billing_address_id,
                modified_at: Some(crate::utils::date_time::now()),
                ..Default::default()
            },
            PaymentIntentUpdate::PGStatusUpdate { status } => Self {
                status: Some(status),
                modified_at: Some(crate::utils::date_time::now()),
                ..Default::default()
            },
            PaymentIntentUpdate::MerchantStatusUpdate {
                status,
                shipping_address_id,
                billing_address_id,
            } => Self {
                status: Some(status),
                client_secret: make_client_secret_null_if_success(Some(status)),
                shipping_address_id,
                billing_address_id,
                modified_at: Some(crate::utils::date_time::now()),
                ..Default::default()
            },
            PaymentIntentUpdate::ResponseUpdate {
                // amount,
                // currency,
                status,
                amount_captured,
                // customer_id,
                return_url,
            } => Self {
                // amount,
                // currency: Some(currency),
                status: Some(status),
                amount_captured,
                // customer_id,
                return_url,
                client_secret: make_client_secret_null_if_success(Some(status)),
                modified_at: Some(crate::utils::date_time::now()),
                ..Default::default()
            },
        }
    }
}

fn make_client_secret_null_if_success(
    status: Option<enums::IntentStatus>,
) -> Option<Option<String>> {
    if status == Some(enums::IntentStatus::Succeeded) {
        Option::<Option<String>>::Some(None)
    } else {
        None
    }
}
