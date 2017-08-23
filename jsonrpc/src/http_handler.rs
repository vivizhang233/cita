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

use base_hanlder::*;
use futures::Async;
use futures::Future;
use futures::Poll;
use futures::Stream;
//use futures::future::FutureResult;
use futures::sync::oneshot;
use futures::sync::oneshot::Receiver;
use hyper;
use hyper::Method;
//use hyper::header::ContentLength;
use hyper::server::{Service, NewService, Request, Response};
use jsonrpc_types::error::Error;
use std::boxed::Box;
use std::io;


impl NewService for RpcHandler {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Instance = RpcHandler;
    fn new_service(&self) -> io::Result<RpcHandler> {
        Ok(self.clone())
    }
}


pub struct HanderReslut {
    inner: Inner<Response, hyper::Error>,
}

impl HanderReslut {
    pub fn new_err(err: hyper::Error) -> Self {
        HanderReslut { inner: Inner::Error(Some(err)) }
    }

    pub fn new_ok(receiver: Receiver<Response>) -> Self {
        HanderReslut { inner: Inner::Ok(receiver) }
    }

    pub fn new_not_ready() -> Self {
        HanderReslut { inner: Inner::NotReady }
    }
}

enum Inner<T, E> {
    Error(Option<E>),
    Ok(Receiver<T>),
    NotReady,
}


impl Future for HanderReslut {
    type Item = Response;
    type Error = hyper::Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.inner {
            Inner::NotReady => Ok(Async::NotReady),
            Inner::Error(ref mut err) => Err(err.take().unwrap()),
            Inner::Ok(ref mut reciver) => Ok(reciver.poll().unwrap()),
        }
    }
}

impl Service for RpcHandler {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = HanderReslut;

    fn call(&self, req: Request) -> Self::Future {
        let path = req.path().to_owned();
        let method = req.method().clone();
        trace!("body {:?}", req.body_ref());

        req.body().for_each(move |chunk| {
                                trace!("-msg--{:?}", chunk);
                                Ok(())
                            });
        return HanderReslut::new_not_ready();

    }
}

//        match req.body().poll() {
//            Ok(asy_data) => {
//                match asy_data {
//                    Async::Ready(op_data) => {
//                        trace!("op_data {:?}", op_data);
//                        let (tx, rx) = oneshot::channel();
//                        let this = self.clone();
//                        let sender = Senders::HTTP(tx);
//                        self.tx_pool.as_ref().unwrap().send(Box::new(move || if let Some(data) = op_data {
//                                                                         if path == "/".to_string() && method == Method::Post {
//                                                                             let body = String::from_utf8(data.to_vec()).unwrap();
//                                                                             trace!("Request data {:?}", body);
//                                                                             this.deal_req(body, sender);
//                                                                         } else {
//                                                                             sender.send(from_err(Error::invalid_request()));
//                                                                         }
//                                                                     } else {
//
//                                                                         sender.send(from_err(Error::invalid_request()));
//                                                                     }));
//                        return HanderReslut::new_ok(rx);
//                    }
//                    Async::NotReady => {
//                        trace!("op_data NotReady");
//                        return HanderReslut::new_not_ready();
//                    }
//                }
//            }
//            Err(err) => {
//                trace!("op_data err {:?}", err);
//                return HanderReslut::new_err(err);
//            }
//        }
//    }
