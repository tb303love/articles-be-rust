use crate::context::AppContext;
use crate::dto::order_dto::{OrderRequest, OrderTypeQuery};
use crate::error::ApiError;
use actix_web::{
   web::{self, get, patch, post, Data, Json, Path, Query},
   HttpResponse,
};

pub fn rute(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/orders")
            .route("", post().to(create_order))
            .route("/today", get().to(today_orders))
            .route("/revenue/today", get().to(today_revenue))
            .route("/{id}/type", patch().to(update_type)),
    );
}

async fn create_order(
    ctx: Data<AppContext>,
    req: Json<OrderRequest>,
) -> Result<HttpResponse, ApiError> {
    let order = ctx
        .order_service()
        .create_order(ctx.pool(), req.into_inner())
        .await?;
    Ok(HttpResponse::Created().json(order))
}

async fn today_orders(ctx: Data<AppContext>) -> Result<HttpResponse, ApiError> {
    let orders = ctx.order_service().today_orders().await?;
    Ok(HttpResponse::Ok().json(orders))
}

async fn today_revenue(ctx: Data<AppContext>) -> Result<HttpResponse, ApiError> {
    let revenue = ctx.order_service().today_revenue().await?;
    Ok(HttpResponse::Ok().json(revenue))
}

async fn update_type(
    ctx: Data<AppContext>,
    path: Path<u64>,
    query: Query<OrderTypeQuery>,
) -> Result<HttpResponse, ApiError> {
    let order = ctx
        .order_service()
        .update_type(path.into_inner(), query.into_inner().new_type)
        .await?;
    Ok(HttpResponse::Ok().json(order))
}
