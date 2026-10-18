use crate::context::AppContext;
use crate::dto::auth_dto::RefreshRequest;
use crate::dto::LoginRequest;
use actix_web::web::{self, Data, Json};
use actix_web::HttpResponse;

// --- CENTRALNA REGISTRACIJA (SAVRŠENA ENKAPSULACIJA) ---

pub fn rute(cfg: &mut web::ServiceConfig) {
    cfg.service(
       web::scope("") // Nalazi se unutar /api/auth opsega koji definiše mod.rs
            .route("/login", web::post().to(login))
            .route("/logout", web::post().to(logout))
            .route("/refresh", web::post().to(refresh_token)),
    );
}

// --- PRIVATNI HANDLERI (ČISTE FUNKCIJE SA EKSPLICITNIM IZVLAČENJEM) ---

async fn login(ctx: Data<AppContext>, body: Json<LoginRequest>) -> HttpResponse {
    let auth_service = ctx.auth_service();

    match auth_service.login(body.into_inner()).await {
        Ok(auth_data) => HttpResponse::Ok().json(auth_data),
        Err(error_poruka) => HttpResponse::BadRequest().body(error_poruka),
    }
}

async fn logout(ctx: Data<AppContext>, dto: Json<RefreshRequest>) -> HttpResponse {
    let auth_service = ctx.auth_service();

    match auth_service.logout(dto.into_inner()).await {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(greska) => HttpResponse::BadRequest().body(greska),
    }
}

async fn refresh_token(ctx: Data<AppContext>, dto: Json<RefreshRequest>) -> HttpResponse {
    let auth_service = ctx.auth_service();

    match auth_service.osvezi_token(dto.into_inner()).await {
        Ok(odgovor) => HttpResponse::Ok().json(odgovor),
        Err(greska) => HttpResponse::BadRequest().body(greska),
    }
}
