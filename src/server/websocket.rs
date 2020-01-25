//! Source: https://github.com/actix/examples/blob/master/websocket/src/main.rs
//! Simple echo websocket server.
//! Open `http://localhost:8080/ws/index.html` in browser
//! or [python console client](https://github.com/actix/examples/blob/master/websocket/websocket-client.py)
//! could be used for testing.

use std::time::{Duration, Instant};

use crate::server::actors::{Subscribe, Unsubscribe, UpdateMasterActor};
use crate::server::{websocket, AppState};
use actix::prelude::*;
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use std::ops::DerefMut;
use std::sync::{Arc, Mutex};

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

/// websocket connection is long running connection, it easier
/// to handle with an actor
pub(crate) struct MyWebSocket {
    /// Client must send ping at least once per 10 seconds (CLIENT_TIMEOUT),
    /// otherwise we drop connection.
    hb: Instant,

    /// Address of the update master to subscribe/unsubscribe
    update_master: Arc<Mutex<Addr<UpdateMasterActor>>>,
}

pub async fn index(
    data: web::Data<AppState>,
    req: HttpRequest,
    stream: web::Payload,
) -> Result<HttpResponse, Error> {
    println!("{:?}", req);
    let res = ws::start(
        websocket::MyWebSocket::new(data.update_master.clone()),
        &req,
        stream,
    );
    println!("{:?}", res);
    res
}

impl Actor for MyWebSocket {
    type Context = ws::WebsocketContext<Self>;

    /// Method is called on actor start. We start the heartbeat process here.
    fn started(&mut self, ctx: &mut Self::Context) {
        println!("Started");
        // Subscribe to get updates
        {
            match self.update_master.as_ref().lock() {
                Ok(ref mut handle) => {
                    let actor = handle.deref_mut();
                    actor.do_send(Subscribe(ctx.address().recipient()));
                }
                Err(err) => log::error!("{}", err.to_string()),
            }
        }

        self.hb(ctx);
    }

    /// Method is called on actor stop. We start the heartbeat process here.
    fn stopped(&mut self, ctx: &mut Self::Context) {
        println!("stopped");
        // Subscribe to stop updates
        {
            match self.update_master.as_ref().lock() {
                Ok(ref mut handle) => {
                    let actor = handle.deref_mut();
                    actor.do_send(Unsubscribe(ctx.address().recipient()));
                }
                Err(err) => log::error!("{}", err.to_string()),
            }
        }

        self.hb(ctx);
    }
}

/// Handler for `ws::Message`
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for MyWebSocket {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        // process websocket messages
        println!("WS: {:?}", msg);
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.hb = Instant::now();
            }
            Ok(ws::Message::Text(text)) => ctx.text(text),
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            Ok(ws::Message::Close(_)) => {
                ctx.stop();
            }
            _ => ctx.stop(),
        }
    }
}

impl MyWebSocket {
    pub(crate) fn new(update_master: Arc<Mutex<Addr<UpdateMasterActor>>>) -> Self {
        Self {
            hb: Instant::now(),
            update_master,
        }
    }

    /// helper method that sends ping to client every second.
    ///
    /// also this method checks heartbeats from client
    fn hb(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            // check client heartbeats
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                // heartbeat timed out
                println!("Websocket Client heartbeat failed, disconnecting!");

                // stop actor
                ctx.stop();

                // don't try to send a ping
                return;
            }

            ctx.ping(b"");
        });
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct PleaseUpdate;

//impl Message for PleaseUpdate {
//    type Result = Result<bool, actix_web::Error>;
//}

/// Define handler for `Ping` message
impl Handler<PleaseUpdate> for MyWebSocket {
    type Result = ();

    fn handle(&mut self, _msg: PleaseUpdate, ctx: &mut ws::WebsocketContext<Self>) -> Self::Result {
        ctx.text("{ \"message\": \"please-update\" }");
    }
}
