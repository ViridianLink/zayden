use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::{env, fmt};

use aws_lc_rs::aead::{AES_256_GCM, Aad, Nonce, RandomizedNonceKey};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use secrecy::{ExposeSecret, SecretString};

use super::CustomBotError;

const KEYS_VAR: &str = "CUSTOM_BOT_KEYS";
const ACTIVE_KEY_VAR: &str = "CUSTOM_BOT_ACTIVE_KEY";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SealedToken {
    pub key_id: i16,
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
}

pub struct Keyring {
    active: i16,
    keys: HashMap<i16, RandomizedNonceKey>,
}

impl Debug for Keyring {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut ids = self.keys.keys().copied().collect::<Vec<_>>();
        ids.sort_unstable();
        f.debug_struct("Keyring")
            .field("active", &self.active)
            .field("key_ids", &ids)
            .finish()
    }
}

impl Keyring {
    pub fn from_env() -> Result<Self, CustomBotError> {
        let keys = env_var(KEYS_VAR)?;
        let active = env_var(ACTIVE_KEY_VAR)?;
        Self::parse(&keys, &active)
    }

    pub fn parse(keys: &str, active: &str) -> Result<Self, CustomBotError> {
        let mut parsed = HashMap::new();

        for entry in keys.split(',').map(str::trim).filter(|e| !e.is_empty()) {
            let (id, encoded) = entry.split_once(':').ok_or_else(|| {
                CustomBotError::InvalidKeySpec("expected `id:base64`".to_owned())
            })?;
            let id = parse_key_id(id)?;
            let bytes = STANDARD
                .decode(encoded.trim())
                .map_err(|source| CustomBotError::KeyNotBase64 { id, source })?;
            let key = RandomizedNonceKey::new(&AES_256_GCM, &bytes)
                .map_err(|source| CustomBotError::KeyLength { id, source })?;

            if parsed.insert(id, key).is_some() {
                return Err(CustomBotError::InvalidKeySpec(format!(
                    "key {id} is listed twice"
                )));
            }
        }

        let active = parse_key_id(active)?;
        if !parsed.contains_key(&active) {
            return Err(CustomBotError::UnknownKey(active));
        }

        Ok(Self { active, keys: parsed })
    }

    #[must_use]
    pub const fn active_key_id(&self) -> i16 {
        self.active
    }

    pub fn seal(
        &self,
        application_id: i64,
        token: &SecretString,
    ) -> Result<SealedToken, CustomBotError> {
        let key = self.key(self.active)?;
        let mut ciphertext = token.expose_secret().as_bytes().to_vec();

        let nonce = key
            .seal_in_place_append_tag(
                Aad::from(application_id.to_be_bytes()),
                &mut ciphertext,
            )
            .map_err(CustomBotError::Encrypt)?;

        Ok(SealedToken {
            key_id: self.active,
            nonce: nonce.as_ref().to_vec(),
            ciphertext,
        })
    }

    pub fn open(
        &self,
        application_id: i64,
        sealed: &SealedToken,
    ) -> Result<SecretString, CustomBotError> {
        let key = self.key(sealed.key_id)?;
        let nonce = Nonce::try_assume_unique_for_key(&sealed.nonce)
            .map_err(CustomBotError::MalformedNonce)?;
        let mut in_out = sealed.ciphertext.clone();

        let plaintext = key
            .open_in_place(
                nonce,
                Aad::from(application_id.to_be_bytes()),
                &mut in_out,
            )
            .map_err(CustomBotError::Decrypt)?;

        Ok(SecretString::from(String::from_utf8(plaintext.to_vec())?))
    }

    fn key(&self, id: i16) -> Result<&RandomizedNonceKey, CustomBotError> {
        self.keys.get(&id).ok_or(CustomBotError::UnknownKey(id))
    }
}

fn parse_key_id(id: &str) -> Result<i16, CustomBotError> {
    let id = id.trim();
    id.parse().map_err(|source| CustomBotError::KeyId { id: id.to_owned(), source })
}

fn env_var(name: &'static str) -> Result<String, CustomBotError> {
    env::var(name).map_err(|source| CustomBotError::MissingEnvVar { name, source })
}
