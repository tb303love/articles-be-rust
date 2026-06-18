// src/handlers/mod.rs
pub mod article_handler;
pub mod auth_handler;
pub mod barcode_handler;
mod ws_handler;

use crate::utils::auth_middleware::Authentication;
use actix_web::web;

/// Centralna funkcija koja sklapa kompletnu mapu ruta za celu aplikaciju
pub fn konfigurisi_sve_rute(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            // ========================================================
            // 1. JAVNA ZONA (Rute bez autentifikacije)
            // ========================================================
            .service(
                web::scope("/auth").configure(auth_handler::rute), // /api/auth/...
            )
            // ========================================================
            // 2. ZAŠTIĆENA ZONA (Sve rute unutar ovog pod-skopa traže JWT token)
            // ========================================================
            .service(
                web::scope("")
                    .wrap(Authentication)
                    // Čist i direktan poziv funkcija modula - bez closure-a, klasa i ctx-a!
                    .configure(barcode_handler::rute) // /api/scans/stock
                    .configure(article_handler::rute), // /api/articles/...
            ),
    );
}
