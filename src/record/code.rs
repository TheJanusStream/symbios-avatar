//! Compact share codes.
//!
//! A share code carries a *look*, not a record: archetype plus quantised axes,
//! packed into a dozen bytes and rendered in Crockford base32. That yields
//! something short enough to read aloud or fit in a QR code, which is how every
//! shipped creator with a "share your character" feature does it.
//!
//! Codes are deliberately **lossy** — each axis is quantised to a byte. The
//! canonical avatar lives in an AT Protocol record with full precision; a code
//! is for passing a look between people, and re-encoding one is not a
//! round-trip through the record.
//!
//! Crockford's alphabet omits `I`, `L`, `O`, and `U`, and decoding folds `I`/`L`
//! to `1` and `O` to `0`, so codes survive being written down by hand.

use thiserror::Error;

use crate::plan::{Archetype, Composites, PlanDecodeError};
use crate::texture::SkinParams;

/// Format version, bumped when the byte layout changes.
///
/// **5** — the composites block (#162): four bytes for the high-level axes,
/// written after the archetype and before the complexion. Added here rather
/// than left for the epic's versioning pass, because a record field that
/// silently drops out of a share code is a look that changes when it is passed
/// between people — the one thing a code exists not to do. Version 4 stays
/// readable and decodes to the neutral composites, which is what a code written
/// before the axes existed meant.
///
/// **4** — the exploration envelope (#160): each plan byte now spans its
/// axis's widened range rather than ±1, so a code can carry the extremes the
/// record can. The byte LAYOUT is version 3's, only the spans moved, which is
/// why 3 is not a refusal: `decode` reads a version-3 payload through the
/// old spans and an old code goes on meaning the body it named when it was
/// written down.
///
/// **3** — the humanoid plan gained `headBreadth` and `faceLength` (#61), which
/// are two more bytes in the middle of its payload. A version 2 code read as a
/// version 3 one would take a head's breadth from what used to be the extremity
/// size and then run off the end, so the version gate is what keeps an old code
/// a clean refusal rather than a body nobody asked for. Codes are for passing a
/// look between people and re-encoding one was never a round trip; the record
/// is the canonical avatar and reads unchanged.
pub const SHARE_CODE_VERSION: u8 = 5;

/// The oldest format whose codes still decode.
const OLDEST_VERSION: u8 = 3;

/// The last format written before the composites block existed (#162).
///
/// Codes at or below it carry no composites and decode to the neutral ones,
/// which is exactly what they meant when they were written down.
const PRE_COMPOSITES_VERSION: u8 = 4;

/// The last format whose plan bytes span ±1 rather than the exploration
/// envelope (#160).
const NARROW_SPAN_VERSION: u8 = 3;

/// Crockford base32 digits.
const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Characters per hyphen-separated group.
const GROUP: usize = 5;

