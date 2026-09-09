use secrecy::{ExposeSecret, SecretString};

use crate::shared::errors::AppError;

const SERVICE_NAME: &str = "com.emmm.mod-manager";
const AI_API_KEY_ACCOUNT: &str = "ai-api-key";

pub struct CredentialStore {
    backend: Backend,
}

enum Backend {
    Native,
    #[cfg(test)]
    Memory(std::sync::Mutex<Option<SecretString>>),
}

impl CredentialStore {
    pub fn new_native() -> Self {
        Self {
            backend: Backend::Native,
        }
    }

    #[cfg(test)]
    fn new_memory() -> Self {
        Self {
            backend: Backend::Memory(std::sync::Mutex::new(None)),
        }
    }

    pub fn set_ai_api_key(&self, api_key: &SecretString) -> Result<(), AppError> {
        match &self.backend {
            Backend::Native => Self::native_entry()?
                .set_password(api_key.expose_secret())
                .map_err(|error| credential_error("store", error)),
            #[cfg(test)]
            Backend::Memory(value) => {
                *value.lock().map_err(|_| lock_error())? = Some(api_key.clone());
                Ok(())
            }
        }
    }

    pub fn get_ai_api_key(&self) -> Result<Option<SecretString>, AppError> {
        match &self.backend {
            Backend::Native => match Self::native_entry()?.get_password() {
                Ok(value) => Ok(Some(SecretString::from(value))),
                Err(keyring::Error::NoEntry) => Ok(None),
                Err(error) => Err(credential_error("read", error)),
            },
            #[cfg(test)]
            Backend::Memory(value) => Ok(value.lock().map_err(|_| lock_error())?.clone()),
        }
    }

    pub fn delete_ai_api_key(&self) -> Result<(), AppError> {
        match &self.backend {
            Backend::Native => match Self::native_entry()?.delete_credential() {
                Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
                Err(error) => Err(credential_error("delete", error)),
            },
            #[cfg(test)]
            Backend::Memory(value) => {
                *value.lock().map_err(|_| lock_error())? = None;
                Ok(())
            }
        }
    }

    pub fn has_ai_api_key(&self) -> Result<bool, AppError> {
        self.get_ai_api_key().map(|value| value.is_some())
    }

    fn native_entry() -> Result<keyring::Entry, AppError> {
        keyring::Entry::new(SERVICE_NAME, AI_API_KEY_ACCOUNT)
            .map_err(|error| credential_error("access", error))
    }
}

impl Default for CredentialStore {
    fn default() -> Self {
        Self::new_native()
    }
}

fn credential_error(action: &str, error: keyring::Error) -> AppError {
    AppError::Security(format!("Could not {action} the AI API credential: {error}"))
}

#[cfg(test)]
fn lock_error() -> AppError {
    AppError::Security("AI credential store lock was poisoned".to_string())
}

#[cfg(test)]
mod tests {
    use secrecy::{ExposeSecret, SecretString};

    use super::CredentialStore;

    #[test]
    fn stores_and_deletes_ai_api_key_without_exposing_it() {
        let store = CredentialStore::new_memory();
        let secret = SecretString::from("test-secret".to_string());

        store.set_ai_api_key(&secret).unwrap();
        assert!(store.has_ai_api_key().unwrap());
        assert_eq!(
            store.get_ai_api_key().unwrap().unwrap().expose_secret(),
            "test-secret"
        );

        store.delete_ai_api_key().unwrap();
        assert!(!store.has_ai_api_key().unwrap());
    }
}
