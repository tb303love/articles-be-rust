// src/handlers/auth_handler.rs
use crate::context::AppContext;
use crate::dtos::LoginRequest;
use actix_web::{post, web, HttpResponse, Responder};

#[post("/login")]
pub async fn login_handler(
    ctx: web::Data<AppContext>,
    body: web::Json<LoginRequest>,
) -> impl Responder {
    match ctx.auth_service.login(body.into_inner()).await {
        Ok(auth_data) => HttpResponse::Ok().json(auth_data),
        Err(error_poruka) => HttpResponse::BadRequest().body(error_poruka),
    }
}

// src/handlers/auth_handler.rs

pub fn init_auth_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(login_handler); // Čista registracija rute bez skopova
}
