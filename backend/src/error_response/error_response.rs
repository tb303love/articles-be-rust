use crate::error::ApiError;
use actix_web::{
    body::{BodySize, MessageBody},
    dev::ServiceResponse,
    error::JsonPayloadError,
    middleware::ErrorHandlerResponse,
    web::{JsonConfig, PathConfig, QueryConfig},
    ResponseError,
};

/// Podrazumevanih 2 MB je premalo za Excel uvoz sa nekoliko hiljada artikala
const JSON_LIMIT: usize = 10 * 1024 * 1024;

/// Nepostojeća ruta dobija JSON poruku. Handler koji je sam vratio 404
/// (npr. "Artikal ne postoji.") već ima telo odgovora - njega ne diramo.
pub fn _404<B: MessageBody + 'static>(
    res: ServiceResponse<B>,
) -> actix_web::Result<ErrorHandlerResponse<B>> {
    if !matches!(res.response().body().size(), BodySize::None | BodySize::Sized(0)) {
        return Ok(ErrorHandlerResponse::Response(res.map_into_left_body()));
    }

    let (req, _) = res.into_parts();
    let odgovor = ApiError::NotFound("Putanja ne postoji.".into()).error_response();
    Ok(ErrorHandlerResponse::Response(
        ServiceResponse::new(req, odgovor).map_into_right_body(),
    ))
}

// --- GREŠKE EKSTRAKTORA ---
// Bez ovoga actix vraća sopstveni plain text umesto `{ "message" }`

pub fn json_config() -> JsonConfig {
    JsonConfig::default()
        .limit(JSON_LIMIT)
        .error_handler(|err, _req| {
            let greska = match &err {
                JsonPayloadError::Overflow { .. } | JsonPayloadError::OverflowKnownLength { .. } => {
                    ApiError::PayloadTooLarge(format!(
                        "Zahtev je veći od dozvoljenih {} MB.",
                        JSON_LIMIT / 1024 / 1024
                    ))
                }
                JsonPayloadError::ContentType => ApiError::BadRequest(
                    "Zahtev mora biti JSON (Content-Type: application/json).".into(),
                ),
                JsonPayloadError::Deserialize(e) => {
                    ApiError::BadRequest(format!("Neispravan JSON: {}", e))
                }
                _ => ApiError::BadRequest(format!("Neispravan JSON zahtev: {}", err)),
            };
            greska.into()
        })
}

/// Npr. `/check-name` bez `name` ili `?newType=NESTO`
pub fn query_config() -> QueryConfig {
    QueryConfig::default().error_handler(|err, _req| {
        ApiError::BadRequest(format!("Neispravan parametar upita: {}", err)).into()
    })
}

/// Npr. `/orders/abc/type` - actix bi inače vratio 404 kao da ruta ne postoji
pub fn path_config() -> PathConfig {
    PathConfig::default().error_handler(|err, _req| {
        ApiError::BadRequest(format!("Neispravna vrednost u putanji: {}", err)).into()
    })
}
