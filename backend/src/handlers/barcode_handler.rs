use crate::context::AppContext;
use crate::dto::barcode_dto::BarcodeRequest;
use crate::error::ApiError;
use actix_web::web::{self, Data, Json};
use actix_web::HttpResponse;

pub fn rute(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/scans").route("/sales", web::post().to(sales)));
}

async fn sales(
    ctx: Data<AppContext>,
    body: Json<BarcodeRequest>,
) -> Result<HttpResponse, ApiError> {
    let valid_barcode = ctx
        .barcode_service()
        .handle_android_scan(body.into_inner().barcode)
        .await?;
    Ok(HttpResponse::Ok().json(valid_barcode))
}
