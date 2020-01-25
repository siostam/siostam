use crate::error::CustomError;
use crate::server::actors::UpdateMasterActor;
use crate::subsystem_mapping::GraphRepresentation;
use actix::{Actor, Addr};
use actix_cors::Cors;
use actix_files as fs;
use actix_web::{http::header, middleware::Logger, web, App, HttpResponse, HttpServer};
use log::{debug, info};
use std::env;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

mod actors;
mod websocket;

/// We get the executable path and search for the 'public' folder besides it.
/// If not available, we search in the working dir
fn get_public_path() -> String {
    let public_path = match env::current_exe() {
        Ok(mut path) => {
            path.pop();
            path.push("public");

            if !path.exists() {
                path = PathBuf::from("./public")
            }

            path.to_string_lossy().to_string()
        }
        Err(_) => "./public".to_owned(),
    };

    public_path
}

pub struct AppState {
    update_master: Arc<Mutex<Addr<UpdateMasterActor>>>,
}

pub(crate) async fn start_server(
    graph_handle: Arc<RwLock<GraphRepresentation>>,
) -> Result<(), CustomError> {
    let address = env::var("SIOSTAM_SERVER_SOCKET_ADDRESS").unwrap_or("127.0.0.1".to_owned());
    let port = env::var("SIOSTAM_SERVER_PORT").unwrap_or("4300".to_owned());
    let bind_address = format!("{}:{}", address, port);

    // Detect where to search for static files
    let public_path = get_public_path();
    debug!("Static files will be searched in {}", public_path);

    HttpServer::new(move || {
        let json_graph_handle = graph_handle.clone();
        let svg_graph_handle = graph_handle.clone();

        let update_master = Arc::from(Mutex::new(actors::UpdateMasterActor::new().start()));
        let app_data = web::Data::new(AppState { update_master });

        App::new()
            .app_data(app_data)
            .wrap(
                Cors::new() // <- Construct CORS middleware builder
                    .allowed_origin("http://localhost:4200")
                    .allowed_origin("http://127.0.0.1:4200")
                    .allowed_origin("http://localhost:4300")
                    .allowed_origin("http://127.0.0.1:4300")
                    .allowed_methods(vec!["GET", "POST"])
                    .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT])
                    .allowed_header(header::CONTENT_TYPE)
                    .max_age(3600)
                    .finish(),
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
            .route("/ws/", web::get().to(websocket::index))
            .service(fs::Files::new("/", public_path.as_str()).index_file("index.html"))
    })
    .bind(&bind_address)
    .map(|server| {
        info!("You may access the server at http://localhost:{}/", port);
        server
    })
    .map_err(|err| {
        CustomError::new(format!(
            "While binding to address `{}`: {}",
            bind_address, err
        ))
    })?
    .run()
    .await
    .map_err(|err| CustomError::new(format!("While starting server: {}", err)))?;

    Ok(())
}
