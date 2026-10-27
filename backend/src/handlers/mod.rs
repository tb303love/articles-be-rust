pub mod article_handler;
pub mod auth_handler;
pub mod barcode_handler;
pub mod order_handler;
mod ws_handler;

use crate::utils::auth_middleware::Authentication;
use actix_web::web;

/// Centralna funkcija koja sklapa kompletnu mapu ruta za celu aplikaciju
pub fn all_api_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            // 1. JAVNA ZONA (Rute bez autentifikacije)
            .service(web::scope("/auth").configure(auth_handler::rute))
            // 2. ZAŠTIĆENA ZONA (Sve rute unutar ovog pod-skopa traže JWT token)
            .service(
                web::scope("")
                    .wrap(Authentication)
                    .configure(barcode_handler::rute)
                    .configure(article_handler::rute)
                    .configure(order_handler::rute),
            ),
    );
}
