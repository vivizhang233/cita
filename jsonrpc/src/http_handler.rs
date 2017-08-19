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

#![allow(deprecated,unused_assignments, unused_must_use)]

use base_hanlder::{BaseHandler, RpcResult};

use base_hanlder::Handler;
use futures::{BoxFuture, Future};
use futures::future::FutureResult;
use futures::sync::oneshot;

use hyper;
use hyper::Post;
use hyper::header::ContentLength;
use hyper::server::{Handler, Request, Response};
use hyper::server::{Http, Service, NewService, Request, Response};
use hyper::server::Server;
use hyper::uri::RequestUri::AbsolutePath;


use jsonrpc_types::error::Error;
use jsonrpc_types::method;
use jsonrpc_types::response::{self as cita_response, RpcSuccess, RpcFailure};
use libproto::{blockchain, request};
use libproto::communication;
use parking_lot::{RwLock, Mutex};
use protobuf::Message;
use serde_json;
use std::cmp::Eq;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::io::Read;
use std::result;
use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;
use util::H256;

impl NewService for Handler {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Instance = HttpHandler;
    fn new_service(&self) -> io::Result<Handler> {
        Ok(self.clone())
    }
}

impl Service for Handler {
	type Request = Request;
	type Response = Response;
	type Error = hyper::Error;
	type Future = BoxFuture<Response, hyper::Error>;
	
	fn call(&self, req: Request) -> Self::Future {
		let (tx, rx) = oneshot::channel();
		let this = self.clone();
		// self.send_pool(||{
		//	this.deal_http_req();
		//
		// });
		rx.map_err(|_| {
			//出错处理。以及错误转换。这个肯定是系统错误了。转化为hyper的系统错误。
			
			
		})
		  .boxed()
		
	}
}


impl Handler{
    pub fn deal_req(&mut self, tx: TransferSender, data: String) -> Result<(), Error> {

        let req_id = Id::Null;
        let jsonrpc_version = None;
        match HttpHandler::into_json(data) {
            Err(err) => Err(err),
            Ok(rpc) => {
                let req_id = rpc.id.clone();
                let jsonrpc_version = rpc.jsonrpc.clone();
                let topic = WsHandler::select_topic(&rpc.method);
                let req_info = ReqInfo {
                    jsonrpc: jsonrpc_version.clone(),
                    id: req_id.clone(),
                };

                _self.method_handler.from_req(rpc).map(|req_type| {
                    match req_type {
                        method::RpcReqType::TX(tx_req) => {
                            //TODO key一定要唯一了。
                            let hash = tx_req.crypt_hash();
                            let data: communication::Message = tx_req.into();
                            let _ = _self.tx.send((topic, data.write_to_bytes().unwrap()));

                            _self.tx_responses.lock().insert(hash, (req_info, tx));
                        }

                        method::RpcReqType::REQ(_req) => {
                            //TODO 请求id唯一可以用数字标识了。整数，usize 类型了，只要保证它是唯一的。这样的话包括容器也可以唯一了。
                            let key = _req.request_id.clone();
                            let data: communication::Message = _req.into();
                            let _ = _self.tx.send((topic, data.write_to_bytes().unwrap()));
                            _self.responses.lock().insert(key, (req_info, tx));
                        }
                    }
                    ()
                })
            }
        }

    }
}


impl Handler {
	
    pub fn pase_url(&self, mut req: Request) -> Result<String, Error> {
        let uri = req.uri.clone();
        let method = req.method.clone();
        match uri {
            AbsolutePath(ref path) => {
                match (&method, &path[..]) {
                    (&Post, "/") => {
                        let mut body = String::new();
                        match req.read_to_string(&mut body) {
                            Ok(_) => Ok(body),
                            Err(_) => Err(Error::invalid_request()),//TODO
                        }
                    }
                    _ => result::Result::Err(Error::invalid_request()),
                }
            }
            _ => result::Result::Err(Error::invalid_request()),
        }
    }


    pub fn deal_req(&self, post_data: String) -> Result<RpcSuccess, RpcFailure> {
        match HttpHandler::into_json(post_data) {
            Err(err) => Err(RpcFailure::from(err)),
            Ok(rpc) => {
                let req_id = rpc.id.clone();
                let jsonrpc_version = rpc.jsonrpc.clone();
                let topic = HttpHandler::select_topic(&rpc.method);
                match self.method_handler.from_req(rpc)? {
                    method::RpcReqType::TX(tx) => {
                        let hash = tx.crypt_hash();
                        self.send_mq(topic, tx.into(), self.tx_responses.clone(), hash)
                            .map_err(|err_data| RpcFailure::from_options(req_id.clone(), jsonrpc_version.clone(), err_data))
                            .map(|data| {
                                     RpcSuccess {
                                         jsonrpc: jsonrpc_version,
                                         id: req_id,
                                         result: cita_response::ResponseBody::from(data), //TODO
                                     }
                                 })
                    }
                    method::RpcReqType::REQ(req) => {
                        let key = req.request_id.clone();
                        self.send_mq(topic, req.into(), self.responses.clone(), key)
                            .map_err(|err_data| RpcFailure::from_options(req_id.clone(), jsonrpc_version.clone(), err_data))
                            .map(|data| {
                                     RpcSuccess {
                                         jsonrpc: jsonrpc_version,
                                         id: req_id,
                                         result: cita_response::ResponseBody::from(data.result.expect("chain response error")), //TODO
                                     }
                                 })
                    }
                }
            }
        }
    }
	
	
}
