// src/handlers/mod.rs
pub mod article_handler;
pub mod auth_handler;

use crate::utils::auth_middleware::Authentication;
use actix_web::web;

/// Centralna funkcija koja sklapa kompletnu mapu ruta za celu aplikaciju
pub fn konfigurisi_sve_rute(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            // ========================================================
            // 1. JAVNA ZONA (Sve rute unutar ovog pod-skopa su potpuno slobodne)
            // ========================================================
            .service(
                web::scope("/auth").configure(auth_handler::init_auth_routes), // Povezuje /api/auth/login
            )
            // ========================================================
            // 2. ZAŠTIĆENA ZONA (Sve rute unutar ovog pod-skopa traže JWT token)
            // ========================================================
            .service(
                web::scope("")
                    .wrap(Authentication)
                    .configure(article_handler::init_article_routes),
            ),
    );
}
