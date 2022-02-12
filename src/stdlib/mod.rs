use std::convert::TryFrom;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum StdLibKind {
    RegExp,
    Http,
    WebSocket,
    File,
    Timer,
    Channel
}

impl TryFrom<&str> for StdLibKind {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "regexp" => Result::Ok(StdLibKind::RegExp),
            "http" => Result::Ok(StdLibKind::Http),
            "websocket" => Result::Ok(StdLibKind::WebSocket),
            "file" => Result::Ok(StdLibKind::File),
            "timer" => Result::Ok(StdLibKind::Timer),
            "channel" => Result::Ok(StdLibKind::Channel),
            unknown => Result::Err(format!("unknown std lib name {:?}",unknown)),
        }
    }
}