use crate::domain::errors::AppError;
use serde::{Deserialize, Serialize};

const UTF8_BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub enum IniEncoding {
    Utf8,
    ShiftJis,
    LossyUtf8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub enum LineTerminator {
    None,
    Lf,
    CrLf,
    Cr,
}

pub struct DecodedIni {
    pub text: String,
    pub had_bom: bool,
    pub clean: bool,
    pub encoding: IniEncoding,
}

pub fn decode_ini_source(bytes: &[u8]) -> DecodedIni {
    let had_bom = bytes.starts_with(&UTF8_BOM);
    let content = if had_bom {
        &bytes[UTF8_BOM.len()..]
    } else {
        bytes
    };

    match String::from_utf8(content.to_vec()) {
        Ok(text) => DecodedIni {
            text,
            had_bom,
            clean: true,
            encoding: IniEncoding::Utf8,
        },
        Err(_) => {
            let (decoded, _encoding, had_errors) = encoding_rs::SHIFT_JIS.decode(content);
            if !had_errors {
                return DecodedIni {
                    text: decoded.into_owned(),
                    had_bom,
                    clean: true,
                    encoding: IniEncoding::ShiftJis,
                };
            }

            DecodedIni {
                text: String::from_utf8_lossy(content).to_string(),
                had_bom,
                clean: false,
                encoding: IniEncoding::LossyUtf8,
            }
        }
    }
}

pub fn decode_ini_bytes(bytes: &[u8]) -> (String, bool, bool) {
    let decoded = decode_ini_source(bytes);
    (decoded.text, decoded.had_bom, decoded.clean)
}

pub fn split_lines_preserving_terminators(text: &str) -> (Vec<String>, Vec<LineTerminator>) {
    let bytes = text.as_bytes();
    let mut lines = Vec::new();
    let mut terminators = Vec::new();
    let mut start = 0;
    let mut index = 0;

    while index < bytes.len() {
        let terminator = match bytes[index] {
            b'\n' => Some((LineTerminator::Lf, 1)),
            b'\r' if bytes.get(index + 1) == Some(&b'\n') => Some((LineTerminator::CrLf, 2)),
            b'\r' => Some((LineTerminator::Cr, 1)),
            _ => None,
        };
        let Some((terminator, width)) = terminator else {
            index += 1;
            continue;
        };

        lines.push(text[start..index].to_string());
        terminators.push(terminator);
        index += width;
        start = index;
    }

    if start < text.len() {
        lines.push(text[start..].to_string());
        terminators.push(LineTerminator::None);
    }

    (lines, terminators)
}

pub fn render_lines(lines: &[String], terminators: &[LineTerminator]) -> Result<String, AppError> {
    if lines.len() != terminators.len() {
        return Err(AppError::Internal(
            "INI line/terminator metadata is inconsistent".to_string(),
        ));
    }

    let mut output = String::new();
    for (line, terminator) in lines.iter().zip(terminators) {
        output.push_str(line);
        output.push_str(match terminator {
            LineTerminator::None => "",
            LineTerminator::Lf => "\n",
            LineTerminator::CrLf => "\r\n",
            LineTerminator::Cr => "\r",
        });
    }
    Ok(output)
}

pub fn encode_ini_text(
    text: &str,
    encoding: IniEncoding,
    had_bom: bool,
) -> Result<Vec<u8>, AppError> {
    let mut output = match encoding {
        IniEncoding::Utf8 => text.as_bytes().to_vec(),
        IniEncoding::ShiftJis => {
            let (encoded, _encoding, had_errors) = encoding_rs::SHIFT_JIS.encode(text);
            if had_errors {
                return Err(AppError::Validation(
                    "Edited text contains characters that Shift-JIS cannot represent".to_string(),
                ));
            }
            encoded.into_owned()
        }
        IniEncoding::LossyUtf8 => {
            return Err(AppError::Validation(
                "Cannot encode an INI decoded with data loss".to_string(),
            ));
        }
    };

    if had_bom {
        let mut with_bom = UTF8_BOM.to_vec();
        with_bom.append(&mut output);
        return Ok(with_bom);
    }
    Ok(output)
}

pub fn source_fingerprint(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}
