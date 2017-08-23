// CITA
// Copyright 2016-2017 Cryptape Technologies LLC.

// This program is free software: you can redistribute it
// and/or modify it under the terms of the GNU General Public
// License as published by the Free Software Foundation,
// either version 3 of the License, or (at your option) any
// later version.

// This program is distributed in the hope that it will be
// useful, but WITHOUT ANY WARRANTY; without even the implied
// warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR
// PURPOSE. See the GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

//#![allow(deprecated,unused_assignments, unused_must_use)]

use futures::sync::oneshot::Sender as HttpSender;
//use hyper::header::ContentLength;
use hyper::server::Response;
use jsonrpc_types::{method, Version, Id, RpcRequest};
use jsonrpc_types::error::Error;
use jsonrpc_types::method::MethodHandler;
use jsonrpc_types::response::{ResponseBody, RpcSuccess, RpcFailure};
use libproto::{communication, submodules, topics, parse_msg, cmd_id, display_cmd, MsgClass};
use parking_lot::Mutex;
use protobuf::Message;
use serde_json;
use std::collections::HashMap;
use std::convert::Into;
use std::result;
use std::sync::Arc;
use std::sync::mpsc::Sender;
use ws::Sender as WsSender;


pub type RpcResult<T> = result::Result<T, Error>;

pub trait FnBox {
    fn call_box(self: Box<Self>);
}

impl<F: FnOnce()> FnBox for F {
    fn call_box(self: Box<F>) {
        (*self)()
    }
}

type Thunk<'a> = Box<FnBox + Send + 'a>;

pub trait BaseHandler {
    fn select_topic(method: &String) -> String {
        let topic = if method.starts_with("cita_send") {
                        "jsonrpc.new_tx"
                    } else if method.starts_with("cita") || method.starts_with("eth") {
                        "jsonrpc.request"
                    } else if method.starts_with("net_") {
                        "jsonrpc.net"
                    } else {
                        "jsonrpc"
                    }
                    .to_string();
        topic
    }

    fn into_json(body: String) -> Result<RpcRequest, Error> {
        let rpc: Result<RpcRequest, serde_json::Error> = serde_json::from_str(&body);
        match rpc {
            Err(_err_msg) => Err(Error::parse_error()),
            Ok(rpc) => Ok(rpc),
        }
    }
}


pub enum Senders {
    HTTP(HttpSender<Response>),
    WEBSOCKET(WsSender),
}

