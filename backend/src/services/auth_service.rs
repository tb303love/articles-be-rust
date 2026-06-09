use crate::config::AppConfigProvider;
use crate::dtos::auth_dtos::RefreshRequest;
use crate::dtos::{AuthResponse, LoginRequest};
use crate::repositories::{RefreshTokenRepository, UserRepository};
use crate::utils::auth_middleware::Claims;
use bcrypt::verify;
use chrono::Utc;
use jsonwebtoken::{encode, EncodingKey, Header};
use sqlx::MySqlPool;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub struct AuthService {
    user_repository: Arc<dyn UserRepository + Send + Sync>,
    token_repository: Arc<dyn RefreshTokenRepository + Send + Sync>,
    _pool: MySqlPool,
    config: Arc<dyn AppConfigProvider>,
}

impl AuthService {
    pub fn new(
        user_repository: Arc<dyn UserRepository + Send + Sync>,
        token_repository: Arc<dyn RefreshTokenRepository + Send + Sync>,
        pool: MySqlPool,
        config: Arc<dyn AppConfigProvider>,
    ) -> Self {
        Self {
            user_repository,
            token_repository,
            _pool: pool,
            config,
        }
    }

    // 1. POMOĆNA FUNKCIJA: Generisanje i čuvanje refresh tokena direktno sirovim SQL-om
    async fn generisi_i_sacuvaj_refresh_token(&self, user_id: i64) -> Result<String, String> {
        // 1. Generišemo nasumičan jedinstveni string
        let novi_token = Uuid::new_v4().to_string();

        // 2. Računamo vreme isticanja (7 dana od sada)
        let expiry_date = (Utc::now() + chrono::Duration::days(7)).naive_utc();

        // 3. Pozivamo repozitorijum koji sve to bezbedno kroz transakciju upisuje u MySQL
        self.token_repository
            .save(&novi_token, expiry_date, user_id)
            .await
            .map_err(|e| format!("Kritična greška pri čuvanju tokena: {:?}", e))?;

        // Vraćamo token kako bi ga login ili refresh metode poslale Angularu
        Ok(novi_token)
    }

    pub fn dobij_jwt_secret(&self) -> String {
        self.config.get_jwt_secret().to_string()
    }

    pub async fn login(&self, dto: LoginRequest) -> Result<AuthResponse, String> {
        let user = self
            .user_repository
            .find_by_username(&dto.username)
            .await
            .map_err(|e| format!("Greška na serveru: {:?}", e))?
            .ok_or_else(|| "Neispravno korisničko ime ili lozinka".to_string())?;

        let lozinka_ispravna = verify(&dto.password, &user.password)
            .map_err(|_| "Greška pri verifikaciji lozinke".to_string())?;

        if !lozinka_ispravna {
            return Err("Neispravno korisničko ime ili lozinka".to_string());
        }

        let user_id = user.id.unwrap_or(0);

        // NOVO: Poziv privatne funkcije
        let access_token = self.kreiraj_jwt(user_id, &user.username)?;
        let refresh_token = self.generisi_i_sacuvaj_refresh_token(user_id).await?;

        Ok(AuthResponse {
            access_token,
            refresh_token,
            username: user.username,
            role: user.role,
        })
    }

    pub async fn osvezi_token(&self, dto: RefreshRequest) -> Result<AuthResponse, String> {
        // 1. Čitanje preko repozitorijuma
        let podaci = self
            .token_repository
            .find_with_user(&dto.refresh_token)
            .await
            .map_err(|e| format!("Greška na serveru: {:?}", e))?
            .ok_or_else(|| "Nevalidan refresh token".to_string())?;

        // 2. Validacija datuma isteka
        if podaci.expiry_date < Utc::now().naive_utc() {
            let _ = self.token_repository.delete(&dto.refresh_token).await;
            return Err("Refresh token je istekao".to_string());
        }

        // 3. NOVO: Generisanje novog JWT-a preko zajedničke funkcije
        let access_token = self.kreiraj_jwt(podaci.user_id, &podaci.username)?;

        // 4. Rotacija tokena
        let _ = self.token_repository.delete(&dto.refresh_token).await;
        let novi_refresh_token = self
            .generisi_i_sacuvaj_refresh_token(podaci.user_id)
            .await?;

        Ok(AuthResponse {
            access_token,
            refresh_token: novi_refresh_token,
            username: podaci.username,
            role: podaci.role,
        })
    }

    fn kreiraj_jwt(&self, user_id: i64, username: &str) -> Result<String, String> {
        let trenutno_vreme = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Token važi 15 minuta (900 sekundi) jer imamo refresh token sistem
        let trajanje_tokena = 15 * 60;
        let exp = trenutno_vreme + trajanje_tokena;

        let claims = Claims {
            sub: user_id,
            username: username.to_string(),
            exp,
        };

        // TODO: Povlačiti ključ iz AppConfig-a
        let secret = self.config.get_jwt_secret().as_bytes();

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret),
        )
        .map_err(|e| format!("Neuspešno generisanje tokena: {:?}", e))
    }

    pub async fn logout(&self, dto: RefreshRequest) -> Result<(), String> {
        self.token_repository
            .delete(&dto.refresh_token)
            .await
            .map_err(|e| format!("Greška pri brisanju tokena iz baze: {:?}", e))?;

        Ok(())
    }
}
