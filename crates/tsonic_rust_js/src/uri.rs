use crate::errors::{uri_error, JsResult};
use crate::JsString;

pub fn encode_uri_component(value: &JsString) -> JsResult<JsString> {
    percent_encode(value, ComponentMode::Component)
}

pub fn encode_uri(value: &JsString) -> JsResult<JsString> {
    percent_encode(value, ComponentMode::Uri)
}

pub fn decode_uri_component(value: &JsString) -> JsResult<JsString> {
    percent_decode(value, ComponentMode::Component)
}

pub fn decode_uri(value: &JsString) -> JsResult<JsString> {
    percent_decode(value, ComponentMode::Uri)
}

#[derive(Clone, Copy)]
enum ComponentMode {
    Uri,
    Component,
}

fn percent_encode(value: &JsString, mode: ComponentMode) -> JsResult<JsString> {
    let text = value
        .to_utf8()
        .map_err(|_| uri_error("URI cannot encode an unpaired UTF-16 surrogate"))?;
    let mut output = Vec::with_capacity(text.len());
    for byte in text.bytes() {
        let character = byte as char;
        let unescaped = character.is_ascii_alphanumeric()
            || matches!(
                character,
                '-' | '_' | '.' | '!' | '~' | '*' | '\'' | '(' | ')'
            )
            || matches!(
                (mode, character),
                (
                    ComponentMode::Uri,
                    ';' | ',' | '/' | '?' | ':' | '@' | '&' | '=' | '+' | '$' | '#'
                )
            );
        if unescaped {
            output.push(u16::from(byte));
        } else {
            output.push(u16::from(b'%'));
            output.push(u16::from(hex(byte >> 4)));
            output.push(u16::from(hex(byte & 0x0f)));
        }
    }
    Ok(JsString::from_units(output))
}

fn percent_decode(value: &JsString, mode: ComponentMode) -> JsResult<JsString> {
    let units = value.units();
    let mut output = Vec::with_capacity(units.len());
    let mut index = 0;
    while index < units.len() {
        if units[index] != u16::from(b'%') {
            output.push(units[index]);
            index += 1;
            continue;
        }

        let first = percent_byte(units, index)?;
        let byte_count =
            utf8_sequence_length(first).ok_or_else(|| uri_error("malformed URI sequence"))?;
        let sequence_end = index
            .checked_add(byte_count.saturating_mul(3))
            .ok_or_else(|| uri_error("malformed URI sequence"))?;
        if sequence_end > units.len() {
            return Err(uri_error("malformed URI sequence"));
        }
        let mut bytes = Vec::with_capacity(byte_count);
        for offset in 0..byte_count {
            bytes.push(percent_byte(units, index + offset * 3)?);
        }
        let decoded =
            std::str::from_utf8(&bytes).map_err(|_| uri_error("malformed URI sequence"))?;
        let mut characters = decoded.chars();
        let character = characters
            .next()
            .ok_or_else(|| uri_error("malformed URI sequence"))?;
        if characters.next().is_some() {
            return Err(uri_error("malformed URI sequence"));
        }

        if matches!(mode, ComponentMode::Uri) && is_uri_reserved(character) {
            output.extend_from_slice(&units[index..sequence_end]);
        } else {
            let mut encoded = [0_u16; 2];
            output.extend_from_slice(character.encode_utf16(&mut encoded));
        }
        index = sequence_end;
    }
    Ok(JsString::from_units(output))
}

fn percent_byte(units: &[u16], index: usize) -> JsResult<u8> {
    if units.get(index) != Some(&u16::from(b'%')) {
        return Err(uri_error("malformed URI sequence"));
    }
    let high = units
        .get(index + 1)
        .copied()
        .and_then(hex_value)
        .ok_or_else(|| uri_error("malformed URI sequence"))?;
    let low = units
        .get(index + 2)
        .copied()
        .and_then(hex_value)
        .ok_or_else(|| uri_error("malformed URI sequence"))?;
    Ok((high << 4) | low)
}

fn utf8_sequence_length(first: u8) -> Option<usize> {
    match first {
        0x00..=0x7f => Some(1),
        0xc2..=0xdf => Some(2),
        0xe0..=0xef => Some(3),
        0xf0..=0xf4 => Some(4),
        _ => None,
    }
}

fn is_uri_reserved(value: char) -> bool {
    matches!(
        value,
        ';' | '/' | '?' | ':' | '@' | '&' | '=' | '+' | '$' | ',' | '#'
    )
}

fn hex(value: u8) -> u8 {
    b"0123456789ABCDEF"[value as usize]
}

fn hex_value(value: u16) -> Option<u8> {
    match value {
        value if (u16::from(b'0')..=u16::from(b'9')).contains(&value) => {
            Some((value - u16::from(b'0')) as u8)
        }
        value if (u16::from(b'a')..=u16::from(b'f')).contains(&value) => {
            Some((value - u16::from(b'a') + 10) as u8)
        }
        value if (u16::from(b'A')..=u16::from(b'F')).contains(&value) => {
            Some((value - u16::from(b'A') + 10) as u8)
        }
        _ => None,
    }
}
