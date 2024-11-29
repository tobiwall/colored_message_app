use anyhow::{Error, Ok};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Algorithm, Argon2, Params, Version,
};

pub struct Config {
    pub memory_cost: u32,
    pub time_cost: u32,
    pub lanes: u32,
}

impl Config {
    pub fn to_argon2(&self) -> Argon2 {
        let parameter = Params::new(self.memory_cost, self.time_cost, self.lanes, None)
            .expect("Invalid Argon parameter");
        Argon2::new(Algorithm::Argon2id, Version::V0x13, parameter)
    }
}

pub fn hash_password(password: &str) -> Result<String, Error> {
    let config = Config {
        memory_cost: 4096,
        time_cost: 3,
        lanes: 1,
    };

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = config.to_argon2();

    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .unwrap()
        .to_string();
    Ok(password_hash)
}

pub fn check_password(password: &str, hash: &str) -> Result<(), argon2::password_hash::Error> {
    let parsed_hash = PasswordHash::new(hash)?;
    let verify = Argon2::default().verify_password(password.as_bytes(), &parsed_hash);
    verify
}
