pub mod handler;

use crate::handler::BasicHandler;

use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct Params(BTreeMap<String, String>);

pub struct App {
    #[allow(dead_code)] // FIXME remove
    routes: BTreeMap<String, BasicHandler<hyper::Body, hyper::Body>>,
}
