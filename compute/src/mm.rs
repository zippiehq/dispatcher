use super::build_machine_id;
use super::configuration::{Concern, Configuration};
use super::dispatcher::{
    AddressField, BoolArray, Bytes32Array, Bytes32Field, FieldType,
    String32Field, U256Array, U256Array5, U256Field,
};
use super::dispatcher::{
    Archive, DApp, Reaction, SampleRequest, SampleStepRequest,
};
use super::emulator::{emu, Operation};
use super::error::Result;
use super::error::*;
use super::ethabi::Token;
use super::ethereum_types::{Address, H256, U256};
use super::serde::de::Error as SerdeError;
use super::serde::{Deserialize, Deserializer, Serializer};
use super::serde_json::Value;
use super::state::Instance;
use super::transaction::TransactionRequest;
use super::Role;

use std::collections::HashSet;

pub struct MM();

// !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
// these two structs and the From trait below shuld be
// obtained from a simple derive
// !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
#[derive(Serialize, Deserialize)]
pub struct MMCtxParsed(
    pub AddressField,  // provider
    pub AddressField,  // client
    pub Bytes32Field,  // initialHash
    pub Bytes32Field,  // newHash
    pub U256Field,     // historyLength
    pub String32Field, // currentState
);

#[derive(Debug)]
pub struct MMCtx {
    pub provider: Address,
    pub client: Address,
    pub initial_hash: H256,
    pub final_hash: H256,
    pub history_length: U256,
    pub current_state: String,
}

impl From<MMCtxParsed> for MMCtx {
    fn from(parsed: MMCtxParsed) -> MMCtx {
        MMCtx {
            provider: parsed.0.value,
            client: parsed.1.value,
            initial_hash: parsed.2.value,
            final_hash: parsed.3.value,
            history_length: parsed.4.value,
            current_state: parsed.5.value,
        }
    }
}

impl DApp<U256> for MM {
    fn react(
        instance: &state::Instance,
        archive: &Archive,
        divergence_time: &U256,
    ) -> Result<Reaction> {
        let parsed: MMCtxParsed = serde_json::from_str(&instance.json_data)
            .chain_err(|| {
                format!(
                    "Could not parse mm instance json_data: {}",
                    &instance.json_data
                )
            })?;
        let ctx: MMCtx = parsed.into();

        trace!("Context for mm {:?}", ctx);

        // should not happen as it indicates an innactive instance,
        // but it is possible that the blockchain state changed between queries
        match ctx.current_state.as_ref() {
            "FinishedReplay" => {
                return Ok(Reaction::Idle);
            }
            _ => {}
        };

        match ctx.current_state.as_ref() {
            "WaitingProofs" => {
                // machine id
                let id = build_machine_id(
                    instance.index,
                    &instance.concern.contract_address,
                );
                trace!("Calculating step of machine {}", id);
                // have we steped this machine yet?
                if let Some(samples) = archive.get(&id) {
                    // take the step samples (not the run samples)
                    let step_samples = &samples.step;
                    // have we sampled the divergence time?
                    if let Some(step_log) = step_samples.get(divergence_time) {
                        // if all proofs have been inserted, finish proof phase
                        if ctx.history_length.as_usize() >= step_log.len() {
                            let request = TransactionRequest {
                                concern: instance.concern.clone(),
                                value: U256::from(0),
                                function: "finishProofPhase".into(),
                                // !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
                                // improve these types by letting the
                                // dapp submit ethereum_types and convert
                                // them inside the transaction manager
                                // !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
                                data: vec![Token::Uint(instance.index)],
                                strategy: transaction::Strategy::Simplest,
                            };
                            return Ok(Reaction::Transaction(request));
                        }

                        // otherwise, submit one more proof step
                        let access =
                            (&step_log[ctx.history_length.as_usize()]).clone();
                        let siblings = access
                            .proof
                            .siblings
                            .into_iter()
                            .map(|array| Token::Uint(U256::from(array)))
                            .collect();
                        match access.operation {
                            Operation::Read => {
                                let request = TransactionRequest {
                                    concern: instance.concern.clone(),
                                    value: U256::from(0),
                                    function: "proveWrite".into(),
                                    // !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
                                    // improve these types by letting the
                                    // dapp submit ethereum_types and convert
                                    // them inside the transaction manager
                                    // !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
                                    data: vec![
                                        Token::Uint(instance.index),
                                        Token::Uint(U256::from(
                                            u64::from_be_bytes(access.address),
                                        )),
                                        Token::Uint(U256::from(
                                            u64::from_be_bytes(
                                                access.value_before,
                                            ),
                                        )),
                                        Token::Uint(U256::from(
                                            u64::from_be_bytes(
                                                access.value_after,
                                            ),
                                        )),
                                        Token::Array(siblings),
                                    ],
                                    strategy: transaction::Strategy::Simplest,
                                };
                                return Ok(Reaction::Transaction(request));
                            }
                            Operation::Write => {
                                let request = TransactionRequest {
                                    concern: instance.concern.clone(),
                                    value: U256::from(0),
                                    function: "proveRead".into(),
                                    // !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
                                    // improve these types by letting the
                                    // dapp submit ethereum_types and convert
                                    // them inside the transaction manager
                                    // !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
                                    data: vec![
                                        Token::Uint(instance.index),
                                        Token::Uint(U256::from(
                                            u64::from_be_bytes(access.address),
                                        )),
                                        Token::Uint(U256::from(
                                            u64::from_be_bytes(
                                                access.value_before,
                                            ),
                                        )),
                                        Token::Array(siblings),
                                    ],
                                    strategy: transaction::Strategy::Simplest,
                                };
                                return Ok(Reaction::Transaction(request));
                            }
                        }
                    }
                };
                // divergence proof has not been calculated yet, request it
                let sample_time: U256 = U256::from(*divergence_time);
                return Ok(Reaction::Step(SampleStepRequest {
                    id: id,
                    time: divergence_time.clone(),
                }));
            }
            _ => {}
        }

        return Ok(Reaction::Idle);
    }
}
