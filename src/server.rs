use crate::error::CustomError;
use crate::subsystem_mapping::GraphRepresentation;
use actix_cors::Cors;
use actix_files as fs;
use actix_web::{http::header, middleware::Logger, web, App, HttpResponse, HttpServer};
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
            .wrap(
                Cors::new() // <- Construct CORS middleware builder
                    .allowed_origin("http://localhost:4200")
                    .allowed_methods(vec!["GET", "POST"])
                    .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT])
                    .allowed_header(header::CONTENT_TYPE)
                    .max_age(3600),
            )
            .wrap(Logger::default())
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
                    let svg: String;

                    {
                        let graph_handle = &svg_graph_handle.clone();
                        let lock = graph_handle.read().unwrap();
                        let graph = lock.deref();
                        svg = graph.svg();
                    }

                    HttpResponse::Ok()
                        .content_type(mime::IMAGE_SVG.as_ref())
                        .body(svg)
                }),
            )
            .service(fs::Files::new("/", "./public").index_file("index.html"))
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
