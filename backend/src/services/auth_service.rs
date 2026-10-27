// src/services/auth_service.rs

use crate::config::AppConfig;
use crate::dto::auth_dto::RefreshRequest;
use crate::dto::{AuthResponse, LoginRequest};
use crate::error::ApiError;
use crate::models::UserRole;
use crate::repositories::{SqlxRefreshTokenRepository, SqlxUserRepository};
use crate::utils::auth_middleware::Claims;
use bcrypt::verify;
use chrono::Utc;
use jsonwebtoken::{encode, EncodingKey, Header};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub struct AuthService {
    user_repository: Arc<SqlxUserRepository>,
    token_repository: Arc<SqlxRefreshTokenRepository>,
    config: Arc<AppConfig>,
}

impl AuthService {
    pub fn new(
        user_repository: Arc<SqlxUserRepository>,
        token_repository: Arc<SqlxRefreshTokenRepository>,
        config: Arc<AppConfig>,
    ) -> Self {
        Self {
            user_repository,
            token_repository,
            config,
        }
    }

    pub fn validiraj_jwt(&self, token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
       use jsonwebtoken::{decode, DecodingKey, Validation};

       let secret = self.config.jwt_secret.as_bytes();
        decode::<Claims>(
            token,
            &DecodingKey::from_secret(secret),
            &Validation::default(),
        )
        .map(|data| data.claims)
    }

    async fn generate_and_save_refresh_token(&self, user_id: u64) -> Result<String, ApiError> {
        let novi_token = Uuid::new_v4().to_string();

        let trajanje_od_sedam_dana =
            chrono::Duration::try_days(self.config.token_best_before as i64).ok_or_else(|| {
                ApiError::Internal("Neuspešno računanje trajanja refresh tokena".into())
            })?;

        let expiry_date = (Utc::now() + trajanje_od_sedam_dana).naive_utc();

        self.token_repository
            .save(&novi_token, expiry_date, user_id)
            .await?;

        Ok(novi_token)
    }

    pub async fn login(&self, dto: LoginRequest) -> Result<AuthResponse, ApiError> {
        // Ista poruka za nepostojećeg korisnika i pogrešnu lozinku - ne otkrivamo koja korisnička imena postoje
        let pogresni_podaci = || ApiError::Unauthorized("Neispravno korisničko ime ili lozinka.".into());

        let user = self
            .user_repository
            .find_by_username(&dto.username)
            .await?
            .ok_or_else(pogresni_podaci)?;

        // Greška ovde znači neispravan hash u bazi, ne pogrešnu lozinku
        let lozinka_ispravna = verify(&dto.password, &user.password).map_err(|e| {
            ApiError::Internal(format!("Neispravan hash lozinke za '{}': {:?}", user.username, e))
        })?;

        if !lozinka_ispravna {
            return Err(pogresni_podaci());
        }

        // Sigurnija provera primarnog ključa
        let user_id = user
            .id
            .ok_or_else(|| ApiError::Internal("Korisnik učitan iz baze nema ID".into()))?;

        let access_token = self.create_jwt(user_id, &user.username, &user.role)?;
        let refresh_token = self.generate_and_save_refresh_token(user_id).await?;

        Ok(AuthResponse {
            access_token,
            refresh_token,
            username: user.username,
            role: user.role,
        })
    }

    pub async fn osvezi_token(&self, dto: RefreshRequest) -> Result<AuthResponse, ApiError> {
        let podaci = self
            .token_repository
            .find_with_user(&dto.refresh_token)
            .await?
            .ok_or_else(|| ApiError::Unauthorized("Nevalidan refresh token.".into()))?;

        if podaci.expiry_date < Utc::now().naive_utc() {
            // Neuspelo brisanje isteklog tokena nije razlog da korisnik ne dobije pravu grešku
            if let Err(e) = self.token_repository.delete(&dto.refresh_token).await {
                log::warn!("Brisanje isteklog refresh tokena neuspešno: {:?}", e);
            }
            return Err(ApiError::Unauthorized("Refresh token je istekao.".into()));
        }

        let access_token = self.create_jwt(podaci.user_id, &podaci.username, &podaci.role)?;

        // Rotacija tokena - stari token mora da nestane, inače bi mogao ponovo da se iskoristi
        self.token_repository.delete(&dto.refresh_token).await?;
        let novi_refresh_token = self.generate_and_save_refresh_token(podaci.user_id).await?;

        Ok(AuthResponse {
            access_token,
            refresh_token: novi_refresh_token,
            username: podaci.username,
            role: podaci.role,
        })
    }

    fn create_jwt(&self, user_id: u64, username: &str, u_role: &str) -> Result<String, ApiError> {
        let trenutno_vreme = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let trajanje_tokena = 15 * 60;
        let exp = trenutno_vreme + trajanje_tokena;
        let role = match u_role.to_lowercase().as_str() {
            "admin" => UserRole::Admin,
            _ => UserRole::User,
        };

        let claims = Claims {
            sub: user_id,
            username: username.to_string(),
            exp,
            role,
        };

        let secret = self.config.jwt_secret.as_bytes();

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret),
        )
        .map_err(|e| ApiError::Internal(format!("Neuspešno generisanje JWT-a: {:?}", e)))
    }

    pub async fn logout(&self, dto: RefreshRequest) -> Result<(), ApiError> {
        self.token_repository.delete(&dto.refresh_token).await?;
        Ok(())
    }
}
