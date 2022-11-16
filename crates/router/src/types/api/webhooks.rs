use error_stack::ResultExt;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use super::ConnectorCommon;
use crate::{
    connection,
    core::errors::{self, CustomResult},
    services,
    types::{api, storage::enums},
    utils::{crypto, custom_serde},
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebhookFlow {
    Payment,
    Refund,
    Subscription,
}

pub type MerchantWebhookConfig = std::collections::HashMap<String, WebhookFlow>;

pub struct IncomingWebhookDetails {
    pub object_reference_id: String,
    pub connector_event_type: String,
    pub resource_object: Vec<u8>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OutgoingWebhook {
    pub merchant_id: String,
    pub event_id: String,
    pub event_type: enums::EventType,
    pub content: OutgoingWebhookContent,
    #[serde(default, with = "custom_serde::iso8601")]
    pub timestamp: PrimitiveDateTime,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "object", rename_all = "snake_case")]
pub enum OutgoingWebhookContent {
    PaymentDetails(api::PaymentsResponse),
}

#[async_trait::async_trait]
pub trait IncomingWebhook: ConnectorCommon + Sync {
    fn get_webhook_body_decoding_algorithm(
        &self,
        _headers: &actix_web::http::header::HeaderMap,
        _body: &[u8],
    ) -> CustomResult<Box<dyn crypto::DecodeMessage + Send>, errors::ConnectorError> {
        Ok(Box::new(crypto::NoAlgorithm))
    }

    async fn get_webhook_body_decoding_merchant_secret(
        &self,
        _merchant_id: &str,
        _redis_conn: connection::RedisPool,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        Ok(Vec::new())
    }

    fn get_webhook_body_decoding_message(
        &self,
        _headers: &actix_web::http::header::HeaderMap,
        body: &[u8],
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        Ok(body.to_vec())
    }

    async fn decode_webhook_body(
        &self,
        headers: &actix_web::http::header::HeaderMap,
        body: &[u8],
        merchant_id: &str,
        redis_conn: connection::RedisPool,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let algorithm = self.get_webhook_body_decoding_algorithm(headers, body)?;

        let message = self
            .get_webhook_body_decoding_message(headers, body)
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        let secret = self
            .get_webhook_body_decoding_merchant_secret(merchant_id, redis_conn)
            .await
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

        algorithm
            .decode_message(&secret, &message)
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)
    }

    fn get_webhook_source_verification_algorithm(
        &self,
        _headers: &actix_web::http::header::HeaderMap,
        _body: &[u8],
    ) -> CustomResult<Box<dyn crypto::VerifySignature + Send>, errors::ConnectorError> {
        Ok(Box::new(crypto::NoAlgorithm))
    }

    async fn get_webhook_source_verification_merchant_secret(
        &self,
        _merchant_id: &str,
        _redis_conn: connection::RedisPool,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        Ok(Vec::new())
    }

    fn get_webhook_source_verification_signature(
        &self,
        _headers: &actix_web::http::header::HeaderMap,
        _body: &[u8],
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        Ok(Vec::new())
    }

    fn get_webhook_source_verification_message(
        &self,
        _headers: &actix_web::http::header::HeaderMap,
        _body: &[u8],
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        Ok(Vec::new())
    }

    async fn verify_webhook_source(
        &self,
        headers: &actix_web::http::header::HeaderMap,
        body: &[u8],
        merchant_id: &str,
        redis_conn: connection::RedisPool,
    ) -> CustomResult<bool, errors::ConnectorError> {
        let algorithm = self
            .get_webhook_source_verification_algorithm(headers, body)
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        let signature = self
            .get_webhook_source_verification_signature(headers, body)
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;
        let message = self
            .get_webhook_source_verification_message(headers, body)
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;
        let secret = self
            .get_webhook_source_verification_merchant_secret(merchant_id, redis_conn)
            .await
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        algorithm
            .verify_signature(&secret, &signature, &message)
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)
    }

    fn get_webhook_object_reference_id(
        &self,
        _body: &[u8],
    ) -> CustomResult<String, errors::ConnectorError>;

    fn get_webhook_event_type(&self, _body: &[u8]) -> CustomResult<String, errors::ConnectorError>;

    fn get_webhook_resource_object(
        &self,
        _body: &[u8],
    ) -> CustomResult<serde_json::Value, errors::ConnectorError>;

    fn get_webhook_api_response(
        &self,
    ) -> CustomResult<services::api::BachResponse<serde_json::Value>, errors::ConnectorError> {
        Ok(services::api::BachResponse::StatusOk)
    }
}
