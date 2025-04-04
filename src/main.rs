use clap::Parser;
use std::collections::HashMap;
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

/// Represents a single field definition parsed from the header.
#[derive(Debug, Clone)]
struct FieldDefinition {
    name: String,
    encoding: u8,
    signed: bool,
    predictor: u8,
}

fn main() -> io::Result<()> {
    let args = Args::parse();

    // Define the desired Field I names
    let desired_fields = [
        "loopIteration",
        "time",
        "axisP[0]",
        "axisP[1]",
        "axisP[2]",
        "axisI[0]",
        "axisI[1]",
        "axisI[2]",
        "axisD[0]",
        "axisD[1]",
        "axisF[0]",
        "axisF[1]",
        "axisF[2]",
    ];

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

    // Print internal column definitions
    println!("Internal Column Definitions:");
    for (i, field) in field_definitions.iter().enumerate() {
        println!(
            "Column {}: Name=\"{}\", Signed={}, Predictor={}, Encoding={}",
            i + 1,
            field.name,
            field.signed,
            field.predictor,
            field.encoding
        );
    }

    // Create a map of field names to their definitions
    let field_map: HashMap<String, FieldDefinition> = field_definitions
        .into_iter()
        .map(|f| (f.name.clone(), f))
        .collect();

    // Write CSV header (only desired Field I data)
    let mut csv_header: Vec<String> = Vec::new();

    for field_name in &desired_fields {
        if field_map.contains_key(*field_name) {
            csv_header.push(field_name.to_string());
        }
    }
    writer.write_record(&csv_header)?;

    // Process the binary data after the headers
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;

    // Decode binary data and write to CSV (only desired Field I data)
    decode_binary_data(&buffer, &field_map, &mut writer, &desired_fields, &headers)?;

    writer.flush()?; // Ensure all data is written to the file

    Ok(())
}

/// Parses field definitions from the plaintext headers.
fn parse_field_definitions(headers: &[String]) -> Vec<FieldDefinition> {
    let mut field_names = Vec::new();
    let mut encoding_types = Vec::new();
    let mut signed_flags = Vec::new();
    let mut predictor_types = Vec::new();

    for header in headers {
        if header.starts_with("H Field I name:") {
            field_names = header["H Field I name:".len()..]
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();
        } else if header.starts_with("H Field I encoding:") {
            encoding_types = header["H Field I encoding:".len()..]
                .split(',')
                .map(|s| s.trim().parse::<u8>().unwrap_or(0))
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
        }
    }

    // Combine parsed fields into a list of `FieldDefinition`
    field_names
        .into_iter()
        .enumerate()
        .map(|(i, name)| FieldDefinition {
            name,
            encoding: *encoding_types.get(i).unwrap_or(&0),
            signed: *signed_flags.get(i).unwrap_or(&false),
            predictor: *predictor_types.get(i).unwrap_or(&0),
        })
        .collect()
}

/// Decodes binary data and writes selected fields to CSV.
fn decode_binary_data(
    data: &[u8],
    field_map: &HashMap<String, FieldDefinition>,
    writer: &mut Writer<File>,
    desired_fields: &[&str],
    headers: &[String],
) -> io::Result<()> {
    let mut cursor = 0;
    let h_field_i_name_line = headers
        .iter()
        .find(|header| header.starts_with("H Field I name:"))
        .expect("H Field I name: header not found");
    let field_names: Vec<&str> = h_field_i_name_line["H Field I name:".len()..]
        .split(',')
        .map(|s| s.trim())
        .collect();
    let h_field_i_encoding_line = headers
        .iter()
        .find(|header| header.starts_with("H Field I encoding:"))
        .expect("H Field I encoding: header not found");
    let encoding_types: Vec<&str> = h_field_i_encoding_line["H Field I encoding:".len()..]
        .split(',')
        .map(|s| s.trim())
        .collect();
    let h_field_i_signed_line = headers
        .iter()
        .find(|header| header.starts_with("H Field I signed:"))
        .expect("H Field I signed: header not found");
    let signed_types: Vec<&str> = h_field_i_signed_line["H Field I signed:".len()..]
        .split(',')
        .map(|s| s.trim())
        .collect();
    let h_field_i_predictor_line = headers
        .iter()
        .find(|header| header.starts_with("H Field I predictor:"))
        .expect("H Field I predictor: header not found");
    let predictor_types: Vec<&str> = h_field_i_predictor_line["H Field I predictor:".len()..]
        .split(',')
        .map(|s| s.trim())
        .collect();

    while cursor < data.len() {
        let mut record: Vec<String> = Vec::new();
        let mut valid_record = true;

        for field_name in desired_fields {
            let field_index = field_names.iter().position(|&r| r == *field_name);

            if let Some(index) = field_index {
                let encoding: u8 = encoding_types[index].parse().unwrap_or(0);
                let signed: bool = signed_types[index] == "1";
                let predictor: u8 = predictor_types[index].parse().unwrap_or(0);

                if let Some(_field) = field_map.get(*field_name) {
                    // Check if there is enough data before reading
                    let bytes_needed = match encoding {
                        0 | 1 => 1, // VLQ encoding needs at least 1 byte
                        _ => 1,     // other encodings also need at least 1 byte
                    };

                    if cursor + bytes_needed > data.len() {
                        valid_record = false;
                        break;
                    }

                    let value = match encoding {
                        0 => {
                            let val = read_signed_vlq(data, &mut cursor);
                            if signed {
                                val
                            } else {
                                val.abs()
                            }
                        }
                        1 => {
                            let val = read_unsigned_vlq(data, &mut cursor) as i32;
                            if signed {
                                val
                            } else {
                                val.abs()
                            }
                        }
                        _ => {
                            valid_record = false;
                            break;
                        }
                    };
                    record.push(value.to_string());
                } else {
                    valid_record = false;
                    break;
                }
            } else {
                valid_record = false;
                break;
            }
        }

        if valid_record && record.len() == desired_fields.len() {
            if let Err(_e) = writer.write_record(&record) {
                break;
            }
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
