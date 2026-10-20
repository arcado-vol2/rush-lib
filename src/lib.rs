use std::{fs, io, path::PathBuf};

mod parse;
mod render;
mod config;
mod resources;

pub use config::ConfigWarning;
pub use render::RenderWarning;
pub use resources::ResourceWarning;

use crate::{config::{Config, DocOut, DocStyle}, parse::rush::Rush, render::common::Backend, resources::Resources};
use crate::render::html::{HtmlBackend, GOSTLIKE_CSS, CSS_NAME};

#[derive(Debug, Clone, PartialEq)]
pub enum Warning {
    Project(ConfigWarning),
    Resource(ResourceWarning),
    Render(RenderWarning),
}

impl From<ConfigWarning> for Warning {
    fn from(w: ConfigWarning) -> Self {
        Self::Project(w)
    }
}

impl From<ResourceWarning> for Warning {
    fn from(w: ResourceWarning) -> Self {
        Self::Resource(w)
    }
}

impl From<RenderWarning> for Warning {
    fn from(w: RenderWarning) -> Self {
        Self::Render(w)
    }
}

pub fn build(project_path: PathBuf) -> io::Result<Vec<Warning>> {
    let (config, config_warns) = Config::load(&project_path);
    let mut warns: Vec<Warning> = config_warns.into_iter().map(Warning::from).collect();

    let (resources, resource_warns) = Resources::build_index(&config.input)?;
    warns.extend(resource_warns.into_iter().map(Warning::from));
    resources.copy_tree(&config.output)?;
    match (config.style, config.out) {
        (DocStyle::GostLike, DocOut::Html) => {
            fs::create_dir_all(&config.output)?;
            fs::write(config.output.join(CSS_NAME), GOSTLIKE_CSS)?;
            for (id, src, local) in resources.files.iter() {
                let text = fs::read_to_string(src)?;
                let rush = Rush::new(&text, &resources);
                let css_href = resources.root_href(id, CSS_NAME);
                let (html, render_warns) = HtmlBackend::render(rush, &resources, id, &css_href).map_err(io::Error::other)?;
                warns.extend(render_warns.into_iter().map(Warning::from));
                fs::write(config.output.join(local).with_extension("html"), html)?;
            }
        }
        _ => unimplemented!("not yet")
    }
    Ok(warns)
}
