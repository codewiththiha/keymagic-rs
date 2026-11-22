//! Compile corpus scripts with the pinned upstream writer.

#![forbid(unsafe_code)]

use std::{env, error::Error, fs, path::Path};

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = env::args_os().skip(1);
    let directory = arguments.next().ok_or("missing output directory")?;
    fs::create_dir_all(&directory)?;
    let mut count = 0;
    for input in arguments {
        let output = Path::new(&directory).join(format!("{count:04}.km2"));
        kms2km2::convert_kms_to_km2(Path::new(&input), &output)?;
        count += 1;
    }
    if count == 0 {
        return Err("no source scripts".into());
    }
    println!("Compiled {count} reference binaries.");
    Ok(())
}
