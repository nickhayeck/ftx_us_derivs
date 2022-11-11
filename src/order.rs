use std::str::FromStr;

use serde::Deserialize;
use ureq::Agent;

// EXAMPLE BASE URL: https://trade.ledgerx.com/api

/// thin wrapper for `ureq::Agent` that contains the order history + configuration info
/// (api_key, base_url, etc.). pass this as a reference in "mngr" functions
pub struct OrderMngr<'a> {
    // config stuff
    base_url: &'a str,
    api_key: String,

    // http client
    agent: Agent,

    // history
    pub order_history: Vec<(OrderResponse, Order)>,
}

impl<'a> OrderMngr<'a> {
    pub fn new(base_url: &'a str, api_key: &'a str) -> Self {
        OrderMngr {
            base_url,
            api_key: format!("JWT {}", api_key),
            agent: Agent::new(),
            order_history: Vec::new(),
        }
    }
    fn append(&mut self, resp: &OrderResponse, ord: &Order) {
        self.order_history.push((resp.clone(), ord.to_owned()));
    }
    fn send<T>(&self, action: &T) -> Result<T::OkType, ureq::Error>
    where
        T: SendWithMngr,
    {
        let out = action.send_with_mngr(&self);

        return out;
    }
    pub fn send_order(&mut self, ord: &Order) -> Result<OrderResponse, ureq::Error> {
        let out = self.send(ord);
        if let Ok(o) = out {
            self.append(&o, ord);
            return Ok(o);
        }
        return out;
    }
    pub fn send_edit(&mut self, edit: &OrderEdit) -> Result<(), ureq::Error> {
        self.send(edit)
    }
    pub fn send_cancel(&mut self, cancel: &Cancel) -> Result<(), ureq::Error> {
        self.send(cancel)
    }
}

/// Sends the order using a pre-defined and pre-stored OrderManager, this is preferred
/// as the TCP connection can be recycled for later use + we can save config info.
pub trait SendWithMngr {
    type OkType;
    /// Sends the order using a pre-defined and pre-stored OrderManager, this is preferred
    /// as the TCP connection can be recycled for later use + we can save config info.
    fn send_with_mngr(&self, mngr: &OrderMngr) -> Result<Self::OkType, ureq::Error>;
}

#[derive(Debug, Clone)]
pub struct Order {
    pub order_type: String,
    pub contract_id: u64,
    pub is_ask: bool,
    pub swap_purpose: String,
    pub size: u64,
    pub price: u64,
    pub volatile: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub struct OrderResponse {
    #[serde(rename = "mid")]
    pub order_id: String,
}

impl Order {
    pub fn new(contract_id: u64, is_ask: bool, price: f64, size: u64) -> Self {
        Order {
            order_type: String::from_str("limit").unwrap(),
            contract_id,
            is_ask,
            swap_purpose: String::from_str("undisclosed").unwrap(),
            size,
            price: (price as u64) * 100,
            volatile: false,
        }
    }
    /// Denotes whether this trade is a bona-fide hedge or not (optional)
    pub fn swap_purpose(&mut self, arg: &str) {
        self.swap_purpose = String::from_str(arg).unwrap();
    }
    /// Specifies whether or not an order should auto-cancel at 4pm (optional)
    pub fn auto_cancel(&mut self, arg: bool) {
        self.volatile = arg;
    }
}

impl SendWithMngr for Order {
    type OkType = OrderResponse;
    fn send_with_mngr(&self, mngr: &OrderMngr) -> Result<OrderResponse, ureq::Error> {
        let path = format!("{}/orders", mngr.base_url);

        let data = format!(
            "{{
                \"order_type\": \"{}\",
                \"contract_id\": {},
                \"is_ask\": {},
                \"swap_purpose\": \"{}\",
                \"size\": {},
                \"price\": {},
                \"volatile\": {}
           }}",
            self.order_type,
            self.contract_id,
            self.is_ask,
            self.swap_purpose,
            self.size,
            self.price,
            self.volatile,
        );

        let resp = mngr
            .agent
            .post(&path)
            .set("Authorization", &mngr.api_key)
            .set("accept", "application/json")
            .set("content-type", "application/json")
            .send_string(&data)?;

        let ord_resp: OrderResponse = serde_json::from_reader(resp.into_reader()).unwrap();
        Ok(ord_resp)
    }
}

pub struct OrderEdit {
    order_id: String,
    contract_id: u64,
    price: u64,
    size: u64,
}
impl OrderEdit {
    pub fn new(order_id: String, contract_id: u64, price: u64, size: u64) -> Self {
        OrderEdit {
            order_id,
            contract_id,
            price,
            size,
        }
    }
}

impl SendWithMngr for OrderEdit {
    type OkType = ();
    fn send_with_mngr(&self, mngr: &OrderMngr) -> Result<(), ureq::Error> {
        let path = &format!("{}/orders/{}/edit", mngr.base_url, self.order_id,);

        let data = format!(
            "{{
                \"contract_id\": {},
                \"size\": {},
                \"price\": {},
           }}",
            self.contract_id, self.size, self.price,
        );

        let resp = mngr
            .agent
            .post(path)
            .set("Authorization", &mngr.api_key)
            .set("Accept", "application/json")
            .set("content-type", "application/json")
            .send_string(&data)
            .and(Ok(()));

        return resp;
    }
}

pub struct Cancel(Option<(String, u64)>);

impl Cancel {
    pub fn one(order_id: String, contract_id: u64) -> Self {
        Cancel(Some((order_id, contract_id)))
    }
    pub fn all() -> Self {
        Cancel(None)
    }
}

impl SendWithMngr for Cancel {
    type OkType = ();
    fn send_with_mngr(&self, mngr: &OrderMngr) -> Result<(), ureq::Error> {
        if let Some((order_id, contract_id)) = &self.0 {
            let path = &format!("{}/orders/{}", mngr.base_url, order_id);

            let data = format!(
                "{{
                    \"contract_id\": {},
               }}",
                contract_id,
            );

            let resp = mngr
                .agent
                .delete(path)
                .set("Authorization", &mngr.api_key)
                .set("Accept", "application/json")
                .set("content-type", "application/json")
                .send_string(&data)
                .and(Ok(()));

            return resp;
        } else {
            let path = &format!("{}/orders", mngr.base_url);

            let resp = mngr
                .agent
                .delete(path)
                .set("Authorization", &mngr.api_key)
                .set("Accept", "application/json")
                .call()
                .and(Ok(()));

            return resp;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const API_KEY: &str = ""; //env!("LEDGERX_TEST_API_KEY");

    fn setup<'a>() -> OrderMngr<'a> {
        OrderMngr::new("https://trade.ledgerx.com/api", API_KEY)
    }
    #[test]
    fn setup_om() {
        setup();
    }

    #[test]
    fn place_order() {
        let mut om = setup();
        let out = Order::new(22252392, false, 1.0, 1).send_with_mngr(&mut om);
        println!(
            "{:?}",
            out.or_else(|x| { Err(x.into_response().unwrap().into_string().unwrap()) })
        );
    }

    #[test]
    fn cancel_all() {
        let mut om = setup();
        let out = Cancel::all().send_with_mngr(&mut om);

        println!("{:?}", out);
    }
}
