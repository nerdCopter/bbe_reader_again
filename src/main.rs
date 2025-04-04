use clap::Parser;
use std::fs::File;
use std::io::{self, BufReader, BufRead, Read};

#[derive(Parser, Debug)]
#[clap(author = "Your Name", version = "1.0", about = "BBL File Reader with Binary Decoding")]
struct Args {
    /// Input .BBL file
    #[clap(short, long)]
    input: String,
}

fn main() -> io::Result<()> {
    let args = Args::parse();

    // Open the file
    let file = File::open(&args.input)?;
    let mut reader = BufReader::new(file);

    // Read all plaintext headers dynamically
    let mut headers = Vec::new();
    loop {
        let mut header_line = Vec::new();
        let bytes_read = reader.read_until(b'\n', &mut header_line)?;

        if bytes_read == 0 {
            // End of file
            break;
        }

        // Check if the line is plaintext (ASCII)
        if header_line.iter().all(|&byte| byte.is_ascii()) {
            headers.push(String::from_utf8_lossy(&header_line).trim().to_string());
        } else {
            // Stop reading headers when binary data is encountered
            break;
        }
    }

    // Print all headers
    println!("Headers:");
    for (index, header) in headers.iter().enumerate() {
        println!("Header {}: {}", index + 1, header);
    }

    // Process the binary data after the headers
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;

    // Parse field definitions from headers
    let field_definitions = parse_field_definitions(&headers);

    // Decode binary data using field definitions
    decode_binary_data(&buffer, &field_definitions);

    Ok(())
}

/// Represents a single field definition parsed from the header.
struct FieldDefinition {
    name: String,
    signed: bool,
}

/// Parses field definitions from the plaintext headers.
fn parse_field_definitions(headers: &[String]) -> Vec<FieldDefinition> {
    let mut field_names = Vec::new();
    let mut signed_flags = Vec::new();

    for header in headers {
        if header.starts_with("H Field I name:") {
            field_names = header["H Field I name:".len()..]
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();
        } else if header.starts_with("H Field I signed:") {
            signed_flags = header["H Field I signed:".len()..]
                .split(',')
                .map(|s| s.trim() == "1")
                .collect();
        }
    }

    // Combine parsed fields into a list of `FieldDefinition`
    field_names
        .into_iter()
        .enumerate()
        .map(|(i, name)| FieldDefinition {
            name,
            signed: *signed_flags.get(i).unwrap_or(&false),
        })
        .collect()
}

/// Decodes binary data using the given field definitions.
fn decode_binary_data(data: &[u8], fields: &[FieldDefinition]) {
    println!("\nDecoded Data:");

    let mut cursor = 0;

    while cursor < data.len() {
        println!("Record:");

        for field in fields {
            if cursor >= data.len() {
                break;
            }

            // Decode based on signed flag
            let value = decode_fixed(data, &mut cursor, field.signed);
            println!("  {}: {}", field.name, value);
        }

        println!(); // Separate records with a blank line
    }
}

/// Decodes a fixed-width integer (e.g., 8-bit or 16-bit).
fn decode_fixed(data: &[u8], cursor: &mut usize, signed: bool) -> i32 {
    if *cursor >= data.len() {
        return 0;
    }

    let value = data[*cursor];

    *cursor += 1;

    if signed {
        value as i8 as i32 // Sign-extend to i32
    } else {
        value as i32
    }
}
