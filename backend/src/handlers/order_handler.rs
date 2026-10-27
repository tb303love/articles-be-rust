use crate::context::AppContext;
use crate::dto::order_dto::{OrderRequest, OrderTypeQuery};
use crate::error::order_error::OrderError;
use actix_web::{
   web::{self, get, patch, post, Data, Json, Path, Query},
   HttpResponse,
};
use serde_json::json;

pub fn rute(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/orders")
            .route("", post().to(create_order))
            .route("/today", get().to(today_orders))
            .route("/revenue/today", get().to(today_revenue))
            .route("/{id}/type", patch().to(update_type)),
    );
}

/// Greške idu kao `{ "message": ... }` - frontend tu poruku prikazuje korisniku (npr. 409 u interceptoru)
fn odgovor_za_gresku(err: OrderError, kontekst: &str) -> HttpResponse {
    match err {
        OrderError::NotFound(msg) => HttpResponse::NotFound().json(json!({ "message": msg })),
        OrderError::BadRequest(msg) => HttpResponse::BadRequest().json(json!({ "message": msg })),
        OrderError::InsufficientStock(msg) => {
            HttpResponse::Conflict().json(json!({ "message": msg }))
        }
        OrderError::Database(e) => {
            log::error!("Greška pri {}: {:?}", kontekst, e);
            HttpResponse::InternalServerError().json(json!({ "message": "Greška na serveru" }))
        }
    }
}

async fn create_order(ctx: Data<AppContext>, req: Json<OrderRequest>) -> HttpResponse {
    let service = ctx.order_service();
    match service.create_order(ctx.pool(), req.into_inner()).await {
        Ok(order) => HttpResponse::Created().json(order),
        Err(e) => odgovor_za_gresku(e, "kreiranju porudžbine"),
    }
}

async fn today_orders(ctx: Data<AppContext>) -> HttpResponse {
    let service = ctx.order_service();
    match service.today_orders().await {
        Ok(orders) => HttpResponse::Ok().json(orders),
        Err(e) => odgovor_za_gresku(e, "čitanju današnjih porudžbina"),
    }
}

async fn today_revenue(ctx: Data<AppContext>) -> HttpResponse {
    let service = ctx.order_service();
    match service.today_revenue().await {
        Ok(revenue) => HttpResponse::Ok().json(revenue),
        Err(e) => odgovor_za_gresku(e, "računanju današnjeg pazara"),
    }
}

async fn update_type(
    ctx: Data<AppContext>,
    path: Path<u64>,
    query: Query<OrderTypeQuery>,
) -> HttpResponse {
    let service = ctx.order_service();
    match service
        .update_type(path.into_inner(), query.into_inner().new_type)
        .await
    {
        Ok(order) => HttpResponse::Ok().json(order),
        Err(e) => odgovor_za_gresku(e, "promeni tipa porudžbine"),
    }
}
