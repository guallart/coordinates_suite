use chrono::{Months, Utc};
use std::fs::{self, File};
use std::io::{BufWriter, Write};

fn main() -> Result<(), std::io::Error> {
    let now = Utc::now().date_naive();
    let expiration_date = now + Months::new(3);
    let expiration_str = expiration_date.format("%Y-%m-%d").to_string();

    let binding = fs::read_to_string("./src/expiration.rs")?;

    let content = binding.split("\n").map(|line| {
        if line.contains("const EXPIRATION_DATE: &str =") {
            format!("const EXPIRATION_DATE: &str = \"{}\";", expiration_str)
        } else {
            line.to_string()
        }
    });

    let file = File::create("./src/expiration.rs")?;
    let mut writer = BufWriter::new(file);
    for line in content.into_iter() {
        writeln!(writer, "{}", line)?;
    }

    Ok(())
}