/// Why a share code could not be read.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Error)]
pub enum ShareCodeError {
    /// The code contains a character outside the alphabet.
    #[error("'{0}' is not a valid share-code character")]
    BadCharacter(char),
    /// The code is too short to hold a version, archetype, and checksum.
    #[error("share code is too short")]
    TooShort,
    /// The checksum does not match, so the code was mistyped.
    #[error("share code checksum does not match; check for a mistyped character")]
    Checksum,
    /// The code was written by a newer format than this build understands.
    #[error("share code format version {0} is newer than this build supports")]
    UnsupportedVersion(u8),
    /// The payload did not describe a body this build knows.
    #[error("share code payload is invalid: {0}")]
    Payload(#[from] PlanDecodeError),
}

/// Renders a body and complexion as a share code.
#[must_use]
pub fn encode(archetype: &Archetype, composites: &Composites, skin: &SkinParams) -> String {
    use crate::plan::{put_signed, put_unit};

    let mut payload = vec![SHARE_CODE_VERSION];
    archetype.encode(&mut payload);
    composites.encode(&mut payload);
    put_unit(&mut payload, skin.melanin);
    put_signed(&mut payload, skin.undertone);
    put_unit(&mut payload, skin.blush);
    put_unit(&mut payload, skin.freckles);
    put_unit(&mut payload, skin.stubble);
    payload.push(checksum(&payload));
    group(&base32_encode(&payload))
}

/// Parses a share code back into an archetype.
///
/// Whitespace and hyphens are ignored and letters are case-insensitive.
///
/// # Errors
///
/// Returns [`ShareCodeError`] if the code contains an unknown character, is
/// truncated, fails its checksum, or was written by a newer format.
pub fn decode(code: &str) -> Result<(Archetype, Composites, SkinParams), ShareCodeError> {
    let bytes = base32_decode(code)?;
    // Version, archetype tag, at least one axis, checksum.
    if bytes.len() < 4 {
        return Err(ShareCodeError::TooShort);
    }

    let (body, &[found]) = bytes.split_at(bytes.len() - 1) else {
        return Err(ShareCodeError::TooShort);
    };
    if checksum(body) != found {
        return Err(ShareCodeError::Checksum);
    }

    let (&version, mut payload) = body.split_first().ok_or(ShareCodeError::TooShort)?;
    if !(OLDEST_VERSION..=SHARE_CODE_VERSION).contains(&version) {
        return Err(ShareCodeError::UnsupportedVersion(version));
    }

    use crate::plan::{take_signed, take_unit};

    let archetype = if version <= NARROW_SPAN_VERSION {
        Archetype::decode_legacy(&mut payload)?
    } else {
        Archetype::decode(&mut payload)?
    };
    // A code older than the composites block described a body that had none,
    // so the neutral ones are what it meant — not a gap to be guessed at.
    let composites = if version <= PRE_COMPOSITES_VERSION {
        Composites::default()
    } else {
        Composites::decode(&mut payload)?
    };
    let mut skin = SkinParams {
        melanin: take_unit(&mut payload)?,
        undertone: take_signed(&mut payload)?,
        blush: take_unit(&mut payload)?,
        freckles: take_unit(&mut payload)?,
        stubble: take_unit(&mut payload)?,
    };
    skin.sanitize();
    Ok((archetype, composites, skin))
}

/// Position-weighted sum, enough to catch a mistyped or transposed character.
fn checksum(bytes: &[u8]) -> u8 {
    bytes.iter().enumerate().fold(0u8, |sum, (index, &byte)| {
        sum.wrapping_add(byte.wrapping_mul(index as u8 | 1))
    })
}

/// Inserts hyphens so a code can be read aloud without losing your place.
fn group(code: &str) -> String {
    let mut out = String::with_capacity(code.len() + code.len() / GROUP);
    for (index, ch) in code.chars().enumerate() {
        if index > 0 && index % GROUP == 0 {
            out.push('-');
        }
        out.push(ch);
    }
    out
}

/// Packs bytes into 5-bit Crockford digits.
fn base32_encode(data: &[u8]) -> String {
    let mut out = String::new();
    let mut buffer: u32 = 0;
    let mut bits = 0;

    for &byte in data {
        buffer = (buffer << 8) | u32::from(byte);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(char::from(ALPHABET[((buffer >> bits) & 0x1F) as usize]));
        }
    }
    if bits > 0 {
        out.push(char::from(
            ALPHABET[((buffer << (5 - bits)) & 0x1F) as usize],
        ));
    }
    out
}

/// Unpacks Crockford digits back into bytes, ignoring separators.
fn base32_decode(code: &str) -> Result<Vec<u8>, ShareCodeError> {
    let mut out = Vec::new();
    let mut buffer: u32 = 0;
    let mut bits = 0;

    for ch in code.chars() {
        if ch == '-' || ch.is_whitespace() {
            continue;
        }
        buffer = (buffer << 5) | u32::from(digit(ch)?);
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push(((buffer >> bits) & 0xFF) as u8);
        }
    }

    Ok(out)
}

