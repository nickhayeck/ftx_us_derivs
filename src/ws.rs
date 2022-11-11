use serde::{Deserialize, Serialize};
use websocket::client::sync::Client;
use websocket::stream::sync::NetworkStream;
use websocket::ClientBuilder;
use websocket::OwnedMessage;

use crate::error::WebSocketError;

pub struct WebSocketClient<'a> {
    // config
    endpoint: &'a str,
    exhaustion_counter: u64,

    // state
    client: Client<Box<dyn NetworkStream + Send>>,
    last_clock: u64,
}

impl<'a> WebSocketClient<'a> {
    pub fn connect(endpoint: &'a str) -> Result<Self, WebSocketError> {
        let client = ClientBuilder::new(endpoint)?.connect(None)?;
        Ok(WebSocketClient {
            endpoint,
            exhaustion_counter: 0,
            client,
            last_clock: 0,
        })
    }
    pub fn yield_msg(&mut self) -> Result<WebSocketMsg, WebSocketError> {
        let web_msg = self.client.recv_message()?;
        
        self.respond_if_ping(WebSocketMsgParser::parse(&web_msg))
    }

    fn respond_if_ping(&mut self, msg: Result<WebSocketMsg, WebSocketError>) -> Result<WebSocketMsg, WebSocketError> {
        if let Ok(inner) = &msg {
            if let WebSocketMsg::Ping(data) = inner {
                // println!("Got Ping:\t{:?}", data);
                self.client.send_message(&websocket::OwnedMessage::Pong(data.to_owned()))?;
                // println!("Sent Pong:\t{:?}", data);
            }
        }
        return msg;
    }
}

#[derive(Debug)]
pub enum WebSocketMsg {
    Ping(Vec<u8>),
    Pong,
    BookTop(BookTop),
    HeartBeat(RawHeartbeat),
    UnAuthSuccess,
    SessionID(String),
}

pub struct WebSocketMsgParser();
impl WebSocketMsgParser {
    pub fn parse(msg: &OwnedMessage) -> Result<WebSocketMsg, WebSocketError> {
        match msg {
            websocket::OwnedMessage::Text(s) => {
                if let Some(_i) = s.find("\"type\": \"book_top\"") {
                    return Ok(WebSocketMsg::BookTop(RawBookTop::parse(s)?.sanitize()));
                } else if let Some(_i) = s.find("\"type\": \"heartbeat\"") {
                    return Ok(WebSocketMsg::HeartBeat(RawHeartbeat::parse(s)?));
                } else if let Some(_i) = s.find("\"type\": \"unauth_success\"") {
                    return Ok(WebSocketMsg::UnAuthSuccess);
                } else if let Some(_i) = s.find("\"type\": \"meta\"") {
                    return Ok(WebSocketMsg::SessionID("unimplemented lol".to_string()));
                }

                return Err(WebSocketError::UnknownMsgType(s.to_string()));
            },
            websocket::OwnedMessage::Ping(data) => {
                return Ok(WebSocketMsg::Ping(data.to_owned()));
            },
            websocket::OwnedMessage::Pong(_) => {
                return Ok(WebSocketMsg::Pong);
            },
            _ => {
                println!("{:?}", msg);
                unimplemented!("unimplemented OwnedMessage type");
            }
        }
    }
}

pub trait RawMsg<'a>
where
    Self: Sized + Deserialize<'a> + std::fmt::Debug,
{
    fn parse(inp: &'a str) -> Result<Self, WebSocketError> {
        serde_json::from_str(inp)
            .map_err(|e| WebSocketError::MsgParsing(e.to_string(), e.line(), e.column()))
    }
}
impl<'a, T> RawMsg<'a> for T where T: Sized + Deserialize<'a> + std::fmt::Debug {}

pub trait SanitizableMsg<'a>
where
    Self: Sized + RawMsg<'a>,
{
    type OUT;
    fn sanitize(self) -> Self::OUT;
}

//
// WS MSG Definitions
//

#[derive(Debug, Serialize, Deserialize)]
pub struct RawHeartbeat {
    timestamp: u64,
    ticks: u64,
    run_id: u64,
    interval_ms: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RawBookTop {
    bid: u64,
    bid_size: u64,

    ask: u64,
    ask_size: u64,

    contract_id: u64,
    contract_type: u64,

    clock: u64,
}
#[derive(Debug)]
pub struct BookTop {
    pub bid: f64,
    pub bid_size: u64,

    pub ask: f64,
    pub ask_size: u64,

    pub contract_id: u64,
    pub contract_type: u64,

    pub clock: u64,
}

impl<'a> SanitizableMsg<'a> for RawBookTop {
    type OUT = BookTop;
    fn sanitize(self) -> Self::OUT {
        BookTop {
            bid: (self.bid as f64) / 100.0,
            bid_size: self.bid_size,

            ask: (self.ask as f64) / 100.0,
            ask_size: self.ask_size,

            contract_id: self.contract_id,
            contract_type: self.contract_type,

            clock: self.clock,
        }
    }
}

pub struct RawOrderResponse {}
pub struct RawBookState {}
pub struct RawPositionList {}
