use crate::core::Core;
use crate::error::CustomError;
use crate::server::actors::UpdateMasterActor;
use actix::{Actor, Addr};
use actix_cors::Cors;
use actix_files as fs;
use actix_web::{http::header, middleware::Logger, web, App, HttpResponse, HttpServer};
use log::{debug, info};
use std::env;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

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

pub(crate) async fn start_server(access_to_core: Arc<Core>) -> Result<(), CustomError> {
    let address = env::var("SIOSTAM_SERVER_SOCKET_ADDRESS").unwrap_or("127.0.0.1".to_owned());
    let port = env::var("SIOSTAM_SERVER_PORT").unwrap_or("4300".to_owned());
    let bind_address = format!("{}:{}", address, port);

    // Detect where to search for static files
    let public_path = get_public_path();
    debug!("Static files will be searched in {}", public_path);

    HttpServer::new(move || {
        let json_access_to_core = access_to_core.clone();
        let svg_access_to_core = access_to_core.clone();
        let update_master_access_to_core = access_to_core.clone();

        // Wrap an access to the core into app_data to allow the actors from websocket to get updates
        let update_master = actors::UpdateMasterActor::new(update_master_access_to_core).start();
        let update_master = Arc::from(Mutex::new(update_master));
        let app_data = web::Data::new(AppState { update_master });

        // Construct the app main routes
        App::new()
            .app_data(app_data)
            .wrap(Logger::default())
            .service(
                web::scope("/graph")
                    .wrap(build_cors().finish())
                    .route(
                        "/json",
                        web::get().to(move || match json_access_to_core.json() {
                            Ok(json) => HttpResponse::Ok().body(json),
                            Err(err) => HttpResponse::InternalServerError()
                                .body(serde_json::to_string(&err).unwrap_or(err.message)),
                        }),
                    )
                    .route(
                        "/svg",
                        web::get().to(move || match svg_access_to_core.svg() {
                            Ok(svg) => HttpResponse::Ok()
                                .content_type(mime::IMAGE_SVG.as_ref())
                                .body(svg),
                            Err(err) => HttpResponse::InternalServerError()
                                .body(serde_json::to_string(&err).unwrap_or(err.message)),
                        }),
                    ),
            )
            .service(web::scope("/ws").route("/", web::get().to(websocket::index)))
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

/// Construct the settings for Cross-Origin Resource Sharing (CORS)
/// Details on https://developer.mozilla.org/fr/docs/Web/HTTP/CORS
/// We must allow only the given origins to avoid security issues
fn build_cors() -> Cors {
    // CORS: allow origin based on what is defined in env var
    let default_origin_settings = "*".to_string();
    let origin_settings =
        env::var("SIOSTAM_SERVER_CORS_ALLOWED_ORIGINS").unwrap_or(default_origin_settings);

    // Multiple origins are separated by commas
    let origins: Vec<&str> = origin_settings.split(",").collect();
    debug!("Allowing origins: {:?}", origins);

    // Construct the CORS builder and allow origins
    let mut cors = Cors::new();
    for origin in origins {
        cors = cors.allowed_origin(origin)
    }

    cors.allowed_methods(vec!["GET", "POST"])
        .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT])
        .allowed_header(header::CONTENT_TYPE)
        .max_age(3600)
}
