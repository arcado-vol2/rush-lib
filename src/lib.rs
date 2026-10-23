use std::{fs, io, path::PathBuf};

mod parse;
mod render;
mod config;
mod resources;
mod content;

pub use config::ConfigWarning;
pub use render::RenderWarning;
pub use resources::ResourceWarning;
pub use parse::common::RefKind;

use crate::{config::{Config, DocOut, DocStyle}, content::Content, parse::rush::Rush, render::common::Backend, resources::Resources};
use crate::render::html::{HtmlBackend, HtmlStyle, gost, gostlike::GostLike};
use crate::parse::block_fragmentation::Counters;

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
    println!("{resources:?}");
    let (content, content_warns) = Content::resolve(&config.content, &resources);
    warns.extend(content_warns.into_iter().map(Warning::from));
    match config.out {
        DocOut::Html if config.style == DocStyle::Gost => {
            let render_warns = build_gost(&config, &resources, &content)?;
            warns.extend(render_warns.into_iter().map(Warning::from));
        }
        DocOut::Html => {
            let style: &dyn HtmlStyle = match config.style {
                DocStyle::GostLike => &GostLike,
                _ => {
                    unimplemented!("not yet")
                }
            };
            let backend = HtmlBackend { style };
            fs::create_dir_all(&config.output)?;
            fs::write(config.output.join(style.css_name()), style.css())?;
            for (id, src, local) in resources.files.iter() {
                let text = fs::read_to_string(src)?;
                let rush = Rush::new(&text, &resources);
                let (html, render_warns) = backend.render(rush, &resources, id, &content).map_err(io::Error::other)?;
                warns.extend(render_warns.into_iter().map(Warning::from));
                fs::write(config.output.join(local).with_extension("html"), html)?;
            }
        }
        _ => {
            println!("not html!");
            unimplemented!("not yet")
        }
    }
    Ok(warns)
}

fn build_gost(config: &Config, resources: &Resources, content: &Content) -> io::Result<Vec<RenderWarning>> {
    fs::create_dir_all(&config.output)?;
    fs::write(config.output.join(gost::CSS_NAME), gost::CSS)?;
    let fonts = config.output.join(gost::FONTS_DIR);
    fs::create_dir_all(&fonts)?;
    for (name, data) in gost::metrics::FONT_FILES {
        fs::write(fonts.join(name), data)?;
    }

    let texts: Vec<String> = content.items.iter()
        .map(|item| fs::read_to_string(resources.files.global_path(item.file)))
        .collect::<io::Result<_>>()?;
    let mut counters = Counters::default();
    let mut docs = Vec::with_capacity(texts.len());
    for (item, text) in content.items.iter().zip(&texts) {
        let rush = Rush::new_from(text, resources, counters);
        counters = rush.counters;
        docs.push(gost::Doc { file: item.file, name: &item.name, rush });
    }
    let (html, warns) = gost::render_project(docs, resources);
    fs::write(config.output.join(gost::OUT_NAME), html)?;
    Ok(warns)
}
