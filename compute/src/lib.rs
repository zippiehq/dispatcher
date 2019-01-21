#![feature(proc_macro_hygiene, generators, transpose_result)]

pub mod compute;
pub mod vg;

extern crate configuration;
extern crate env_logger;
extern crate error;

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;
extern crate dispatcher;
extern crate ethabi;
extern crate ethereum_types;
extern crate hex;
extern crate serde;
extern crate serde_json;
extern crate state;
extern crate transaction;

use configuration::{Concern, Configuration};
use dispatcher::{
    AddressField, Bytes32Field, FieldType, String32Field, U256Field,
};
use dispatcher::{Archive, DApp, Reaction, SampleRequest, Samples};
use error::Result;
use error::*;
use ethabi::Token;
use ethereum_types::{Address, H256, U256};
use serde::de::Error as SerdeError;
use serde::{Deserialize, Deserializer, Serializer};
use serde_json::Value;
use state::Instance;

pub use compute::Compute;
pub use vg::VG;

#[derive(Debug)]
enum Role {
    Claimer,
    Challenger,
}
