use crate::login::RocketSecretKey;
use anyhow::Result;
use rand::rng;
use rocket::figment::providers::{Env, Format as _, Toml};
use rocket::figment::{Figment, Profile};
use rocket::Config;

pub(crate) fn figment() -> Result<Figment> {
    let figment = default_figment();
    let secret_keys_path: String = figment.extract_inner("secret_keys_path")?;
    let key = RocketSecretKey::read_or_generate(secret_keys_path, &mut rng()).unwrap();
    Ok(figment.merge((rocket::Config::SECRET_KEY, &key.0)))
}

/// Adapted from [`Config::figment`] but with the
/// ability to have two rocket config files.
fn default_figment() -> Figment {
    Figment::from(Config::default())
        .merge(Toml::file(Env::var_or("ROCKET_DEFAULT_CONFIG", "Rocket.default.toml")).nested())
        .merge(Toml::file(Env::var_or("ROCKET_CONFIG", "Rocket.toml")).nested())
        .merge(Env::prefixed("ROCKET_").ignore(&["PROFILE"]).global())
        .select(Profile::from_env_or(
            "ROCKET_PROFILE",
            Config::DEFAULT_PROFILE,
        ))
}
