pub mod config;
pub mod context;
mod dtos;
mod error_response;
mod handlers;
mod models;
mod repositories;
mod services;
mod utils;

use std::sync::Arc;
use actix_files::NamedFile;
use actix_web::{middleware::ErrorHandlers, web, App, HttpServer};

use config::AppConfig;
use context::AppContext;
use error_response::_404;
use handlers::konfigurisi_sve_rute;
use services::pokreni_windows_servis;
use sqlx::MySqlPool;
use utils::ssl_configuration::ucitaj_ssl_konfiguraciju;
use crate::config::AppConfigProvider;

fn main() {
    // 1. Inicijalizujemo loger bezbedno
    let _ = env_logger::builder()
        .parse_env(env_logger::Env::new().default_filter_or("info"))
        .try_init();

    // 2. Čitamo argumente prosleđene aplikaciji kroz terminal
    let args: Vec<String> = std::env::args().collect();

    // Proveravamo da li je prosleđen flag za lokalni razvoj
    let pokrenuto_lokalno = args.iter().any(|arg| arg == "local" || arg == "--dev");

    if !pokrenuto_lokalno {
        // PRODUKCIJA: Windows SCM pokreće aplikaciju kao servis (bez argumenata)
        log::info!("Registracija Windows servisa u toku...");
        if let Err(e) = pokreni_windows_servis() {
            log::error!("Neuspešno pokretanje kroz Windows Service SCM: {:?}", e);
        }
        return; // SCM preuzima kontrolu, main ovde završava
    }

    // RAZVOJ (LOKAL): Pokrenuto ručno preko `cargo run -- local`
    log::info!("🚀 Razvojni režim detektovan. Pokrećem Actix direktno...");

    let sys = actix_web::rt::System::new();
    if let Err(actix_err) = sys.block_on(async {
        match pokreni_actix_server().await {
            Ok((_handle, server)) => {
                // Za lokalni razvoj, samo pustimo server da blokira trenutnu nit
                server.await
            }
            Err(e) => Err(e),
        }
    }) {
        log::error!(
            "Greška pri lokalnom pokretanju Actix servera: {:?}",
            actix_err
        );
    }
}

// Pozadinska nit unutar vašeg src/services/win_service.rs poziva ovu funkciju u produkciji,
// dok je main poziva direktno u razvojnom režimu.
pub async fn pokreni_actix_server(
) -> std::io::Result<(actix_web::dev::ServerHandle, actix_web::dev::Server)> {
    let run_env = std::env::var("RUN_ENV").unwrap_or_else(|_| "production".to_string());

    // Ispravljanje radnog direktorijuma je neophodno samo kada aplikacija radi kao servis
    if run_env != "development" {
        if let Ok(trenutni_exe) = std::env::current_exe() {
            if let Some(exe_direktorijum) = trenutni_exe.parent() {
                let _ = std::env::set_current_dir(exe_direktorijum);
            }
        }
    }

    let config = AppConfig::from_env();
    let config_arc: Arc<dyn AppConfigProvider> = Arc::new(config);

    log::info!(
        "🔒 Pokretanje SIGURNOG servera (HTTPS) u [{}] režimu na adresi https://{}:{}",
        config_arc.get_app_env(),
        config_arc.get_server_ip(),
        config_arc.get_server_port()
    );

    // Konekcija sa MySQL bazom podataka
    let pool = match MySqlPool::connect(&config_arc.get_database_url()).await {
        Ok(p) => p,
        Err(e) => {
            log::error!("CRITICAL: Konekcija sa MySQL bazom neuspešna: {:?}", e);
            return Err(std::io::Error::new(
                std::io::ErrorKind::ConnectionRefused,
                e,
            ));
        }
    };

    // 2. Prosleđujemo trejt objekat u vaš AppContext
    let app_context = AppContext::new(pool, config_arc.clone());
    let app_data = web::Data::new(app_context);
    let ssl_config = ucitaj_ssl_konfiguraciju();

    // =========================================================================
    // NOVO: Pripremamo dinamičke putanje iz config strukture pre HttpServer-a
    // =========================================================================
    let klijent_dir = config_arc.get_frontend_dir().to_string();

    // Kreiramo asinhronu putanju do index.html koju će radne niti (workers) deliti
    let index_html_putanja = std::path::PathBuf::from(&klijent_dir).join("index.html");


    // Inicijalizacija Actix servera
    let server = HttpServer::new(move || {
        // Kloniramo putanju za index unutar closure-a svakog radnika (workera)
        // Kloniramo putanju za index unutar closure-a svakog radnika (workera)
        let index_putanja = index_html_putanja.clone();

        // NOVO: Kloniramo i string direktorijuma za potrebe ovog radnika
        let dir_za_radnika = klijent_dir.clone();

        App::new()
            .app_data(app_data.clone())
            .configure(konfigurisi_sve_rute)
            .service(
                // IZMENJENO: Dinamički prosleđujemo putanju iz konfiguracije
                actix_files::Files::new("/", &dir_za_radnika)
                    .index_file("index.html")
                    // Prikaz strukture foldera isključujemo u produkciji radi bezbednosti
                    .show_files_listing()
                    .default_handler(web::to(move |req: actix_web::HttpRequest| {
                        let putanja_za_handler = index_putanja.clone();
                        async move {
                            // IZMENJENO: Otvaramo index.html sa dinamičke lokacije
                            match NamedFile::open_async(&putanja_za_handler).await {
                                Ok(file) => file.into_response(&req),
                                Err(e) => {
                                    log::error!("Kritična greška: Angular index.html nije pronađen na lokaciji {:?}: {:?}", putanja_za_handler, e);
                                    actix_web::HttpResponse::NotFound().body("Frontend nije dostupan.")
                                }
                            }
                        }
                    })),
            )
            .wrap(ErrorHandlers::new().handler(actix_web::http::StatusCode::NOT_FOUND, _404))
    })
    .bind_rustls_0_23((config_arc.get_server_ip(), config_arc.get_server_port()), ssl_config)?
    .run();

    let server_handle = server.handle();

    // Vraćamo i handle i instancu nazad pozivaocu
    Ok((server_handle, server))
}
