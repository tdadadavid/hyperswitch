use crate::{
    core::errors::{self, CustomResult},
    types::{
        api,
        storage::{PaymentIntent, PaymentIntentNew, PaymentIntentUpdate},
    },
};

#[async_trait::async_trait]
pub trait IPaymentIntent {
    async fn update_payment_intent(
        &self,
        this: PaymentIntent,
        payment_intent: PaymentIntentUpdate,
    ) -> CustomResult<PaymentIntent, errors::StorageError>;

    async fn insert_payment_intent(
        &self,
        new: PaymentIntentNew,
    ) -> CustomResult<PaymentIntent, errors::StorageError>;

    async fn find_payment_intent_by_payment_id_merchant_id(
        &self,
        payment_id: &str,
        merchant_id: &str,
    ) -> CustomResult<PaymentIntent, errors::StorageError>;

    async fn filter_payment_intent_by_constraints(
        &self,
        merchant_id: &str,
        pc: &api::PaymentListConstraints,
    ) -> CustomResult<Vec<PaymentIntent>, errors::StorageError>;
}

#[cfg(feature = "kv_store")]
mod storage {
    use error_stack::{IntoReport, ResultExt};
    use fred::prelude::{RedisErrorKind, *};

    use super::IPaymentIntent;
    use crate::{
        core::errors::{self, CustomResult},
        services::Store,
        types::{api, storage::payment_intent::*},
        utils::date_time,
    };

    #[async_trait::async_trait]
    impl IPaymentIntent for Store {
        async fn insert_payment_intent(
            &self,
            new: PaymentIntentNew,
        ) -> CustomResult<PaymentIntent, errors::StorageError> {
            let key = format!("{}_{}", new.payment_id, new.merchant_id);
            let created_intent = PaymentIntent {
                id: 0i32,
                payment_id: new.payment_id,
                merchant_id: new.merchant_id,
                status: new.status,
                amount: new.amount,
                currency: new.currency,
                amount_captured: new.amount_captured,
                customer_id: new.customer_id,
                description: new.description,
                return_url: new.return_url,
                metadata: new.metadata,
                connector_id: new.connector_id,
                shipping_address_id: new.shipping_address_id,
                billing_address_id: new.billing_address_id,
                statement_descriptor_name: new.statement_descriptor_name,
                statement_descriptor_suffix: new.statement_descriptor_suffix,
                created_at: new.created_at.unwrap_or_else(date_time::now),
                modified_at: new.created_at.unwrap_or_else(date_time::now),
                last_synced: new.last_synced,
                setup_future_usage: new.setup_future_usage,
                off_session: new.off_session,
                client_secret: new.client_secret,
            };
            // TODO: Add a proper error for serialization failure
            let redis_value = serde_json::to_string(&created_intent)
                .into_report()
                .change_context(errors::StorageError::KVError)?;
            match self
                .redis_conn
                .pool
                .hsetnx::<u8, &str, &str, &str>(&key, "pa", &redis_value)
                .await
            {
                Ok(0) => Err(errors::StorageError::DuplicateValue(format!(
                    "Payment Intent already exists for payment_id: {}",
                    key
                )))
                .into_report(),
                Ok(1) => Ok(created_intent),
                Ok(i) => Err(errors::StorageError::KVError)
                    .into_report()
                    .attach_printable_lazy(|| format!("Invalid response for HSETNX: {}", i)),
                Err(er) => Err(er)
                    .into_report()
                    .change_context(errors::StorageError::KVError),
            }
        }

        async fn update_payment_intent(
            &self,
            this: PaymentIntent,
            payment_intent: PaymentIntentUpdate,
        ) -> CustomResult<PaymentIntent, errors::StorageError> {
            let key = format!("{}_{}", this.payment_id, this.merchant_id);

            let updated_intent = payment_intent.apply_changeset(this);
            // Check for database presence as well Maybe use a read replica here ?
            // TODO: Add a proper error for serialization failure
            let redis_value = serde_json::to_string(&updated_intent)
                .into_report()
                .change_context(errors::StorageError::KVError)?;
            self.redis_conn
                .pool
                .hset::<u8, &str, (&str, String)>(&key, ("pi", redis_value))
                .await
                .map(|_| updated_intent)
                .into_report()
                .change_context(errors::StorageError::KVError)
        }

        async fn find_payment_intent_by_payment_id_merchant_id(
            &self,
            payment_id: &str,
            merchant_id: &str,
        ) -> CustomResult<PaymentIntent, errors::StorageError> {
            let key = format!("{}_{}", payment_id, merchant_id);
            self.redis_conn
                .pool
                .hget::<String, &str, &str>(&key, "pi")
                .await
                .map_err(|err| match err.kind() {
                    RedisErrorKind::NotFound => errors::StorageError::ValueNotFound(format!(
                        "Payment Intent does not exist for {}",
                        key
                    )),
                    _ => errors::StorageError::KVError,
                })
                .into_report()
                .and_then(|redis_resp| {
                    serde_json::from_str::<PaymentIntent>(&redis_resp)
                        .into_report()
                        .change_context(errors::StorageError::KVError)
                })
            // Check for database presence as well Maybe use a read replica here ?
        }
        async fn filter_payment_intent_by_constraints(
            &self,
            merchant_id: &str,
            pc: &api::PaymentListConstraints,
        ) -> CustomResult<Vec<PaymentIntent>, errors::StorageError> {
            //TODO: Implement this
            Err(errors::StorageError::KVError.into())
        }
    }
}

#[cfg(not(feature = "kv_store"))]
mod storage {
    use super::IPaymentIntent;
    use crate::{
        connection::pg_connection,
        core::errors::{self, CustomResult},
        services::Store,
        types::{api, storage::payment_intent::*},
    };

    #[async_trait::async_trait]
    impl IPaymentIntent for Store {
        async fn insert_payment_intent(
            &self,
            new: PaymentIntentNew,
        ) -> CustomResult<PaymentIntent, errors::StorageError> {
            let conn = pg_connection(&self.pg_pool.conn).await;
            new.insert(&conn).await
        }

        async fn update_payment_intent(
            &self,
            this: PaymentIntent,
            payment_intent: PaymentIntentUpdate,
        ) -> CustomResult<PaymentIntent, errors::StorageError> {
            let conn = pg_connection(&self.pg_pool.conn).await;
            this.update(&conn, payment_intent).await
        }

        async fn find_payment_intent_by_payment_id_merchant_id(
            &self,
            payment_id: &str,
            merchant_id: &str,
        ) -> CustomResult<PaymentIntent, errors::StorageError> {
            let conn = pg_connection(&self.pg_pool.conn).await;
            PaymentIntent::find_by_payment_id_merchant_id(&conn, payment_id, merchant_id).await
        }

        async fn filter_payment_intent_by_constraints(
            &self,
            merchant_id: &str,
            pc: &api::PaymentListConstraints,
        ) -> CustomResult<Vec<PaymentIntent>, errors::StorageError> {
            let conn = pg_connection(&self.pg_pool.conn).await;
            PaymentIntent::filter_by_constraints(&conn, merchant_id, pc).await
        }
    }
}
