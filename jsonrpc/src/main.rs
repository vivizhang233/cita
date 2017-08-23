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

#![feature(plugin)]
//#![feature(box_snytax)]
//#![allow(deprecated)]
//#![allow(unused_assignments)]
#![allow(unused_must_use)]
#![allow(unused_variables)]
extern crate futures;
extern crate hyper;
extern crate libproto;
extern crate protobuf;
extern crate uuid;
#[macro_use]
extern crate log;
extern crate util;
extern crate serde_json;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate rustc_serialize;
extern crate pubsub;
extern crate time;
extern crate proof;
extern crate docopt;
extern crate cpuprofiler;
extern crate jsonrpc_types;
extern crate dotenv;
extern crate transaction as cita_transaction;
extern crate cita_log;
extern crate threadpool;
extern crate num_cpus;
extern crate parking_lot;
extern crate ws;
extern crate clap;

pub mod http_handler;
pub mod base_hanlder;
pub mod ws_handler;
pub mod config;


use clap::App;
use config::ProfileConfig;
use cpuprofiler::PROFILER;
use dotenv::dotenv;
use hyper::server::Http;

use log::LogLevelFilter;
use pubsub::start_pubsub;

use std::boxed::Box;
use std::net::ToSocketAddrs;
use std::sync::mpsc::channel;
use std::thread;
use std::time::Duration;


fn start_profile(config: &ProfileConfig) {
    if config.enable {

        if config.flag_prof_start != 0 && config.flag_prof_duration != 0 {
            let start = config.flag_prof_start;
            let duration = config.flag_prof_duration;
            thread::spawn(move || {
                              thread::sleep(Duration::new(start, 0));
                              PROFILER.lock().unwrap().start("./jsonrpc.profile").expect("Couldn't start");
                              thread::sleep(Duration::new(duration, 0));
                              PROFILER.lock().unwrap().stop().unwrap();
                          });
        }

    }
}



fn main() {
    dotenv().ok();
    ::std::env::set_var("RUST_BACKTRACE", "full");
    cita_log::format(LogLevelFilter::Info);
    info!("CITA:jsonrpc ");

    // todo load config
    let matches = App::new("JsonRpc")
        .version("0.1")
        .author("Cryptape")
        .about("CITA JSON-RPC by Rust")
        .args_from_usage("-c, --config=[FILE] 'Sets a custom config file'")
        .get_matches();

    let mut config_path = "./jsonrpc.json";
    if let Some(c) = matches.value_of("config") {
        info!("Value for config: {}", c);
        config_path = c;
    }

    let config = config::read_user_from_file(config_path).expect("config error!");
    info!("CITA:jsonrpc config \n {:?}", serde_json::to_string_pretty(&config).unwrap());

    start_profile(&config.profile_config);
    // init pubsub
    let (tx_sub, rx_sub) = channel();
    let (tx_pub, rx_pub) = channel();
    start_pubsub("jsonrpc", vec!["*.rpc"], tx_sub, rx_pub);

    //main loop channel
    let (tx_pool, rx_pool) = channel();
    let tx_pool_clone = tx_pool.clone();

    //create handler
    let handler = base_hanlder::RpcHandler::default();
    let mut ws_handler = handler.clone();
    ws_handler.set_tx_mq(tx_pub);
    ws_handler.set_tx_pool(tx_pool);

    let http_handler = ws_handler.clone();

    //http
    if config.http_config.enable {
        let http_config = config.http_config.clone();
        thread::spawn(move || {
                          let addr = http_config.listen_ip.clone() + ":" + &http_config.listen_port.clone().to_string();
                          info!("Http Listening on {}", addr);
                          let addr = addr.to_socket_addrs().unwrap().as_slice()[0];
                          let server = Http::new().bind(&addr, http_handler).unwrap();
                          server.run().unwrap();
                      });
    }

    //ws
    if config.ws_config.enable {
        let ws_config = config.ws_config.clone();
        thread::spawn(move || {
                          let url = ws_config.listen_ip.clone() + ":" + &ws_config.listen_port.clone().to_string();
                          info!("WebSocket Listening on {}", url);
                          let mut ws_build = ws::Builder::new();
                          ws_build.with_settings(ws_config.into());
                          let ws_server = ws_build.build(ws_handler).unwrap();
                          let _ = ws_server.listen(url);
                      });
    }

    //mq loop
    thread::spawn(move || loop {
                      let (key, msg) = rx_sub.recv().unwrap();
                      let mut this = handler.clone();
                      tx_pool_clone.send(Box::new(move || { this.deal_res(key, msg); }));

                  });

    //thread pool loop
    let thread_pool = threadpool::ThreadPool::new(10);
    loop {
        let run = rx_pool.recv().unwrap();
        thread_pool.execute(move || { run.call_box(); });
    }
}
