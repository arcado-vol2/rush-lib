# rush-lib

A Rust library for parsing and rendering **rush**, a minimalist markup language for documents and help pages written in Russian.

rush is built so you can write the whole document on a Russian keyboard layout without switching layouts: tags are made of punctuation and Cyrillic lettersю

## Features

- Parses `.rush` files into a document model. Parsing never fails: invalid markup becomes plain text, and unresolved references are reported as warnings.
- Supports multi-file projects with a table of contents, directories, internal links between documents and images.
- Output formats:
  - **HTML**: one page per `.rush` file, or a single paginated document for the whole project.
  - **Markdown**: one file per `.rush` file, or a single file for the whole project.
- Styles: `blog`, `modern` and `gost7-32` (formatting based on the Russian GOST 7.32 standard for research reports).
- Light, dark and auto themes.
- MathJax formulas are rendered as MathML.
- Output is returned as in-memory files, so you decide where and how to write them.

## Command-line tool

[**rush-cli**](https://github.com/arcado-vol2/rush-cli) is the reference application built on this library. It creates projects, manages the table of contents in `project.yaml` and builds documents. If you just want to write rush documents, use rush-cli.

## Installation

```toml
[dependencies]
rush-lib = { git = "https://github.com/arcado-vol2/rush-lib" }
```

## Quick start

```rust
use rush_lib::{build, Config, DocOut, DocStyle, Project};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut project = Project::new();

    let text = r"\+ Intro

The text with \:bold: и \;italic; words.

\. first item
\. second item";
    project.add_file("intro", "intro.rush", text.into()).expect("unique file name");
    project.add_content("Intro".into(), "intro".into());

    let config = Config { style: DocStyle::Modern, out: DocOut::Html, ..Config::default() };
    let (files, warnings) = build(&config, &project)?;

    for warning in &warnings {
        eprintln!("warning: {warning:?}");
    }

    let out = std::path::Path::new("out");
    for file in files {
        let dst = out.join(&file.path);
        std::fs::create_dir_all(dst.parent().unwrap())?;
        std::fs::write(dst, file.data)?;
    }
    Ok(())
}
```

This writes `out/intro.html` together with `style.css` and the fonts it uses.

## Usage

Building a document has three steps: describe a `Project`, choose a `Config`, and call `rush_lib::build`.

### Project

A `Project` holds the sources and the table of contents.

```rust
use std::path::Path;
use rush_lib::Project;

let (mut project, warnings) = Project::index_dir_common(Path::new("src"), Some(Path::new("src/out")))?;
```

It recursively collects `.rush` files and images (`png`, `jpg`, `jpeg`, `webp`) under the root. The second argument excludes a directory, such as an output folder nested inside the sources.

**Adding sources manually** is useful when the sources do not come from disk:

```rust
project.add_file("guide/setup", "guide/setup.rush", text)?;   // key, path, contents
project.add_image("img/scheme", "img/scheme.png", Some((800, 600)))?; // key, path, size in pixels
```

Both calls return `Err(ProjectWarning::FileNameCollision)` if the key is already taken.

**Table of contents.** It sets the document order and titles, and builds the navigation. Entries are added in order. Directories become sections:

```rust
project.add_dir("Руководство".into(), "guide".into());        // section title, directory key
project.add_content("Установка".into(), "guide/setup".into()); // page title, file key
```

`project.images()` lists the local paths of all images the project uses. The library does not copy images, so copy them to the output directory yourself.

### Config

| Field    | Type       | Values                                       | Default |
|----------|------------|----------------------------------------------|---------|
| `style`  | `DocStyle` | `Blog`, `Modern`, `Gost7_32`                 | `Blog`  |
| `out`    | `DocOut`   | `Html`, `Markdown`                           | `Html`  |
| `single` | `bool`     | one output file for the whole project if `true`, otherwise one file per `.rush` file | `false` |
| `no_js`  | `bool`     | build HTML without JavaScript                | `false` |
| `theme`  | `Theme`    | `Auto`, `Light`, `Dark`                      | `Auto`  |

`DocOut` also has `Pdf`, `Odt` and `Docx` variants. They are reserved for future backends and are not implemented yet.

All three config enums implement `ConfigConst`. It converts them to and from their string names (`"gost7-32"`, `"markdown"`, `"dark"`, ...), which is convenient for config files and command-line arguments:


### Output and warnings

`build` returns `Result<(Vec<OutFile>, Vec<Warning>), std::fmt::Error>`.

- `OutFile { path, data }`: a generated file. `path` is relative to the output directory and uses `/`. Besides documents, the output may include stylesheets and fonts.
- `Warning`: problems that did not stop the build.
  - `Warning::Project(ProjectWarning)`: problems with the project layout.
  - `Warning::Render(RenderWarning)`: problems in the documents. 

## Syntax

A short taste of rush:

```
\+ Report

Results are shown on image \*i(node schema)*\1.

\(img/scheme.png, Node schema, 80)

\№ Values
/ Argument / Value /
\№- L / R
/ Speed / 5 km_/h /
\№

\**1 The data was colected in New Zealand
```

The full language reference is in [SYNTAX.md](SYNTAX.md) (English) and [SYNTAX_RU.md](SYNTAX_RU.md) (Russian).
