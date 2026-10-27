use crate::context::AppContext;
use crate::dto::barcode_dto::BarcodeRequest;
use crate::error::barcode_error::BarcodeError;
use actix_web::web::{self, Data, Json};
use actix_web::HttpResponse;

pub fn rute(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/scans").route("/sales", web::post().to(sales)));
}

async fn sales(ctx: Data<AppContext>, body: Json<BarcodeRequest>) -> HttpResponse {
    let barcode_service = ctx.barcode_service();

    match barcode_service
        .handle_android_scan(body.into_inner().barcode)
        .await
    {
        Ok(valid_barcode) => HttpResponse::Ok().json(valid_barcode),
        Err(BarcodeError::NotFound) => HttpResponse::NotFound().body("Artikal nije pronađen."),
        Err(BarcodeError::DatabaseError(e)) => {
            log::error!("Greška na bazi unutar handlera: {}", e);
            HttpResponse::InternalServerError().json("Greška na serveru")
        }
    }
}
