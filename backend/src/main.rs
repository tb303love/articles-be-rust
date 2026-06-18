mod config;
mod context;
mod dto;
mod error;
mod error_response;
mod handlers;
mod models;
mod repositories;
mod services;
mod utils;
mod ws;

use crate::handlers::konfigurisi_sve_rute;
use crate::utils::handle_files::handle_files;
use actix_web::dev::{Server, ServerHandle};
use actix_web::http::StatusCode;
use actix_web::web::Data;
use actix_web::{middleware::ErrorHandlers, App, HttpServer};
use config::AppConfig;
use context::AppContext;
use env_logger::Env;
use error_response::_404;
use services::pokreni_windows_servis;
use sqlx::MySqlPool;
use std::env::{args, set_current_dir, var};
use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync::Arc;
use utils::ssl_configuration::ucitaj_ssl_konfiguraciju;

fn main() {
    dotenvy::dotenv().ok();
    // 1. Inicijalizujemo loger bezbedno
    let _ = env_logger::builder()
        .parse_env(Env::new().default_filter_or("info"))
        .try_init();

    // 2. Čitamo argumente prosleđene aplikaciji kroz terminal
    let arguments: Vec<String> = args().collect();

    // Proveravamo da li je prosleđen flag za lokalni razvoj
    let pokrenuto_lokalno = arguments.iter().any(|arg| arg == "local" || arg == "--dev");

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
pub async fn pokreni_actix_server() -> std::io::Result<(ServerHandle, Server)> {
    let run_env = var("RUN_ENV").unwrap_or_else(|_| "production".to_string());

    // Ispravljanje radnog direktorijuma je neophodno samo kada aplikacija radi kao servis
    if run_env != "development" {
        if let Ok(trenutni_exe) = std::env::current_exe() {
            if let Some(exe_direktorijum) = trenutni_exe.parent() {
                let _ = set_current_dir(exe_direktorijum);
            }
        }
    }

    // 1. Pravimo Arc oko konfiguracije
    let config_arc: Arc<AppConfig> = Arc::new(AppConfig::from_env());

    log::info!(
        "🔒 Pokretanje SIGURNOG servera (HTTPS) u [{}] režimu na adresi https://{}:{}",
        config_arc.app_env,
        config_arc.server_ip,
        config_arc.server_port
    );

    // 2. Kloniramo samo database_url jer sqlx zahteva vlasništvo nad Stringom
    let db_url = config_arc.database_url.clone();

    // Konekcija sa MySQL bazom podataka
    let pool = match MySqlPool::connect(&db_url).await {
        Ok(p) => p,
        Err(e) => {
            log::error!("CRITICAL: Konekcija sa MySQL bazom neuspešna: {:?}", e);
            return Err(std::io::Error::new(ErrorKind::ConnectionRefused, e));
        }
    };

    let ssl_config = ucitaj_ssl_konfiguraciju();

    // 3. Pripremamo podatke za HttpServer closure (svaki worker thread dobija svoj klon)
    let index_html_putanja = PathBuf::from(&config_arc.frontend_dir).join("index.html");
    let frontend_dir_za_rute = config_arc.frontend_dir.clone();
    let server_ip = config_arc.server_ip.clone();
    let server_port = config_arc.server_port;

    // 4. Ovde kontekst bezbedno preuzima originalni config_arc bez ikakvog uništavanja
    let ctx = AppContext::new(pool, config_arc);

    // 5. Pravimo web::Data omotač za Actix DI (dependency injection)
    let app_data = Data::new(ctx);

    // Inicijalizacija Actix servera
    let server = HttpServer::new(move || {
        let index_putanja = index_html_putanja.clone();
        let f_dir = frontend_dir_za_rute.clone();

        App::new()
            // Ubacujemo naš globalni deljeni kontekst u Actix DI sistem
            .app_data(app_data.clone())
            // Pozivamo funkciju direktno, bez closure-a i bez prosleđivanja ctx-a
            .configure(konfigurisi_sve_rute)
            .service(handle_files(index_putanja, &f_dir))
            .wrap(ErrorHandlers::new().handler(StatusCode::NOT_FOUND, _404))
    })
    .bind_rustls_0_23((server_ip, server_port), ssl_config)?
    .run();

    let server_handle = server.handle();

    // Vraćamo i handle i instancu nazad pozivaocu
    Ok((server_handle, server))
}
