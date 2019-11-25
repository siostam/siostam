use crate::error::CustomError;
use crate::subsystem_mapping::GraphRepresentation;
use actix_web::{web, App, HttpResponse, HttpServer};
use std::env;
use std::ops::Deref;
use std::sync::{Arc, RwLock};

pub(crate) fn start_server(
    graph_handle: Arc<RwLock<GraphRepresentation>>,
) -> Result<(), CustomError> {
    let address =
        env::var("SUBSYSTEM_MAPPER_SERVER_SOCKET_ADDRESS").unwrap_or("127.0.0.1".to_owned());
    let port = env::var("SUBSYSTEM_MAPPER_SERVER_PORT").unwrap_or("4300".to_owned());
    let bind_address = format!("{}:{}", address, port);

    HttpServer::new(move || {
        let json_graph_handle = graph_handle.clone();
        let svg_graph_handle = graph_handle.clone();

        App::new()
            .route(
                "/graph/json",
                web::get().to(move || {
                    let json: String;

                    {
                        let graph_handle = &json_graph_handle.clone();
                        let lock = graph_handle.read().unwrap();
                        let graph = lock.deref();
                        json = graph.json();
                    }

                    HttpResponse::Ok().body(json)
                }),
            )
            .route(
                "/graph/svg",
                web::get().to(move || {
                    let json: String;

                    {
                        let graph_handle = &svg_graph_handle.clone();
                        let lock = graph_handle.read().unwrap();
                        let graph = lock.deref();
                        json = graph.svg();
                    }

                    HttpResponse::Ok()
                        .content_type(mime::IMAGE_SVG.as_ref())
                        .body(json)
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
