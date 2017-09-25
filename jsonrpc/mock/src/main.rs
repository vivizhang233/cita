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
#![allow(unused_variables, unused_extern_crates, unused_imports)]
extern crate pubsub;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate serde;
#[macro_use]
extern crate log;
extern crate cita_crypto as crypto;
extern crate libproto;
extern crate jsonrpc_types;
extern crate protobuf;

use libproto::{MsgClass, TxResponse, parse_msg, request, response, factory, submodules, communication, topics};
use protobuf::Message;

use pubsub::start_pubsub;
use std::sync::mpsc::channel;
use std::thread;
use std::time::{Duration, SystemTime};
const TX_NUM: usize = 50000;

///主要用于JsonRpc的性能测试,测试JsonRpc处理交易的性能

fn main() {
    let mut count = 0;
    let (tx_sub, rx_sub) = channel();
    let (tx_pub, rx_pub) = channel();
    start_pubsub("chain", vec!["jsonrpc.request", "jsonrpc.new_tx"], tx_sub, rx_pub);
    let mut is_stop = false;
    println!("start loop wait for request");
    loop {
        let mut start = SystemTime::now();
        let (key, msg) = rx_sub.recv().unwrap();
        if is_stop {
            is_stop = true;
            start = SystemTime::now();
        }

        count = count + 1;
        if count < TX_NUM {
            let (cmd_id, origin, msg) = parse_msg(&msg);
            match msg {
                MsgClass::REQUEST(req) => {
                    let mut res = response::Response::new();
                    res.set_request_id(req.request_id.clone());
                    match req.req.unwrap() {
                        request::Request_oneof_req::un_tx(tx) => {
                            let tx_res = TxResponse::new(tx.crypt_hash(), "ok".to_string());
                            res.set_tx_state(serde_json::to_string(&tx_res).unwrap());
                        }

                        request::Request_oneof_req::block_number(_) => {
                            res.set_block_number(20);
                        }

                        _ => println!("error request value"),
                    }

                    let msg = factory::create_msg(submodules::CONSENSUS, topics::RESPONSE, communication::MsgType::RESPONSE, res.write_to_bytes().unwrap());
                    tx_pub.send(("consensus.rpc".to_string(), msg.write_to_bytes().unwrap())).unwrap();
                }
                _ => {
                    println!("error JsonRpc request value!!!!");
                }
            }

        } else {
            let sys_time = SystemTime::now();
            let diff = sys_time.duration_since(start).expect("SystemTime::duration_since failed");
            println!{"count = {:?}, timer diff: {:?}", count, diff};
            break;
        }
    }
}
