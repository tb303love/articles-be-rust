// src/error/api_error.rs
use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use serde_json::json;
use std::fmt;

/// Jedinstvena greška API-ja. Telo odgovora je uvek `{ "message": "..." }`.
/// Tehnički detalji (`Internal`) idu samo u log, korisnik dobija opštu poruku.
#[derive(Debug)]
pub enum ApiError {
    BadRequest(String),
    Unauthorized(String),
    NotFound(String),
    Conflict(String),
    PayloadTooLarge(String),
    Internal(String),
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiError::BadRequest(m)
            | ApiError::Unauthorized(m)
            | ApiError::NotFound(m)
            | ApiError::Conflict(m)
            | ApiError::PayloadTooLarge(m)
            | ApiError::Internal(m) => f.write_str(m),
        }
    }
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        match self {
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::Conflict(_) => StatusCode::CONFLICT,
            ApiError::PayloadTooLarge(_) => StatusCode::PAYLOAD_TOO_LARGE,
            ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let poruka = match self {
            ApiError::Internal(detalj) => {
                log::error!("Interna greška: {}", detalj);
                "Greška na serveru.".to_string()
            }
            _ => self.to_string(),
        };
        HttpResponse::build(self.status_code()).json(json!({ "message": poruka }))
    }
}

// MySQL brojevi grešaka koje korisnik može sam da izazove
const DUPLIKAT: u16 = 1062; // UNIQUE ključ
const RED_SE_KORISTI: u16 = 1451; // Brisanje reda na koji pokazuje FK (RESTRICT)
const VEZA_NE_POSTOJI: u16 = 1452; // FK pokazuje na nepostojeći red
const PREDUGACKO: u16 = 1406; // Tekst duži od kolone (npr. ime > 255 znakova)
const VAN_OPSEGA: u16 = 1264; // Broj ne staje u kolonu (npr. cena > decimal(19,4))

impl From<sqlx::Error> for ApiError {
    fn from(e: sqlx::Error) -> Self {
        if let sqlx::Error::RowNotFound = e {
            return ApiError::NotFound("Traženi podatak ne postoji.".into());
        }

        if let sqlx::Error::Database(db) = &e {
            if let Some(mysql) = db.try_downcast_ref::<sqlx::mysql::MySqlDatabaseError>() {
                match mysql.number() {
                    DUPLIKAT => return ApiError::Conflict(poruka_za_duplikat(mysql.message())),
                    VEZA_NE_POSTOJI => {
                        return ApiError::BadRequest(
                            "Povezani podatak ne postoji (npr. komponenta paketa ili kategorija)."
                                .into(),
                        )
                    }
                    RED_SE_KORISTI => {
                        return ApiError::Conflict(
                            "Podatak se koristi na drugom mestu i ne može da se obriše.".into(),
                        )
                    }
                    PREDUGACKO => {
                        return ApiError::BadRequest("Neka od unetih vrednosti je predugačka.".into())
                    }
                    VAN_OPSEGA => {
                        return ApiError::BadRequest("Neki od unetih brojeva je prevelik.".into())
                    }
                    _ => {}
                }
            }
        }

        ApiError::Internal(format!("{:?}", e))
    }
}

impl From<std::io::Error> for ApiError {
    fn from(e: std::io::Error) -> Self {
        ApiError::Internal(format!("I/O greška: {:?}", e))
    }
}

/// MySQL poruka izgleda ovako: "Duplicate entry 'Kafa' for key 'article.uk_article_name'"
fn poruka_za_duplikat(mysql_poruka: &str) -> String {
    let vrednost = mysql_poruka
        .split_once("entry '")
        .and_then(|(_, ostatak)| ostatak.rsplit_once("' for key"))
        .map(|(v, _)| v);

    match vrednost {
        Some(v) if mysql_poruka.contains("uk_article_name") => {
            format!("Artikal \"{}\" već postoji.", v)
        }
        Some(v) if mysql_poruka.contains("uk_article_barcodes_barcode") => {
            format!("Barkod {} je već dodeljen drugom artiklu.", v)
        }
        Some(v) if mysql_poruka.contains("uk_users_username") => {
            format!("Korisničko ime \"{}\" je zauzeto.", v)
        }
        Some(v) if mysql_poruka.contains("uk_category_name_name") => {
            format!("Kategorija \"{}\" već postoji.", v)
        }
        _ => "Podatak već postoji.".to_string(),
    }
}
