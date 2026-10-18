// src/utils/auth_middleware.rs (ili gde se već nalazi tvoj middleware)

use crate::context::AppContext; // Pretpostavka da je ovde tvoj AppContext
use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    error::ErrorUnauthorized,
    web, Error, HttpMessage,
};
use futures_util::future::LocalBoxFuture;
use serde::{Deserialize, Serialize};
use std::future::{ready, Ready};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: u64,
    pub username: String,
    pub exp: u64,
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
        // Izvlačimo auth_service direktno iz konteksta aplikacije
        let auth_service = if let Some(ctx) = req.app_data::<web::Data<AppContext>>() {
            ctx.auth_service()
        } else {
            log::error!("Kritična greška u middleware-u: AppContext nije pronađen!");
            return Box::pin(ready(Err(ErrorUnauthorized("Interna greška servera"))));
        };

        if let Some(auth_header) = req.headers().get("Authorization") {
            if let Ok(auth_str) = auth_header.to_str() {
                if auth_str.starts_with("Bearer ") {
                    let token = auth_str.trim_start_matches("Bearer ");

                    // 🧠 FIKSIRANO: Pozivamo novu metodu iz servisa koja interno koristi .env tajnu!
                    return match auth_service.validiraj_jwt(token) {
                        Ok(claims) => {
                            // Ako je token ispravan, ubacujemo claims podatke u req ekstenzije.
                            req.extensions_mut().insert(claims);

                            let fut = self.service.call(req);
                            Box::pin(async move {
                                let res = fut.await?;
                                Ok(res)
                            })
                        }
                        Err(_) => {
                            // Token je istekao ili je modifikovan
                            Box::pin(ready(Err(ErrorUnauthorized("Nevalidan ili istekao token"))))
                        }
                    };
                }
            }
        }

        Box::pin(ready(Err(ErrorUnauthorized("Nedostaje autorizacioni token"))))
    }
}
