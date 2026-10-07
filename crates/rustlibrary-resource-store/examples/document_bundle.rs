use rustlibrary_resource_store::{ListOptions, ResourceStore, WriteMode};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut resources = ResourceStore::default();
    resources.create_directory("drafts")?;
    resources.write_many(
        [
            ("data.xml", b"<document>Hello</document>".as_slice()),
            ("styles/main.xsl", b"<stylesheet/>".as_slice()),
            ("styles/common.xsl", b"<templates/>".as_slice()),
        ],
        WriteMode::Create,
    )?;

    let input = resources.entry("data.xml")?;
    let capture = resources.snapshot();
    let sibling = resources.resolve("styles/main.xsl", "common.xsl")?;
    assert_eq!(resources.read_text(&sibling)?, "<templates/>");

    resources.write_text(
        "data.xml",
        "<document>Updated</document>",
        WriteMode::CompareVersion(input.version),
    )?;
    resources.move_entry("data.xml", "output/document.xml")?;
    assert_eq!(resources.entry_by_id(input.id)?.path, "output/document.xml");
    assert_eq!(capture.read_text("data.xml")?, "<document>Hello</document>");

    for entry in resources.list(
        "",
        ListOptions {
            recursive: true,
            include_hidden: true,
        },
    )? {
        println!("{:?}: {} ({} bytes)", entry.kind, entry.path, entry.size);
    }
    println!(
        "{} resources, {} bytes",
        resources.len(),
        resources.total_bytes()
    );
    Ok(())
}
