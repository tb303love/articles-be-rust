// src/config.rs
use std::env;
use winreg::enums::*;
use winreg::RegKey;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub server_ip: String,
    pub app_env: String,
    pub server_port: u16,
    pub database_url: String,
    pub frontend_dir: String,
    pub log_dir: String,
    pub jwt_secret: String, // Bezbedno preuzet i konfigurisan tajni ključ
    pub token_best_before: u8,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let args: Vec<String> = env::args().collect();
        let pokrenuto_lokalno = args.iter().any(|arg| arg == "local" || arg == "--dev");

        let run_env_var = env::var("RUN_ENV").unwrap_or_default();
        let development_env = pokrenuto_lokalno || run_env_var == "development";
        let token_duration_str = env::var("TOKEN_BEST_BEFORE").unwrap_or_else(|_| "24".to_string());
        let token_best_before: u8 = token_duration_str
            .parse()
            .expect("TOKEN_BEST_BEFORE mora biti validan broj između 0 i 255");

        if development_env {
            let _ = dotenvy::from_filename(".env.development");
            let default_dev_frontend = "./target/release/frontend".to_string();
            let frontend_dir = env::var("FRONTEND_DIR").unwrap_or(default_dev_frontend);
            let log_dir = env::var("LOG_DIR").unwrap_or_else(|_| ".".to_string());

            return Self {
                server_ip: env::var("SERVER_IP").unwrap_or_else(|_| "127.0.0.1".to_string()),
                app_env: "development".to_string(),
                server_port: env::var("SERVER_PORT")
                    .unwrap_or_else(|_| "8443".to_string())
                    .parse()
                    .unwrap_or(8443),
                database_url: env::var("DATABASE_URL")
                    .unwrap_or_else(|_| "mysql://root:pass@localhost/db".to_string()),
                frontend_dir,
                log_dir,
                jwt_secret: env::var("JWT_SECRET").unwrap_or_else(|_| String::from("asd123")),
                token_best_before,
            };
        }

        // PRODUKCIJA (Windows Servis)
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        let subkey = match hklm.open_subkey("SOFTWARE\\OgilenkoServer2") {
            Ok(key) => key,
            Err(_) => {
                log::warn!(
                    "Registry ključ 'SOFTWARE\\OgilenkoServer2' nije pronađen. Koristim fallback."
                );
                return Self::fallback_production();
            }
        };

        let server_ip: String = subkey
            .get_value("SERVER_IP")
            .unwrap_or_else(|_| "0.0.0.0".to_string());
        let app_env: String = subkey
            .get_value("APP_ENV")
            .unwrap_or_else(|_| "production".to_string());

        let server_port: u16 = match subkey.get_value::<u32, _>("SERVER_PORT") {
            Ok(port_u32) => port_u32 as u16,
            Err(_) => {
                let port_str: String = subkey
                    .get_value("SERVER_PORT")
                    .unwrap_or_else(|_| "443".to_string());
                port_str.parse().unwrap_or(443)
            }
        };

        let database_url: String = subkey
            .get_value("DATABASE_URL")
            .unwrap_or_else(|_| "mysql://root:root_password@127.0.0.1/ogilenko_db".to_string());
        let frontend_dir: String = subkey
            .get_value("FRONTEND_DIR")
            .unwrap_or_else(|_| "./frontend".to_string());
        let log_dir: String = subkey
            .get_value("LOG_DIR")
            .unwrap_or_else(|_| ".".to_string());

        // STRATEGIJA ZA JWT_SECRET U PRODUKCIJI:
        // 1. Prvo probamo da pročitamo string vrednost direktno iz Windows Registry podključa.
        // 2. Ako ne postoji u Registry-ju, gledamo sistemske env varijable.
        // 3. Ako ni tamo nema ničega, koristimo bezbedni generički fallback kako login ne bi pucao.
        let jwt_secret: String = subkey.get_value("JWT_SECRET").unwrap_or_else(|_| {
            env::var("JWT_SECRET")
                .unwrap_or_else(|_| String::from("privremeni_produkcijski_kljuc_promeni_me"))
        });

        Self {
            server_ip,
            app_env,
            server_port,
            database_url,
            frontend_dir,
            log_dir,
            jwt_secret,
            token_best_before,
        }
    }

    fn fallback_production() -> Self {
        Self {
            server_ip: "0.0.0.0".to_string(),
            app_env: "production".to_string(),
            server_port: 443,
            database_url: "mysql://root:root_password@127.0.0.1/ogilenko_db".to_string(),
            frontend_dir: "./frontend".to_string(),
            log_dir: ".".to_string(),
            jwt_secret: env::var("JWT_SECRET")
                .unwrap_or_else(|_| String::from("privremeni_produkcijski_kljuc_promeni_me")),
            token_best_before: 7,
        }
    }
}
