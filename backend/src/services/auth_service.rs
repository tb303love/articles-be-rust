// src/services/auth_service.rs
use crate::dtos::{AuthResponse, LoginRequest};
use crate::repositories::UserRepository;
use crate::utils::auth_middleware::Claims;
use bcrypt::verify;
use jsonwebtoken::{encode, EncodingKey, Header};
use sqlx::MySqlPool;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct AuthService {
    user_repository: Arc<dyn UserRepository + Send + Sync>,
    _private_pool: MySqlPool,
}

impl AuthService {
    pub fn new(user_repository: Arc<dyn UserRepository + Send + Sync>, pool: MySqlPool) -> Self {
        Self {
            user_repository,
            _private_pool: pool,
        }
    }

    pub async fn login(&self, dto: LoginRequest) -> Result<AuthResponse, String> {
        // 1. Pronađi korisnika u bazi preko repozitorijuma
        let user = self
            .user_repository
            .find_by_username(&dto.username)
            .await
            .map_err(|e| format!("Greška na serveru: {:?}", e))?
            .ok_or_else(|| "Neispravno korisničko ime ili lozinka".to_string())?;

        // 2. Proveri lozinku (poredi čist tekst sa bcrypt hešom iz baze)
        let lozinka_ispravna = verify(&dto.password, &user.password)
            .map_err(|_| "Greška pri verifikaciji lozinke".to_string())?;

        if !lozinka_ispravna {
            return Err("Neispravno korisničko ime ili lozinka".to_string());
        }

        // 3. Generiši JWT token ako je sve u redu
        let trenutno_vreme = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let trajanje_tokena = 24 * 3600; // Token važi 24 sata
        let exp = trenutno_vreme + trajanje_tokena;

        let claims = Claims {
            sub: user.id.unwrap_or(0),
            username: user.username.clone(),
            exp,
        };

        // TODO: Tajni ključ treba povlačiti iz config.rs (AppConfig) kako bi bio usklađen sa middleware-om
        let secret = "moj_super_tajni_kljuc_za_jwt_123".as_bytes();

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret),
        )
        .map_err(|e| format!("Neuspešno generisanje tokena: {:?}", e))?;

        Ok(AuthResponse {
            token,
            username: user.username,
            role: user.role,
        })
    }
}
