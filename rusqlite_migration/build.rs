use std::{
    env,
    error::Error,
    fs::{read_to_string, File},
    io::{self, BufWriter, Write},
};

fn main() -> Result<(), Box<dyn Error>> {
    let readme_path = env::var("CARGO_PKG_README")?;
    println!("cargo:rerun-if-changed={readme_path}");

    let out_dir = env::var("OUT_DIR")?;
    let readme_for_rustdoc = File::create(format!("{out_dir}/readme_for_rustdoc.md"))?;
    let mut out = BufWriter::new(readme_for_rustdoc);

    let readme = read_to_string(readme_path)?;
    readme
        .lines()
        .skip_while(|line| line != &"<!-- rustdoc start -->")
        .skip(1) // Discard the pattern line
        .try_fold(0, |lines_written, line| -> Result<usize, io::Error> {
            writeln!(out, "{}", line)?;
            Ok(lines_written + 1)
        })
        .map(|lines_written| {
            println!("Wrote {lines_written} lines from the README to the rustdoc.");
            assert!(
                lines_written > 70,
                "the size of the documentation produced from the README.md file is suspiciously small"
            )
        })
        ?;

    Ok(())
}
