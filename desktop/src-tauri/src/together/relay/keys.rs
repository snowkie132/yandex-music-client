use std::env;

use ed25519_dalek::SigningKey;
#[cfg(desktop)]
use keyring::Entry;
use together_crypto::encoding::to_base64;
use together_crypto::identity;

pub const SERVICE: &str = "mashiro-together";

pub const ACCOUNT: &str = "seed-phrase";

const ACCOUNT_ENV: &str = "MASHIRO_TOGETHER_ACCOUNT";

const SEED_ENV: &str = "MASHIRO_TOGETHER_SEED";

fn account() -> String {
    match env::var(ACCOUNT_ENV) {
        Ok(suffix) if !suffix.trim().is_empty() => format!("{ACCOUNT}-{}", suffix.trim()),
        _ => ACCOUNT.to_string(),
    }
}

fn env_seed() -> Option<String> {
    let raw = env::var(SEED_ENV).ok()?;
    let normalized = identity::normalize_phrase(&raw);
    if normalized.is_empty() {
        return None;
    }

    identity::keypair_from_phrase(&normalized).ok()?;
    Some(normalized)
}

#[cfg(desktop)]
fn entry() -> Result<Entry, String> {
    Entry::new(SERVICE, &account())
        .map_err(|error| format!("хранилище ключей недоступно: {error}"))
}

#[cfg(not(desktop))]
fn seed_path() -> Result<std::path::PathBuf, String> {
    let dir = std::env::temp_dir().join("mashiro-together");
    std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    Ok(dir.join(format!("{}.seed", account())))
}

pub fn save(phrase: &str) -> Result<String, String> {
    let normalized = identity::normalize_phrase(phrase);
    identity::keypair_from_phrase(&normalized).map_err(|error| error.to_string())?;

    #[cfg(desktop)]
    entry()?
        .set_password(&normalized)
        .map_err(|error| format!("не удалось сохранить сид-фразу: {error}"))?;

    #[cfg(not(desktop))]
    std::fs::write(seed_path()?, normalized.as_bytes())
        .map_err(|error| format!("не удалось сохранить сид-фразу: {error}"))?;

    Ok(normalized)
}

pub fn load() -> Result<Option<String>, String> {
    if let Some(seed) = env_seed() {
        return Ok(Some(seed));
    }

    #[cfg(desktop)]
    {
        match entry()?.get_password() {
            Ok(phrase) => Ok(Some(phrase)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(format!("не удалось прочитать хранилище ключей: {error}")),
        }
    }

    #[cfg(not(desktop))]
    {
        let path = seed_path()?;
        if !path.exists() {
            return Ok(None);
        }
        let phrase = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
        let trimmed = phrase.trim().to_string();
        if trimmed.is_empty() {
            Ok(None)
        } else {
            Ok(Some(trimmed))
        }
    }
}

pub fn forget() -> Result<(), String> {
    #[cfg(desktop)]
    {
        match entry()?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(format!("не удалось удалить сид-фразу: {error}")),
        }
    }

    #[cfg(not(desktop))]
    {
        let path = seed_path()?;
        if path.exists() {
            std::fs::remove_file(path).map_err(|error| error.to_string())?;
        }
        Ok(())
    }
}

pub fn create(words: usize) -> Result<String, String> {
    let phrase = identity::generate_phrase(words).map_err(|error| error.to_string())?;
    save(&phrase)
}

pub fn signing_key(phrase: &str) -> Result<SigningKey, String> {
    identity::keypair_from_phrase(phrase).map_err(|error| error.to_string())
}

pub fn public(phrase: &str) -> Result<String, String> {
    Ok(to_base64(&signing_key(phrase)?.verifying_key().to_bytes()))
}
