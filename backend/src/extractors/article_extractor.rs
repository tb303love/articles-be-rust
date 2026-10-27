use crate::error::ApiError;
use crate::models::ImageUploadData;
use actix_web::{dev::Payload, web::BytesMut, Error as ActixError, FromRequest, HttpRequest};
use futures_util::{future::LocalBoxFuture, FutureExt, TryStreamExt};
use serde::de::DeserializeOwned;

pub struct ArticleUpload<T> {
    pub dto: T,
    pub slika: Option<ImageUploadData>,
}

/// Prekinut ili neispravan multipart strim je greška klijenta, ne "prazna forma"
fn greska_strima(e: actix_multipart::MultipartError) -> ApiError {
    ApiError::BadRequest(format!("Neispravan multipart zahtev: {}", e))
}

impl<T> FromRequest for ArticleUpload<T>
where
    T: DeserializeOwned + 'static,
{
    type Error = ActixError;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    // ISPRAVLJENO: Umesto Self::Future, napisan je pun, konkretan tip povrata
    fn from_request(
        req: &HttpRequest,
        payload: &mut Payload,
    ) -> LocalBoxFuture<'static, Result<Self, Self::Error>> {
        // ISPRAVLJENO: Nema match bloka jer Multipart::new ne vraća Result
        let mut multipart = actix_multipart::Multipart::new(req.headers(), payload.take());

        async move {
            let mut dto_opt: Option<T> = None;
            let mut slika_podaci: Option<ImageUploadData> = None;

            // Koristimo try_next() za bezbedno kretanje kroz polja forme
            while let Some(mut field) = multipart.try_next().await.map_err(greska_strima)? {
                let name = field.name().unwrap_or("");

                match name {
                    "article" => {
                        let mut bytes = BytesMut::new();
                        // ISPRAVLJENO: Koristimo try_next() i za čitanje chunk-ova unutar polja
                        // Ovo garantuje da se strim isprazni do kraja bez zaglavljivanja multipart-a
                        while let Some(chunk) = field.try_next().await.map_err(greska_strima)? {
                            bytes.extend_from_slice(&chunk);
                        }

                        let parsed_dto = serde_json::from_slice::<T>(&bytes).map_err(|e| {
                            ApiError::BadRequest(format!("Neispravan JSON u polju 'article': {}", e))
                        })?;
                        dto_opt = Some(parsed_dto);
                    }
                    "image" => {
                        let original_name = field
                            .content_disposition()
                            .and_then(|cd| cd.get_filename().map(|fname| fname.to_string()));

                        let content_type = field
                            .content_type()
                            .map(|ct| ct.to_string())
                            .unwrap_or_else(|| "image/jpeg".to_string());

                        match original_name.filter(|n| !n.is_empty()) {
                            Some(name) => {
                                // Fajl se upisuje na disk i služi preko /images - prihvatamo samo slike
                                if !content_type.starts_with("image/") {
                                    return Err(ApiError::BadRequest(format!(
                                        "Fajl '{}' nije slika (tip: {}).",
                                        name, content_type
                                    ))
                                    .into());
                                }

                                let mut bytes = Vec::new();
                                // ISPRAVLJENO: Koristimo try_next() i ovde za čitanje bajtova slike
                                while let Some(chunk) =
                                    field.try_next().await.map_err(greska_strima)?
                                {
                                    bytes.extend_from_slice(&chunk);
                                }
                                if !bytes.is_empty() {
                                    slika_podaci = Some(ImageUploadData {
                                        bytes,
                                        original_name: name,
                                        content_type,
                                    });
                                }
                            }
                            // Polje bez fajla (prazan input) i dalje mora da se isprazni
                            None => while field.try_next().await.map_err(greska_strima)?.is_some() {},
                        }
                    }
                    _ => {
                        // OBAVEZNO: Ako stigne bilo koje drugo polje, moramo ga potpuno isprazniti
                        // da ne bi blokiralo sledeći try_next() na multipart strimu
                        while field.try_next().await.map_err(greska_strima)?.is_some() {}
                    }
                }
            }

            let dto = dto_opt.ok_or_else(|| {
                ApiError::BadRequest("Nedostaje obavezno form-data polje 'article'.".into())
            })?;

            Ok(ArticleUpload {
                dto,
                slika: slika_podaci,
            })
        }
        .boxed_local()
    }
}
