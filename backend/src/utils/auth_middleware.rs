use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    error::ErrorUnauthorized,
    Error, HttpMessage,
};
use futures_util::future::LocalBoxFuture;
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::future::{ready, Ready};

// Struktura podataka koja se pakuje unutar JWT tokena (Claims)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: i64, // ID korisnika (User ID)
    pub username: String,
    pub exp: u64, // Vreme kada token ističe (Timestamp)
}

pub struct Authentication;

impl<S, B> Transform<S, ServiceRequest> for Authentication
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = AuthenticationMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthenticationMiddleware { service }))
    }
}

pub struct AuthenticationMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for AuthenticationMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // 1. Izvlacimo "Authorization" zaglavlje iz HTTP zahteva
        if let Some(auth_header) = req.headers().get("Authorization") {
            if let Ok(auth_str) = auth_header.to_str() {
                // Proveravamo da li krene sa "Bearer " prefixom
                if auth_str.starts_with("Bearer ") {
                    let token = auth_str.trim_start_matches("Bearer ");

                    // TODO: Ključ za dekriptovanje izvlačiti iz config.rs (AppConfig)
                    let secret = "moj_super_tajni_kljuc_za_jwt_123".as_bytes();

                    // 2. Validacija i dekodiranje tokena
                    match decode::<Claims>(
                        token,
                        &DecodingKey::from_secret(secret),
                        &Validation::default(),
                    ) {
                        Ok(token_data) => {
                            // Ako je token ispravan, ubacujemo claims podatke u req ekstenzije.
                            // Handleri mogu izvući ove podatke da znaju KOJI korisnik pravi zahtev.
                            req.extensions_mut().insert(token_data.claims);

                            let fut = self.service.call(req);
                            return Box::pin(async move {
                                let res = fut.await?;
                                Ok(res)
                            });
                        }
                        Err(_) => {
                            // Token je istekao ili je modifikovan
                            return Box::pin(ready(Err(ErrorUnauthorized(
                                "Nevalidan ili istekao token",
                            ))));
                        }
                    }
                }
            }
        }

        // Ako zaglavlje uopšte ne postoji ili je neispravno
        Box::pin(ready(Err(ErrorUnauthorized(
            "Nedostaje autorizacioni token",
        ))))
    }
}
