// SPDX-License-Identifier: Apache-2.0
// Copyright Clément Joly and contributors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//    http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Insert the readme as documentation of the crate

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
        .filter(|line| *line != "</div>") // Known unclosed div because we don’t start from the top
        .try_fold(0, |lines_written, line| -> Result<usize, io::Error> {
            writeln!(out, "{line}")?;
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
