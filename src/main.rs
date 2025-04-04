use clap::Parser;
use std::fs::File;
use std::io::{self, BufReader, BufRead, Read};
use csv::Writer;

/// Command-line arguments
#[derive(Parser, Debug)]
#[clap(author = "Your Name", version = "0.1.0", about = "BBL File Reader with CSV Output")]
struct Args {
    /// Input .BBL file
    #[clap(short, long)]
    input: String,
}

fn main() -> io::Result<()> {
    let args = Args::parse();

    // Determine output CSV file name
    let input_path = std::path::Path::new(&args.input);
    let file_stem = input_path.file_stem().unwrap().to_str().unwrap();
    let csv_file_name = format!("{}.csv", file_stem);

    // Create CSV writer
    let mut writer = Writer::from_path(csv_file_name)?;

    // Open the BBL file
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

    // Print all headers to console
    println!("Headers:");
    for (index, header) in headers.iter().enumerate() {
        println!("Header {}: {}", index + 1, header);
    }

    // Parse field definitions from headers
    let field_definitions = parse_field_definitions(&headers);

    // Print internal column definitions to console
    println!("\nInternal Column Definitions:");
    for (index, field) in field_definitions.iter().enumerate() {
        println!(
            "Column {}: Name=\"{}\", Signed={}",
            index + 1,
            field.name,
            field.signed
        );
    }

    // Write CSV header (first 13 columns)
    writer.write_record(
        &field_definitions.iter().take(13).map(|f| f.name.clone()).collect::<Vec<_>>(),
    )?;

    // Process the binary data after the headers
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;

    // Decode binary data and write to CSV
    decode_binary_data(&buffer, &field_definitions, &mut writer)?;

    writer.flush()?; // Ensure all data is written to the file

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

/// Decodes binary data and writes selected fields to CSV.
fn decode_binary_data(
    data: &[u8],
    fields: &[FieldDefinition],
    writer: &mut Writer<File>,
) -> io::Result<()> {
    let mut cursor = 0;

    while cursor < data.len() {
        let mut record = vec![];

        for field in fields.iter().take(13) { // Only process first 13 columns
            if cursor >= data.len() {
                break;
            }

            if field.signed {
                record.push(decode_i8(data, &mut cursor).to_string());
            } else {
                record.push(decode_u32(data, &mut cursor).to_string());
            }
        }

        writer.write_record(record)?;
   }

   Ok(())
}

/// Decodes a 32-bit unsigned integer.
fn decode_u32(data: &[u8], cursor: &mut usize) -> u32 {
   if *cursor + 4 > data.len() {
       return 0;
   }

   let value = u32::from_le_bytes([
       data[*cursor],
       data[*cursor + 1],
       data[*cursor + 2],
       data[*cursor + 3],
   ]);

   *cursor += 4;
   value
}

/// Decodes an 8-bit signed integer.
fn decode_i8(data: &[u8], cursor: &mut usize) -> i8 {
     if *cursor >= data.len() {
         return 0;
     }

     let value = data[*cursor] as i8;
     *cursor += 1;
     value
}
