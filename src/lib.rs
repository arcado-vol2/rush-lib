use std::fmt;

mod parse;
mod render;
mod style;
mod config;
mod resources;
mod content;
mod project;
mod color;

pub use config::{Config, DocOut, DocStyle};
pub use project::{Project, ProjectWarning};
pub use render::{Feature, Origin, RenderWarning, RenderWarningKind};
pub use parse::common::RefKind;

use crate::content::Content;

#[derive(Debug, Clone, PartialEq)]
pub enum Warning {
    Project(ProjectWarning),
    Render(RenderWarning),
}

impl From<ProjectWarning> for Warning {
    fn from(w: ProjectWarning) -> Self {
        Self::Project(w)
    }
}

impl From<RenderWarning> for Warning {
    fn from(w: RenderWarning) -> Self {
        Self::Render(w)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OutFile {
    pub path: String,
    pub data: Vec<u8>,
}

pub fn build(config: &Config, project: &Project) -> Result<(Vec<OutFile>, Vec<Warning>), fmt::Error> {
    let (content, content_warns) = Content::resolve(&project.content, &project.resources);
    let mut warns: Vec<Warning> = content_warns.into_iter().map(Warning::from).collect();
    if config.single {
        for (id, local, _) in project.resources.files.iter() {
            if !content.items.iter().any(|item| item.file == id) {
                warns.push(ProjectWarning::NotInContent(local.to_string_lossy().into_owned()).into());
            }
        }
    }
    let (files, render_warns) = render::build(config, project, &content)?;
    warns.extend(render_warns.into_iter().map(Warning::from));
    Ok((files, warns))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_warns_about_files_outside_content() {
        let mut p = Project::new();
        p.add_file("a", "a.rush", "текст".into()).unwrap();
        p.add_file("b", "b.rush", "текст".into()).unwrap();
        p.add_content("А".into(), "a".into());
        let not_in_content = vec![Warning::Project(ProjectWarning::NotInContent("b.rush".into()))];

        for out in [DocOut::Html, DocOut::Markdown] {
            let (_, warns) = build(&Config { out, single: true, ..Config::default() }, &p).unwrap();
            assert_eq!(warns, not_in_content);
            let (_, warns) = build(&Config { out, single: false, ..Config::default() }, &p).unwrap();
            assert!(warns.is_empty(), "{warns:?}");
        }
    }
}
