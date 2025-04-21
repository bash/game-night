use anyhow::Result;
use rocket_dyn_templates::tera::Tera;
use rocket_dyn_templates::Engines;

mod assets;
mod functions;

pub(crate) fn configure_template_engines(
    engines: &mut Engines,
) -> Result<(), Box<dyn std::error::Error>> {
    Ok(configure_tera(&mut engines.tera)?)
}

pub(crate) fn configure_tera(tera: &mut Tera) -> Result<()> {
    functions::register_custom_functions(tera);
    assets::register_asset_map_functions(tera)?;
    Ok(())
}
