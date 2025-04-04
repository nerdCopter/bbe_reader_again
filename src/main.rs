use clap::Parser;
use std::fs::File;
use std::io::{self, BufReader, BufRead, Read};
use csv::Writer;

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

    // Write CSV header
    writer.write_record(&[
        "loopIteration",
        "time",
        "P[0]",
        "P[1]",
        "P[2]",
        "I[0]",
        "I[1]",
        "I[2]",
        "D[0]",
        "D[1]",
        "FF[0]",
        "FF[1]",
        "FF[2]",
    ])?;

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

/// Decodes binary data and writes loopIteration, time, P, I, D, FF for each axis to CSV.
fn decode_binary_data(data: &[u8], fields: &[FieldDefinition], writer: &mut Writer<File>) -> io::Result<()> {
    let mut cursor = 0;

    while cursor < data.len() {
        // Find indices for loopIteration, time, and each axis' P, I, D, FF values
        let loop_iteration_index = fields.iter().position(|f| f.name == "loopIteration");
        let time_index = fields.iter().position(|f| f.name == "time");

        // Axis-specific fields
        let p_indices: Vec<_> = (0..3)
            .map(|i| fields.iter().position(|f| f.name == format!("axisP[{}]", i)))
            .collect();

        let i_indices: Vec<_> = (0..3)
            .map(|i| fields.iter().position(|f| f.name == format!("axisI[{}]", i)))
            .collect();

        let d_indices: Vec<_> = (0..2)
            .map(|i| fields.iter().position(|f| f.name == format!("axisD[{}]", i)))
            .collect();

        let ff_indices: Vec<_> = (0..3)
            .map(|i| fields.iter().position(|f| f.name == format!("axisF[{}]", i)))
            .collect();

        // Decode values for each field based on their index
        let loop_iteration_value =
            decode_field(data, &mut cursor, loop_iteration_index.map_or(0, |index| index), fields);

        let time_value =
            decode_field(data, &mut cursor, time_index.map_or(0, |index| index), fields);

        let p_values: Vec<_> = p_indices
            .iter()
            .map(|&index| decode_field(data, &mut cursor, index.unwrap_or(0), fields))
            .collect();

        let i_values: Vec<_> = i_indices
            .iter()
            .map(|&index| decode_field(data, &mut cursor, index.unwrap_or(0), fields))
            .collect();

        let d_values: Vec<_> = d_indices
            .iter()
            .map(|&index| decode_field(data, &mut cursor, index.unwrap_or(0), fields))
            .collect();

        let ff_values: Vec<_> = ff_indices
            .iter()
            .map(|&index| decode_field(data, &mut cursor, index.unwrap_or(0), fields))
            .collect();

        // Write values to CSV
        writer.write_record(&[
            loop_iteration_value.to_string(),
            time_value.to_string(),
            p_values[0].to_string(),
            p_values[1].to_string(),
            p_values[2].to_string(),
            i_values[0].to_string(),
            i_values[1].to_string(),
            i_values[2].to_string(),
            d_values.get(0).unwrap_or(&0).to_string(),
            d_values.get(1).unwrap_or(&0).to_string(),
            ff_values[0].to_string(),
            ff_values[1].to_string(),
            ff_values[2].to_string(),
        ])?;

        cursor += 1; // Move cursor to next record (adjust based on actual record size)
    }

    Ok(())
}

/// Decodes a single field based on its index and signedness.
fn decode_field(data: &[u8], cursor: &mut usize, index: usize, fields: &[FieldDefinition]) -> i32 {
    if *cursor >= data.len() || index >= fields.len() {
        return 0;
    }

    decode_fixed(data, cursor, fields[index].signed)
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
