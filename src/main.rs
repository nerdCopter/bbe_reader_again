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
            "Column {}: Name=\"{}\", Signed={}, Predictor={}, Encoding={}",
            index + 1,
            field.name,
            field.signed,
            field.predictor,
            field.encoding,
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
    predictor: u8,
    encoding: u8,
}

/// Parses field definitions from the plaintext headers.
fn parse_field_definitions(headers: &[String]) -> Vec<FieldDefinition> {
    let mut field_names = Vec::new();
    let mut signed_flags = Vec::new();
    let mut predictor_types = Vec::new();
    let mut encoding_types = Vec::new();

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
        } else if header.starts_with("H Field I predictor:") {
            predictor_types = header["H Field I predictor:".len()..]
                .split(',')
                .map(|s| s.trim().parse::<u8>().unwrap_or(0))
                .collect();
        } else if header.starts_with("H Field I encoding:") {
            encoding_types = header["H Field I encoding:".len()..]
                .split(',')
                .map(|s| s.trim().parse::<u8>().unwrap_or(0))
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
            predictor: *predictor_types.get(i).unwrap_or(&0),
            encoding: *encoding_types.get(i).unwrap_or(&0),
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
        let mut record = Vec::new();

        for field in fields.iter().take(13) {
            if cursor >= data.len() {
                break;
            }

            let value = match field.encoding {
                0 => read_signed_vlq(data, &mut cursor),
                1 => read_unsigned_vlq(data, &mut cursor) as i32,
                _ => 0,
            };
            record.push(value.to_string());
        }

        if record.len() == 13 {
            if let Err(e) = writer.write_record(&record) {
                eprintln!("CSV write error: {:?}", e);
                break;
            }
        } else {
            eprintln!(
                "Warning: Mismatched record length: expected 13, got {}. Cursor position: {}",
                record.len(),
                cursor
            );
            break;
        }
    }

    Ok(())
}

/// Reads a signed variable-length quantity (VLQ) from the data buffer.
fn read_signed_vlq(data: &[u8], cursor: &mut usize) -> i32 {
    let value = read_unsigned_vlq(data, cursor);
    let sign = (value & 1) as i32;
    let magnitude = (value >> 1) as i32;

    if sign != 0 {
        -magnitude
    } else {
        magnitude
    }
}

/// Reads an unsigned variable-length quantity (VLQ) from the data buffer.
fn read_unsigned_vlq(data: &[u8], cursor: &mut usize) -> u32 {
    let mut value: u32 = 0;
    let mut shift: u32 = 0;

    loop {
        if *cursor >= data.len() {
            break;
        }

        let byte = data[*cursor] as u32;
        *cursor += 1;

        value |= (byte & 0x7F) << shift;
        shift += 7;

        if (byte & 0x80) == 0 {
            break;
        }
    }

    value
}