/// Maps one character to its 5-bit value, folding look-alikes.
fn digit(ch: char) -> Result<u8, ShareCodeError> {
    let upper = ch.to_ascii_uppercase();
    let folded = match upper {
        'I' | 'L' => '1',
        'O' => '0',
        other => other,
    };
    ALPHABET
        .iter()
        .position(|&d| d == folded as u8)
        .map(|value| value as u8)
        .ok_or(ShareCodeError::BadCharacter(ch))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{HumanoidParams, QuadrupedParams};
    use crate::texture::SkinParams;

    #[test]
    fn a_look_round_trips() {
        let original = Archetype::Humanoid(HumanoidParams {
            height: 1.83,
            shoulder_width: -0.25,
            ..Default::default()
        });
        let code = encode(&original, &Composites::default(), &SkinParams::default());
        let (Archetype::Humanoid(back), _, _) = decode(&code).expect("decodes") else {
            panic!("archetype changed");
        };

        assert!((back.height - 1.83).abs() < 0.002);
        assert!((back.shoulder_width + 0.25).abs() < 0.01);
    }

    #[test]
    fn quadrupeds_round_trip_too() {
        let original = Archetype::Quadruped(QuadrupedParams {
            height: 0.9,
            tail_length: 1.0,
            leg_length: -0.5,
            ..Default::default()
        });
        let code = encode(&original, &Composites::default(), &SkinParams::default());
        let (Archetype::Quadruped(back), _, _) = decode(&code).expect("decodes") else {
            panic!("archetype changed");
        };
        assert!((back.height - 0.9).abs() < 0.002);
        assert!((back.tail_length - 1.0).abs() < 0.01);
        assert!((back.leg_length + 0.5).abs() < 0.01);
    }

    #[test]
    fn codes_stay_short_and_grouped() {
        let code = encode(
            &Archetype::default(),
            &Composites::default(),
            &SkinParams::default(),
        );
        // Short enough to read aloud or fit a QR code comfortably. Body plus
        // complexion is about a dozen bytes; the ceiling leaves room to grow.
        assert!(code.len() <= 48, "code is {} chars: {code}", code.len());
        assert!(code.contains('-'), "grouped for legibility: {code}");
    }

    #[test]
    fn codes_survive_being_written_down() {
        let code = encode(
            &Archetype::default(),
            &Composites::default(),
            &SkinParams::default(),
        );
        // Lower case, look-alike letters, stray spaces, missing hyphens.
        let mangled = code
            .to_lowercase()
            .replace('-', " ")
            .replace('1', "l")
            .replace('0', "O");
        assert_eq!(decode(&mangled), decode(&code));
    }

    #[test]
    fn a_mistyped_character_is_caught() {
        let code = encode(
            &Archetype::default(),
            &Composites::default(),
            &SkinParams::default(),
        );
        let digits: String = code.chars().filter(|c| *c != '-').collect();

        // Swap one digit for a different one; the checksum must notice.
        let mut wrong: Vec<char> = digits.chars().collect();
        let last = wrong.len() - 2;
        wrong[last] = if wrong[last] == '0' { '1' } else { '0' };
        let wrong: String = wrong.into_iter().collect();

        assert_eq!(decode(&wrong), Err(ShareCodeError::Checksum));
    }

    #[test]
    fn a_version_3_code_still_names_the_body_it_named() {
        // A version-3 payload assembled with the byte spans that version used
        // — ±1 per signed byte, 0..1 per unit byte — exactly as an old code in
        // someone's notes was written. Version 4 widened the spans (#160);
        // this is the contract that the old code goes on meaning the same
        // body.
        use crate::plan::{put_signed, put_unit};
        let mut payload = vec![3u8, 1]; // version 3, humanoid tag
        crate::plan::put_length(&mut payload, 1.83);
        put_signed(&mut payload, 0.5); // build
        put_unit(&mut payload, 0.75); // muscle
        for axis in [-0.25, 0.1, 0.3, -0.4, 0.2, 0.6, -0.7, 0.15] {
            put_signed(&mut payload, axis);
        }
        put_unit(&mut payload, 0.35); // melanin
        put_signed(&mut payload, 0.0); // undertone
        put_unit(&mut payload, 0.45); // blush
        put_unit(&mut payload, 0.0); // freckles
        put_unit(&mut payload, 0.0); // stubble
        payload.push(checksum(&payload));
        let code = group(&base32_encode(&payload));

        let (Archetype::Humanoid(back), composites, skin) =
            decode(&code).expect("a v3 code decodes")
        else {
            panic!("archetype changed");
        };
        assert!((back.height - 1.83).abs() < 0.002);
        assert!((back.shoulder_width + 0.25).abs() < 0.01);
        assert!((back.face_length + 0.7).abs() < 0.01);
        assert!((skin.melanin - 0.35).abs() < 0.01);
        assert_eq!(
            composites,
            Composites::default(),
            "a code written before composites existed described a body with the neutral ones"
        );
    }

    #[test]
    fn a_version_4_code_reads_without_composites_and_does_not_eat_the_complexion() {
        // The bug this guards is the one a mid-payload insertion invites: read a
        // v4 code as though it carried composites and the four bytes taken come
        // out of the COMPLEXION, so the body decodes fine and the skin is
        // nonsense. Assembled as a v4 writer would have written it.
        use crate::plan::{put_signed, put_span, put_unit};
        let mut payload = vec![4u8, 1]; // version 4, humanoid tag
        crate::plan::put_length(&mut payload, 1.72);
        let signed = crate::plan::explore_range(0.0, (-1.0, 1.0));
        put_span(&mut payload, 0.5, signed); // build
        put_span(
            &mut payload,
            0.75,
            crate::plan::explore_range(0.0, (0.0, 1.0)),
        ); // muscle
        for axis in [-0.25, 0.1, 0.3, -0.4, 0.2, 0.6, -0.7, 0.15] {
            put_span(&mut payload, axis, signed);
        }
        put_unit(&mut payload, 0.62); // melanin
        put_signed(&mut payload, -0.3); // undertone
        put_unit(&mut payload, 0.45); // blush
        put_unit(&mut payload, 0.0); // freckles
        put_unit(&mut payload, 0.0); // stubble
        payload.push(checksum(&payload));
        let code = group(&base32_encode(&payload));

        let (Archetype::Humanoid(back), composites, skin) =
            decode(&code).expect("a v4 code decodes")
        else {
            panic!("archetype changed");
        };
        assert!((back.height - 1.72).abs() < 0.002);
        assert_eq!(composites, Composites::default());
        assert!(
            (skin.melanin - 0.62).abs() < 0.01,
            "the complexion must not be read out of the composites' bytes"
        );
        assert!((skin.undertone + 0.3).abs() < 0.01);
    }

    #[test]
    fn a_version_5_code_carries_the_composites() {
        let composites = Composites {
            femininity: 0.8,
            mass: -0.6,
            body_fat: 0.34,
            age: 61,
        };
        let code = encode(&Archetype::default(), &composites, &SkinParams::default());
        let (_, back, _) = decode(&code).expect("decodes");
        assert!((back.femininity - 0.8).abs() < 0.03);
        assert!((back.mass + 0.6).abs() < 0.03);
        assert!((back.body_fat - 0.34).abs() < 0.01);
        assert!(back.age.abs_diff(61) <= 1);
    }

    #[test]
    fn a_version_4_code_carries_the_envelope() {
        // The point of the version bump: an extreme the record can hold
        // survives the code. Coarser per step than v3 — a ±3 axis moves in
        // ~0.024 — which is documented at `put_span` and below what a slider
        // shows.
        let original = Archetype::Humanoid(HumanoidParams {
            shoulder_width: 2.0,
            face_length: -2.2,
            ..Default::default()
        });
        let code = encode(&original, &Composites::default(), &SkinParams::default());
        let (Archetype::Humanoid(back), _, _) = decode(&code).expect("decodes") else {
            panic!("archetype changed");
        };
        assert!((back.shoulder_width - 2.0).abs() < 0.03);
        assert!((back.face_length + 2.2).abs() < 0.03);
    }

    #[test]
    fn malformed_codes_are_reported_precisely() {
        assert_eq!(decode(""), Err(ShareCodeError::TooShort));
        assert_eq!(decode("!!!!!"), Err(ShareCodeError::BadCharacter('!')));

        // A payload whose version byte is from the future.
        let mut payload = vec![SHARE_CODE_VERSION + 1, 1, 0, 0];
        payload.push(checksum(&payload));
        let future = base32_encode(&payload);
        assert_eq!(
            decode(&future),
            Err(ShareCodeError::UnsupportedVersion(SHARE_CODE_VERSION + 1))
        );
    }
}
