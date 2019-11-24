use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use crate::error::CustomError;
use std::env;

fn index() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

fn index2() -> impl Responder {
    HttpResponse::Ok().body("Hello world again!")
}

pub(crate) fn start_server() -> Result<(), CustomError> {
    let address = env::var("SUBSYSTEM_MAPPER_SERVER_SOCKET_ADDRESS").unwrap_or("127.0.0.1".to_owned());
    let port = env::var("SUBSYSTEM_MAPPER_SERVER_PORT").unwrap_or("4300".to_owned());
    let bind_address = format!("{}:{}", address, port);

    HttpServer::new(|| {
        App::new()
            .route("/", web::get().to(index))
            .route("/again", web::get().to(index2))
    })
        .bind(&bind_address)
        .map_err(|err| CustomError::new(format!("While binding to address `{}`: {}", bind_address, err)))?
        .run()
        .map_err(|err| CustomError::new(format!("While starting server: {}", err)))?;

    Ok(())
}