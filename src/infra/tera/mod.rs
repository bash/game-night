use anyhow::Result;
use rocket::figment::Figment;
use rocket_dyn_templates::tera::Tera;
use rocket_dyn_templates::Engines;

mod assets;
mod functions;

pub(crate) fn configure_template_engines(
    ctx: TeraConfigurationContext,
) -> impl Fn(&mut Engines) -> Result<(), Box<dyn std::error::Error>> {
    move |engines| Ok(configure_tera(&mut engines.tera, &ctx)?)
}

pub(crate) fn configure_tera(tera: &mut Tera, ctx: &TeraConfigurationContext) -> Result<()> {
    functions::register_custom_functions(tera);
    assets::register_asset_map_functions(tera)?;
    Ok(())
}

#[derive(Debug)]
pub(crate) struct TeraConfigurationContext {}

impl TeraConfigurationContext {
    pub(crate) fn from_figment(_figment: &Figment) -> Result<Self> {
        Ok(Self {})
    }
}