impl Senders {
    pub fn send(self, data: String) {
        match self {
            Senders::HTTP(sender) => {
                let mut res = Response::new();
                //TODO
                res.set_body(data);
                sender.send(res);
            }
            Senders::WEBSOCKET(sender) => {
                sender.send(data);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReqInfo {
    pub jsonrpc: Option<Version>,
    pub id: Id,
}

unsafe impl Send for ReqInfo {}

impl ReqInfo {
    pub fn new(jsonrpc: Option<Version>, id: Id) -> ReqInfo {
        ReqInfo { jsonrpc: jsonrpc, id: id }
    }
}

#[derive(Clone)]
pub struct RpcHandler {
    responses: Arc<Mutex<HashMap<Vec<u8>, (ReqInfo, Senders)>>>,
    tx_responses: Arc<Mutex<HashMap<Vec<u8>, (ReqInfo, Senders)>>>,

    pub tx_mq: Option<Sender<(String, Vec<u8>)>>,
    pub tx_pool: Option<Sender<Thunk<'static>>>,
    //    pub tx_http: Cell<Option<HttpSender<Response>>>,
    pub tx_ws: Option<WsSender>,
}

unsafe impl Send for RpcHandler {}

unsafe impl Sync for RpcHandler {}

impl Default for RpcHandler {
    fn default() -> RpcHandler {
        RpcHandler {
            responses: Arc::new(Mutex::new(HashMap::with_capacity(1000 as usize))),
            tx_responses: Arc::new(Mutex::new(HashMap::with_capacity(1000 as usize))),
            tx_mq: None,
            tx_ws: None,
            tx_pool: None,
        }
    }
}

impl RpcHandler {
    pub fn set_tx_pool(&mut self, tx_pool: Sender<Thunk<'static>>) {
        self.tx_pool = Some(tx_pool);
    }

    pub fn set_tx_mq(&mut self, tx_mq: Sender<(String, Vec<u8>)>) {
        self.tx_mq = Some(tx_mq);
    }

    pub fn set_tx_ws(&mut self, tx_ws: WsSender) {
        self.tx_ws = Some(tx_ws);
    }
}


impl BaseHandler for RpcHandler {}

pub fn from_err(err: Error) -> String {
    serde_json::to_string(&RpcFailure::from(err)).unwrap()
}

impl RpcHandler {
    pub fn deal_req(&self, data: String, sender: Senders) {
        let req_id = Id::Null;
        let jsonrpc_version: Option<Version> = None;
        match RpcHandler::into_json(data) {
            Err(err) => sender.send(from_err(err)),
            Ok(rpc) => {
                let req_id = rpc.id.clone();
                let jsonrpc_version = rpc.jsonrpc.clone();
                let topic = RpcHandler::select_topic(&rpc.method);
                let req_info = ReqInfo {
                    jsonrpc: jsonrpc_version.clone(),
                    id: req_id.clone(),
                };
                let method_handler = MethodHandler;
                match method_handler.from_req(rpc) {
                    Err(err) => sender.send(from_err(err)),
                    Ok(req_type) => {
                        match req_type {
                            method::RpcReqType::TX(tx_req) => {
                                let hash = tx_req.crypt_hash().to_vec();
                                let data: communication::Message = tx_req.into();
                                let _ = self.tx_mq.as_ref().unwrap().send((topic, data.write_to_bytes().unwrap()));
                                self.tx_responses.lock().insert(hash.to_vec(), (req_info, sender));
                            }

                            method::RpcReqType::REQ(_req) => {
                                let key = _req.request_id.clone();
                                let data: communication::Message = _req.into();
                                let _ = self.tx_mq.as_ref().unwrap().send((topic, data.write_to_bytes().unwrap()));
                                self.responses.lock().insert(key, (req_info, sender));
                            }
                        }
                    }
                };
            }
        }
    }


    pub fn deal_res(&mut self, key: String, body: Vec<u8>) {
        let (id, _, content_ext) = parse_msg(body.as_slice());
        trace!("routint_key {:?},get msg cmid {:?}", key, display_cmd(id));
        let this = self.clone();
        //TODO match
        if id == cmd_id(submodules::CHAIN, topics::RESPONSE) {
            match content_ext {
                MsgClass::RESPONSE(content) => {
                    let req_id = content.request_id.clone();
                    self.deal_respone(req_id, content.result.unwrap(), self.responses.clone());
                }
                _ => {}
            }
        } else if id == cmd_id(submodules::CONSENSUS, topics::TX_RESPONSE) {
            match content_ext {
                MsgClass::TXRESPONSE(content) => {
                    let hash = content.hash.clone();
                    self.deal_respone(hash, content, self.responses.clone());
                }
                _ => {}
            }
        }
    }


    fn deal_respone<T: Into<ResponseBody> + Send + 'static>(&self, req_id: Vec<u8>, result: T, con: Arc<Mutex<HashMap<Vec<u8>, (ReqInfo, Senders)>>>) {
        self.tx_pool.as_ref().unwrap().send(Box::new(move || {
            let pair = con.lock().remove(&req_id);
            drop(con);
            if let Some(pair) = pair {
                let rpc_success = RpcSuccess {
                    jsonrpc: pair.0.jsonrpc.clone(),
                    id: pair.0.id.clone(),
                    result: result.into(),
                };
                let data = serde_json::to_string(&rpc_success).unwrap();
                pair.1.send(data);
            }
        }));
    }
}


#[cfg(test)]
mod test {
    use super::BaseHandler;

    struct Handler {}

    impl BaseHandler for Handler {}

    #[test]
    fn test_get_topic() {
        assert_eq!(Handler::select_topic(&"net_work".to_string()), "jsonrpc.net".to_string());
        assert_eq!(Handler::select_topic(&"cita_send".to_string()), "jsonrpc.new_tx".to_string());
        assert_eq!(Handler::select_topic(&"cita".to_string()), "jsonrpc.request".to_string());
        assert_eq!(Handler::select_topic(&"eth".to_string()), "jsonrpc.request".to_string());
        assert_eq!(Handler::select_topic(&"123".to_string()), "jsonrpc".to_string());
    }
}
