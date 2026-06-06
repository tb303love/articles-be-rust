use std::env;
use winreg::enums::*;
use winreg::RegKey;

pub struct AppConfig {
    pub server_ip: String,
    pub app_env: String,
    pub server_port: u16,
    pub database_url: String,
    pub frontend_dir: String, // Dodato polje za frontend
}

impl AppConfig {
    pub fn from_env() -> Self {
        // 1. Proveravamo argumente komandne linije (sinhronizovano sa main.rs)
        let args: Vec<String> = env::args().collect();
        let pokrenuto_lokalno = args.iter().any(|arg| arg == "local" || arg == "--dev");

        // Takođe proveravamo RUN_ENV kao sekundarnu opciju za svaki slučaj
        let run_env_var = env::var("RUN_ENV").unwrap_or_default();
        let je_development = pokrenuto_lokalno || run_env_var == "development";

        if je_development {
            // Lokalni razvoj: učitavamo .env.development datoteku
            let _ = dotenvy::from_filename(".env.development");

            // Za lokalni razvoj podrazumevamo putanju gde se nalazi vaš Angular kod van rust projekta.
            // Ako u .env.development fajlu ipak definišete FRONTEND_DIR, uzeće se ta vrednost.
            let default_dev_frontend = "./frontend".to_string();
            let frontend_dir = env::var("FRONTEND_DIR").unwrap_or(default_dev_frontend);

            return Self {
                server_ip: env::var("SERVER_IP").unwrap_or_else(|_| "127.0.0.1".to_string()),
                app_env: "development".to_string(),
                server_port: env::var("SERVER_PORT")
                    .unwrap_or_else(|_| "8443".to_string())
                    .parse()
                    .unwrap_or(8443),
                database_url: env::var("DATABASE_URL")
                    .unwrap_or_else(|_| "mysql://root:pass@localhost/db".to_string()),
                frontend_dir, // Dodato u dev inicijalizaciju
            };
        }

        // 2. PRODUKCIJA (Windows Servis): Čitamo direktno iz Windows Registra!
        // Putanja u registru: HKEY_LOCAL_MACHINE\SOFTWARE\OgilenkoServer2
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

        let subkey = match hklm.open_subkey("SOFTWARE\\OgilenkoServer2") {
            Ok(key) => key,
            Err(_) => {
                log::warn!("Registry ključ 'SOFTWARE\\OgilenkoServer2' nije pronađen. Koristim fallback vrednosti.");
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

        // Čitanje putanje frontenda iz registra sa fallback-om na relativni "./frontend" folder tik uz .exe
        let frontend_dir: String = subkey
            .get_value("FRONTEND_DIR")
            .unwrap_or_else(|_| "./frontend".to_string());

        Self {
            server_ip,
            app_env,
            server_port,
            database_url,
            frontend_dir, // Dodato u produkcionu inicijalizaciju
        }
    }

    fn fallback_production() -> Self {
        Self {
            server_ip: "0.0.0.0".to_string(),
            app_env: "production".to_string(),
            server_port: 443,
            database_url: "mysql://root:root_password@127.0.0.1/ogilenko_db".to_string(),
            frontend_dir: "./frontend".to_string(), // Dodato u fallback
        }
    }
}
