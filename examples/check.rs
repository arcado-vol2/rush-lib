use std::{env, path::PathBuf, process::ExitCode};

fn main() -> ExitCode {
    let Some(path) = env::args().nth(1) else {
        eprintln!("usage: check <project_path>");
        return ExitCode::from(2);
    };

    match simple_ru_help::build(PathBuf::from(path)) {
        Ok(warnings) => {
            for w in &warnings {
                println!("{w:?}");
            }
            println!("{} warinings", warnings.len());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Build error: {e}");
            ExitCode::FAILURE
        }
    }
}
