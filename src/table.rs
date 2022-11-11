use chrono::TimeZone;
use chrono::{offset::Utc, DateTime};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::rc::Rc;

use crate::error::WebSocketError;
use crate::ws::{RawMsg, SanitizableMsg};

fn parse_ftx_datetime(dt: &str) -> DateTime<Utc> {
    Utc.datetime_from_str(dt, "%Y-%m-%d %H:%M:%S%z").unwrap()
}

fn years_til_strfdt(dt: &str) -> f64 {
    let datetime = parse_ftx_datetime(dt);
    let now = Utc::now();
    let tte = datetime.timestamp() - now.timestamp();
    return tte as f64 / 31556926.0;
}

//
// SANITIZED TABLES
//

#[derive(Debug, Clone)]
pub struct ContractSpecTable {
    pub id_table: HashMap<u64, Rc<ContractSpec>>,
    pub label_table: HashMap<String, Rc<ContractSpec>>,
}

impl ContractSpecTable {
    pub fn build() -> Result<Self, WebSocketError> {
        Ok(RawContractSpecTable::build()?.sanitize())
    }
}

#[derive(Debug, Clone)]
pub enum ContractSpec {
    Future(FutureContractSpec),
    Option(OptionContractSpec),
    Swap(SwapSpec),
}

impl ContractSpec {
    pub fn as_opt(self) -> Option<OptionContractSpec> {
        match self {
            ContractSpec::Option(o) => Some(o),
            _ => None,
        }
    }
    pub fn as_opt_ref(&self) -> Option<&OptionContractSpec> {
        match self {
            ContractSpec::Option(o) => Some(o),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OptionContractSpec {
    // identifiers
    pub id: u64,
    pub label: String,
    // contract specs
    pub underlying: String,
    pub strike_price: u64,
    pub is_call: bool,
    pub tte: f64, // annualized
    pub open_interest: u32,
    // contract specs pt.2
    pub multiplier: f64,
    pub min_increment: f64,
    // auxilliary data
    pub active: bool,
    pub date_live: DateTime<Utc>,
    pub date_expires: DateTime<Utc>,
    pub collateral_asset: String,
    // wtf is this
    pub is_ecp_only: bool,
}

// build these below ones out as needed
#[derive(Debug, Clone)]
pub struct FutureContractSpec(RawContractSpec);

#[derive(Debug, Clone)]
pub struct SwapSpec(RawContractSpec);

//
// NON-SANITIZED TABLES
//

#[derive(Debug, Serialize, Deserialize)]
pub struct RawContractSpecTable {
    pub data: Vec<RawContractSpec>,
}

impl RawContractSpecTable {
    pub fn build() -> Result<Self, WebSocketError> {
        let resp = ureq::get("https://api.ledgerx.com/trading/contracts")
            .set("Accept", "application/json")
            .call()
            .unwrap();
        let resp_string = resp.into_string().unwrap();
        RawContractSpecTable::parse(&resp_string)
    }
}

impl<'a> SanitizableMsg<'a> for RawContractSpecTable {
    type OUT = ContractSpecTable;
    fn sanitize(self) -> Self::OUT {
        let mut out = ContractSpecTable {
            id_table: HashMap::new(),
            label_table: HashMap::new(),
        };

        for i in self.data.into_iter() {
            let out_i = i.id;
            let out_s = i.label.clone();

            let out_c = match i.derivative_type.as_str() {
                "day_ahead_swap" => ContractSpec::Swap(SwapSpec(i)),
                "future_contract" => ContractSpec::Future(FutureContractSpec(i)),
                "options_contract" => ContractSpec::Option(OptionContractSpec {
                    id: i.id,
                    label: i.label,

                    underlying: i.underlying_asset,
                    strike_price: (i.strike_price.unwrap() / 100) as u64,
                    is_call: i.is_call.unwrap(),
                    tte: years_til_strfdt(&i.date_expires),
                    open_interest: i.open_interest.unwrap_or(0),

                    multiplier: i.multiplier as f64,
                    min_increment: i.min_increment as f64 / 100.0,

                    active: i.active,
                    date_live: parse_ftx_datetime(&i.date_live),
                    date_expires: parse_ftx_datetime(&i.date_expires),
                    collateral_asset: i.collateral_asset,

                    is_ecp_only: i.is_ecp_only,
                }),
                _ => {
                    unimplemented!()
                }
            };

            let out_p = Rc::new(out_c);

            out.id_table.insert(out_i, out_p.clone());
            out.label_table.insert(out_s, out_p.clone());
        }

        return out;
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RawContractSpec {
    pub id: u64,
    pub label: String,
    pub is_call: Option<bool>,
    pub active: bool,
    pub strike_price: Option<u32>,
    pub min_increment: u32,
    pub date_live: String,
    pub date_expires: String,
    pub date_exercise: Option<String>,
    pub underlying_asset: String,
    pub collateral_asset: String,
    pub derivative_type: String,
    pub open_interest: Option<u32>,
    pub is_next_day: bool,
    pub multiplier: u32,
    pub is_ecp_only: bool,
}
