use rustlibrary_ulid::{Generator, Ulid};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let id = Generator::new().generate()?;
    let parsed: Ulid = id.to_string().parse()?;
    assert_eq!(id, parsed);
    println!("{id}");
    Ok(())
}
