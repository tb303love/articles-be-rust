// src/ws/protocol.rs
use serde::{Deserialize, Serialize};

/// Sve što klijent (Angular) može da POŠALJE serveru
#[derive(Deserialize, Debug)]
#[serde(tag = "tip", content = "podaci")]
pub enum KlijentPoruka {
    SkeniranBarkod { kod: String },
    PridruziSeGrupi { ime_grupe: String },
    NapustiGrupu { ime_grupe: String },
}

/// Sve što server može da VRATI klijentu
#[derive(Serialize, Debug, Clone)]
#[serde(tag = "tip", content = "podaci")]
pub enum ServerPoruka {
    BarkodProcesiran { kod: String, status: String },
    Obavestenje { tekst: String },
    Greska { poruka: String },
}
