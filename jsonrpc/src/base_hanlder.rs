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
use jsonrpc_types::error::Error;
use jsonrpc_types::request::RpcRequest;
use libproto::communication;
use num_cpus;
use parking_lot::Mutex;
use protobuf::Message;
use serde_json;
use serde_json;
use std::collections::HashMap;
use std::result;
use std::sync::Arc;
use std::sync::mpsc::Sender;
use threadpool::ThreadPool;
use util::hash::H256;
use ws;
use ws::{Factory, CloseCode, Handler};



pub type RpcResult<T> = result::Result<T, Error>;

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

#[derive(Debug, Clone, PartialEq)]
pub enum TransferType {
    ALL,
    HTTP,
    WEBSOCKET,
}


impl Default for TransferType {
    fn default() -> TransferType {
        TransferType::ALL
    }
}


use futures::sync::oneshot::Sender as HttpSender;
use ws::Sender as WsSender;



//Inner是同一个，唯一的一个，而线程池应该也是同一个，但是线程池不能和它作为一个
#[derive(Clone)]
struct Inner {
    tx_responses: Mutex<HashMap<H256, (ReqInfo, ws::Sender)>>,
    responses: Mutex<HashMap<Vec<u8>, (ReqInfo, ws::Sender)>>,
}

//这是一个客户的请求产生一个。
//这里可以看到，其实mq,http,ws都是指向了同一结构体。因此我们可以利用通道，指向同一个结构体，
//线程池就不需要枷锁了。整个jsonrpc就一个线程池。
//
//多个线程访问同一个线程池。

use std::collections::HashMap;


#[derive(Clone)]
struct FactoryHandler {
    inner: Arc<Inner>, //固定
	tx_mq: Option<Sender<(String, Vec<u8>)>>, //必须有的。固定。
}



#[derive(Clone)]
struct Handler {
    inner: Arc<Inner>, //固定
    tx_mq: Option<Sender<(String, Vec<u8>)>>, //必须有的。固定。

    //上面是Factory的。下面是，或者请求的。
    //	tx_mq: Option<Sender<(String, Vec<u8>)>>, //固定不变，但是必须有的。且是复制的。
    tx_http: Option<HttpSender>, //这个不是固定的，是随机改变的。
    tx_ws: Option<WsSender>, //这个也是随机改变的。
}


impl Default for Handler {
    fn default() -> Handler {
        Handler {
            inner: Arc::new(Inner {
                                responses: Mutex::new(HashMap::capacity(1000)),
                                tx_responses: Mutex::new(HashMap::capacity(1000)),
                            }),
            tx_mq: None,
            tx_http: None,
            tx_ws: None,
        }
    }
}

impl Handler {
    pub fn set_tx_mq(&mut self, tx_mq: Sender<(String, Vec<u8>)>) {
        self.tx_mq = Some(tx_mq);
    }

    pub fn set_tx_http(&mut self, tx_http: HttpSender) {
        self.tx_http = Some(tx_http);
    }

    pub fn set_tx_ws(&mut self, tx_ws: WsSender) {
        self.tx_ws = Some(tx_ws);
    }
}


impl BaseHandler for Handler {}



#[derive(Clone)]
pub enum TransferSender {
    HTTP(HttpSender),
    WS(WsSender),
}

impl TransferSender {
    pub fn send<T>(self, t: T) {
        match self {
            TransferSender::HTTP(sender) => {
                //TODO 错误转换。
                sender.send(t);
            }
            TransferSender::WS(sender) => {
                //TODO 错误转换。
                sender.send(t);
            }

        }
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
