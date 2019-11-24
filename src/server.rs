use crate::error::CustomError;
use crate::subsystem_mapping::Graph;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use std::env;
use std::ops::Deref;
use std::sync::{Arc, RwLock};

fn index(req: HttpRequest) -> impl Responder {
    let result = match req.headers().get("Accept-Language") {
        Some(lang) => lang.to_str().unwrap_or("--"),
        None => "Hello world!",
    };

    HttpResponse::Ok().body(result.to_owned())
}

fn index2() -> impl Responder {
    HttpResponse::Ok().body("Hello world again!")
}

pub(crate) fn start_server(graph_handle: Arc<RwLock<Graph>>) -> Result<(), CustomError> {
    let address =
        env::var("SUBSYSTEM_MAPPER_SERVER_SOCKET_ADDRESS").unwrap_or("127.0.0.1".to_owned());
    let port = env::var("SUBSYSTEM_MAPPER_SERVER_PORT").unwrap_or("4300".to_owned());
    let bind_address = format!("{}:{}", address, port);

    HttpServer::new(move || {
        let graph_handle = graph_handle.clone();

        App::new().route(
            "/",
            web::get().to(move || {
                let json: String;

                {
                    let graph_handle = graph_handle.clone();
                    let lock = graph_handle.read().unwrap();
                    let graph = lock.deref();
                    json = graph.to_json().expect("Error on json output");
                }

                HttpResponse::Ok().body(json)
            }),
        )
    })
    .bind(&bind_address)
    .map_err(|err| {
        CustomError::new(format!(
            "While binding to address `{}`: {}",
            bind_address, err
        ))
    })?
    .run()
    .map_err(|err| CustomError::new(format!("While starting server: {}", err)))?;

    Ok(())
}
