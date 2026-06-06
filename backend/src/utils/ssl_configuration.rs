use rustls::ServerConfig;
use rustls_pki_types::{CertificateDer, PrivateKeyDer};
use std::fs::File;
use std::io::BufReader;

pub fn ucitaj_ssl_konfiguraciju() -> ServerConfig {
    // 1. Otvaranje generisanih .pem fajlova
    let cert_file = &mut BufReader::new(
        File::open("cert.pem").expect("Nije moguće otvoriti cert.pem u root folderu"),
    );
    let key_file = &mut BufReader::new(
        File::open("key.pem").expect("Nije moguće otvoriti key.pem u root folderu"),
    );

    // 2. Čitanje sertifikata
    let cert_chain: Vec<CertificateDer> = rustls_pemfile::certs(cert_file)
        .collect::<Result<_, _>>()
        .expect("Neispravan cert.pem format");

    // 3. Čitanje privatnog ključa
    let key: PrivateKeyDer = rustls_pemfile::private_key(key_file)
        .expect("Greška pri parsiranju ključa")
        .expect("Nije pronađen privatni ključ u key.pem");

    // 4. Inicijalizacija Actix-kompatibilnog Ring provajdera za Rustls 0.23
    let crypto_provider = rustls::crypto::ring::default_provider();

    ServerConfig::builder_with_provider(crypto_provider.into())
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(cert_chain, key)
        .expect("Neuspešno konfigurisanje SSL-a")
}
