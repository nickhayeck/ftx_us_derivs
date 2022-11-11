use websocket::url::ParseError;

#[derive(Debug)]
pub enum WebSocketError {
    ConnectionError(String),
    MsgParsing(String, usize, usize),
    UnknownMsgType(String),
    Misc(String),
}

impl From<ParseError> for WebSocketError {
    fn from(f: ParseError) -> Self {
        WebSocketError::ConnectionError(f.to_string())
    }
}
impl From<websocket::WebSocketError> for WebSocketError {
    fn from(f: websocket::WebSocketError) -> Self {
        WebSocketError::ConnectionError(format!("connecting failed!\n{}", f))
    }
}

pub enum TableError {
    ClientError(u16, String),
}

pub enum OrderError {}
