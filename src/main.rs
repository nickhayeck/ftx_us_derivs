use ftx_us_derivs::{
    table::{ContractSpec, ContractSpecTable},
    ws::{WebSocketClient, WebSocketMsg},
};

pub fn main() {
    let mut c = WebSocketClient::connect("wss://api.ledgerx.com/ws").unwrap();

    let ct = ContractSpecTable::build().unwrap();

    for _ in 0..25 {
        let msg = c.yield_msg().unwrap();
        match msg {
            WebSocketMsg::BookTop(bt) => {
                let contract = ct.id_table[&bt.contract_id].as_ref();
                if let ContractSpec::Option(opt) = contract {
                    println!(
                        "{}: {} @ {}, {}x{}",
                        opt.label, bt.bid, bt.ask, bt.bid_size, bt.ask_size
                    );
                }
            }
            _ => {}
        };
    }
}
