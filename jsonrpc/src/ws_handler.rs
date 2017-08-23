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

use base_hanlder::*;
use ws;
use ws::{Factory, CloseCode, Handler};


impl Factory for RpcHandler {
    type Handler = RpcHandler;
    fn connection_made(&mut self, ws: ws::Sender) -> RpcHandler {
        let mut this = self.clone();
        this.set_tx_ws(ws);
        this
    }
}


impl Handler for RpcHandler {
    fn on_message(&mut self, msg: ws::Message) -> ws::Result<()> {
        trace!("Server got message '{}'  post thread_pool deal task ", msg);
        let this = self.clone();
        let sender = Senders::WEBSOCKET(this.tx_ws.as_ref().unwrap().clone());
        self.tx_pool
            .as_ref()
            .unwrap()
            .send(Box::new(move || { this.deal_req(msg.into_text().unwrap(), sender); }));
        Ok(())
    }

    fn on_close(&mut self, code: CloseCode, reason: &str) {
        trace!("WebSocket closing for ({:?}) {}, {}", code, reason, self.tx_ws.as_ref().unwrap().token().0);
    }
}
