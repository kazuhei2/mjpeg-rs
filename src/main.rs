use actix_web::web::Data;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};

#[macro_use]
extern crate log;
use env_logger;

extern crate clap;
use clap::load_yaml;

use std::sync::Mutex;

mod broadcaster;
use broadcaster::Broadcaster;

fn main() {
    let yaml = load_yaml!("mjpeg-rs.yml");
    let matches = clap::App::from_yaml(yaml).get_matches();

    let ip_port = matches.value_of("IP_PORT").unwrap_or("0.0.0.0:8080");
    println!("IP_PORT is {}", ip_port);

    env_logger::init();

    let data = Broadcaster::create();

    HttpServer::new(move || {
        App::new()
            .register_data(data.clone())
            .route("/", web::get().to(index))
            .route("/streaming", web::get().to(new_client))
    })
    .bind(ip_port)
    .expect("Unable to bind port")
    .run()
    .unwrap();
}

fn index() -> impl Responder {
    let content = include_str!("index.html");

    HttpResponse::Ok()
        .header("Content-Type", "text/html")
        .body(content)
}

/// Register a new client and return a response
fn new_client(broadcaster: Data<Mutex<Broadcaster>>) -> impl Responder {
    info!("new_client...");
    let rx = broadcaster.lock().unwrap().new_client();

    HttpResponse::Ok()
        .header("Cache-Control", "no-store, must-revalidate")
        .header("Pragma", "no-cache")
        .header("Expires", "0")
        .header("Connection", "close")
        .header(
            "Content-Type",
            "multipart/x-mixed-replace;boundary=boundarydonotcross",
        )
        .no_chunking()
        .streaming(rx) // now starts streaming
}
