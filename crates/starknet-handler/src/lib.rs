#![deny(unused_crate_dependencies)]

pub mod account;
pub mod provider;
use common::LightClientError;
use thiserror::Error;
use starknet::accounts::single_owner::SignError;
use starknet::signers::local_wallet::SignError as LocalWalletSignError;
use starknet::accounts::AccountError;

#[derive(Error, Debug)]
pub enum StarknetHandlerError {
    #[error("Failed to parse: {0}")]
    ParseError(String),
    #[error("Failed to create selector: {0}")]
    SelectorError(String),
    #[error("Failed to execute transaction: {0}")]
    TransactionError(String),
    #[error("LightClient error: {0}")]
    LightClient(#[from] LightClientError),
    #[error("Starknet error: {0}")]
    Starknet(#[from] SignError<LocalWalletSignError>),
    #[error("Account error: {0}")]
    Account(#[from] AccountError<SignError<LocalWalletSignError>>),
}
