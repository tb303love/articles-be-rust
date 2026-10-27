use crate::context::AppContext;
use crate::dto::auth_dto::RefreshRequest;
use crate::dto::LoginRequest;
use crate::error::ApiError;
use actix_web::web::{self, Data, Json};
use actix_web::HttpResponse;

// --- CENTRALNA REGISTRACIJA (SAVRŠENA ENKAPSULACIJA) ---

pub fn rute(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("")
            .route("/login", web::post().to(login))
            .route("/logout", web::post().to(logout))
            .route("/refresh", web::post().to(refresh_token)),
    );
}

async fn login(ctx: Data<AppContext>, body: Json<LoginRequest>) -> Result<HttpResponse, ApiError> {
    let auth_data = ctx.auth_service().login(body.into_inner()).await?;
    Ok(HttpResponse::Ok().json(auth_data))
}

async fn logout(ctx: Data<AppContext>, dto: Json<RefreshRequest>) -> Result<HttpResponse, ApiError> {
    ctx.auth_service().logout(dto.into_inner()).await?;
    Ok(HttpResponse::NoContent().finish())
}

async fn refresh_token(
    ctx: Data<AppContext>,
    dto: Json<RefreshRequest>,
) -> Result<HttpResponse, ApiError> {
    let odgovor = ctx.auth_service().osvezi_token(dto.into_inner()).await?;
    Ok(HttpResponse::Ok().json(odgovor))
}
