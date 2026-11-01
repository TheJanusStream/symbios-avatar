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
/// **12** - a beast's code ends with the colours a creator picked for its coat
/// (#401), after the stiffness byte, which this version always carries: a mask of
/// the picked channels, then three bytes for each. **Written only by a code that
/// needs it**: a beast that picked none goes on writing 11 or 10, byte for byte.
/// Versions up to 11 read as picking nothing, which is what they meant. A colour
/// goes as sRGB bytes, as lossy as the rest of a code; on the record it is
/// thousandths. After the colours, what a beast wears on its head (#402): a mask of
/// the kinds a `horn` byte cannot say, then each one's six spans and six colour
/// bytes, and an antler's points and fork. **Written only by a code that needs it**
/// as well, and a body wearing exactly a unicorn's horn needs none: it goes on saying
/// so in its `horn` byte at the version it always wrote. A code of this version that
/// ends at the colours wears nothing more, and top bits of the mask read as nothing.
///
/// **11** - a beast's code ends with one more byte, after the complexion: its
/// mane's stiffness (#389). **Written only by a code that needs it**: a person,
/// and a beast at the default stiffness, go on writing version 10, byte for
/// byte, so that a client that knows only 10 can still read them. Version 10
/// reads as the default stiffness, which is what every record meant before it
/// had one. A byte at the end rather than among the fur's, so that the layouts
/// before it stay the one `Archetype::decode` reads.
///
/// **10** - the quadruped payload loses `build` and `muscle`, which retired
/// into the composites (#178), and gains its family, the family it is crossed
/// with (`255` for none), how far it is blended toward it, and the six axes
/// that came with the families: `muzzleLength`, `stance`, `neckCarriage`,
/// `tailCarriage`, `chestDepth` and `appeal`, appended (#366). After them,
/// while 10 is unpublished, the head's (#367): the spans `muzzleBreadth`,
/// `stop` and `eyeSet`, the `pupil` byte, the spans `earSize`, `earPoint`
/// and `earCarriage`, the `nose` and `feet` bytes, the `whiskers` span and the
/// `horn` byte; then the coat's six bytes (#368) - its loci packed into
/// three, the `intensity` span and the two of its `marks` seed - and the
/// fur's five spans (#369): `mane`, `ruff`, `tail`, `feathering`, `tufts`. A
/// family, part or locus code this build does not know reads as its default -
/// a dog, round pupils, leather, clawed paws, no horn, the default coat's
/// locus - as an unknown name does on a record. Version 9 stays readable through `PRE_FAMILIES_VERSION` (private,
/// below): its `build` and `muscle` are read and dropped, and a code minted
/// before the families meant the plan's neutral dog, which is what every
/// default of the appended bytes says. The humanoid payload does not move.
///
/// **9** — the humanoid payload gains `chestSpacing` and `chestFullness`,
/// appended (#289). They are where a chest's volume sits — across the ribcage
/// and up it — over what the trunk's own constants derive, and the pair is the
/// last of milestone #9's axes. Version 8 stays readable through
/// `PRE_DISTRIBUTION_VERSION` (private, below), the same layout without the two
/// bytes: a code minted before them meant a chest sitting where the constants
/// put it, which IS the neutral offset.
///
/// **8** — the humanoid payload gains `chestVolume`, `chestProjection` and
/// `chestLift`, appended (#273). Version 7 and below stay readable through
/// `PRE_CHEST_VERSION` (private, below); a code written before the chest axes
/// existed described the chest its composites came to, which is their neutral.
///
/// **7** — the complexion loses its `stubble` byte. It followed exactly
/// the path `build` and `muscle` took: the thing it drove was replaced — first
/// by the painted hair layer, then by a density and a colour per follicle
/// region — and the axis was left on the wire, still written and still
/// read by nothing. Version 6 and below stay readable through
/// `PAINTED_STUBBLE_VERSION` (private, below), which is the same spans with the
/// slot still present; the byte is taken off the payload and dropped, because
/// what it said has no field left to say it into.
///
/// **6** — the humanoid payload loses the two dead bytes the retired `build`
/// and `muscle` axes left behind. Their slots were held rather than
/// removed so that codes already in circulation kept decoding at the right
/// offsets; this is the version that collects the removal. Version
/// 5 and 4 stay readable through `Archetype::decode_reserved`, which is the
/// same spans with the two slots still on the wire; the quadruped's payload is
/// unchanged, because its own `build` and `muscle` never retired.
///
/// **5** — the composites block: four bytes for the high-level axes,
/// written after the archetype and before the complexion. Added the moment
/// the axes existed, because a record field that
/// silently drops out of a share code is a look that changes when it is passed
/// between people — the one thing a code exists not to do. Version 4 stays
/// readable and decodes to the neutral composites, which is what a code written
/// before the axes existed meant.
///
/// **4** — the exploration envelope: each plan byte now spans its
/// axis's widened range rather than ±1, so a code can carry the extremes the
/// record can. The byte LAYOUT is version 3's, only the spans moved, which is
/// why 3 is not a refusal: `decode` reads a version-3 payload through the
/// old spans and an old code goes on meaning the body it named when it was
/// written down.
///
/// **3** — the humanoid plan gained `headBreadth` and `faceLength`, which
/// are two more bytes in the middle of its payload. A version 2 code read as a
/// version 3 one would take a head's breadth from what used to be the extremity
/// size and then run off the end, so the version gate is what keeps an old code
/// a clean refusal rather than a body nobody asked for. Codes are for passing a
/// look between people and re-encoding one was never a round trip; the record
/// is the canonical avatar and reads unchanged.
pub const SHARE_CODE_VERSION: u8 = 12;

/// The version every code that carries no mane stiffness is written at.
const PRE_STIFFNESS_VERSION: u8 = 10;

/// The version every code that carries no picked colours is written at, at most.
const PRE_COLOURS_VERSION: u8 = 11;

/// The oldest format whose codes still decode.
const OLDEST_VERSION: u8 = 3;

/// The last format written before the composites block existed.
///
/// Codes at or below it carry no composites and decode to the neutral ones,
/// which is exactly what they meant when they were written down.
const PRE_COMPOSITES_VERSION: u8 = 4;

/// The last format whose plan bytes span ±1 rather than the exploration
/// envelope.
const NARROW_SPAN_VERSION: u8 = 3;

/// The last format that carried the retired `build` and `muscle` slots.
const RESERVED_SLOTS_VERSION: u8 = 5;

/// The last format that carried the retired `stubble` byte.
///
/// Codes at or below it have one more unit byte after the freckles, which
/// `decode` takes and throws away: the complexion it belonged to has no field
/// for it, and the hair it described is on the record rather than in a code.
const PAINTED_STUBBLE_VERSION: u8 = 6;

/// The last version whose humanoid payload ended at `extremitySize`.
///
/// Version 8 appends `chestVolume`, `chestProjection` and `chestLift` (#273).
/// A version-7 code read as a version-8 one would run off the end of its own
/// payload, which is the whole reason the gate is here rather than a length
/// check: the checksum covers the body, so a short read is not otherwise an
/// error until the next field is added after it.
const PRE_CHEST_VERSION: u8 = 7;

/// The last version whose humanoid payload ended at `chestLift`.
///
/// Version 9 appends `chestSpacing` and `chestFullness` (#289), and the gate is
/// here for [`PRE_CHEST_VERSION`]'s reason exactly: the checksum covers the
/// bytes rather than the fields, so a version-8 code read through today's
/// layout would run two spans off the end of its own payload and the only thing
/// that catches it is knowing which layout wrote it.
const PRE_DISTRIBUTION_VERSION: u8 = 8;

/// The last version whose quadruped payload carried `build` and `muscle` and
/// ended at `tailLength`.
///
/// Version 10 drops the two and appends the family, its cross and six axes
/// (#366, #178), and the gate is here for [`PRE_CHEST_VERSION`]'s reason: a
/// version-9 quadruped read through today's layout would take its legs from
/// its `build` and its family from the composites block.
const PRE_FAMILIES_VERSION: u8 = 9;

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
    /// Every field was read and there were bytes left over.
    ///
    /// **A layout disagreement between the writer and the reader**, and the one
    /// failure this format could otherwise have without saying so. The
    /// checksum is over the bytes rather than over the fields, so a reader that
    /// stops one field early leaves the remainder on the payload and returns a
    /// body that decodes cleanly. It is harmless exactly until the next field is
    /// appended, at which point every code of that version reads one position
    /// early — which is the bug the version gate exists to prevent and could not
    /// catch here, because the reader and the writer are the same build.
    #[error("share code has {0} unread byte(s): its layout is not the one this build reads")]
    Trailing(usize),
}

/// Renders a body and complexion as a share code.
#[must_use]
pub fn encode(archetype: &Archetype, composites: &Composites, skin: &SkinParams) -> String {
    use crate::plan::{put_signed, put_unit};

    // Only a beast whose colours or ornaments a reader of version 11 would lose,
    // and after that, whose stiffness a reader of version 10 would get wrong. The
    // layout of version 12 carries the stiffness whatever it is.
    let colours = match archetype {
        Archetype::Quadruped(params)
            if !params.coat.colours.is_empty() || params.ornaments.needs_bytes() =>
        {
            Some((params.coat.colours, params.ornaments))
        }
        _ => None,
    };
    let stiffness = match archetype {
        Archetype::Quadruped(params)
            if colours.is_some() || params.fur.stiffness_needs_a_byte() =>
        {
            Some(params.fur.stiffness_byte())
        }
        _ => None,
    };
    let version = match (&colours, stiffness) {
        (Some(_), _) => SHARE_CODE_VERSION,
        (None, Some(_)) => PRE_COLOURS_VERSION,
        (None, None) => PRE_STIFFNESS_VERSION,
    };
    let mut payload = vec![version];
    archetype.encode(&mut payload);
    composites.encode(&mut payload);
    put_unit(&mut payload, skin.melanin);
    put_signed(&mut payload, skin.undertone);
    put_unit(&mut payload, skin.blush);
    put_unit(&mut payload, skin.freckles);
    payload.extend(stiffness);
    if let Some((colours, ornaments)) = colours {
        colours.encode(&mut payload);
        ornaments.encode(&mut payload);
    }
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

    // Six plan layouts are readable and the order of these arms is the order
    // they arrived in: version 3's narrow spans, versions 4 and 5's envelope
    // spans with the retired slots still on the wire, versions 6 and 7's
    // payload ending at `extremitySize`, version 8's ending at `chestLift`,
    // version 9's with the whole chest on it, and today's with the quadruped's
    // families.
    let mut archetype = if version <= NARROW_SPAN_VERSION {
        Archetype::decode_legacy(&mut payload)?
    } else if version <= RESERVED_SLOTS_VERSION {
        Archetype::decode_reserved(&mut payload)?
    } else if version <= PRE_CHEST_VERSION {
        Archetype::decode_pre_chest(&mut payload)?
    } else if version <= PRE_DISTRIBUTION_VERSION {
        Archetype::decode_pre_distribution(&mut payload)?
    } else if version <= PRE_FAMILIES_VERSION {
        Archetype::decode_pre_families(&mut payload)?
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
    };
    if version <= PAINTED_STUBBLE_VERSION {
        // Taken and dropped rather than left on the payload: the checksum
        // covers the whole body, so a byte that is not consumed is not an
        // error — it is a trailing byte nothing notices until the next field
        // is added after it and reads one position early.
        take_unit(&mut payload)?;
    }
    skin.sanitize();
    // Only a beast's: nothing writes a person at 11, and a person's byte too many
    // is the layout disagreement `Trailing` says it is.
    if version > PRE_STIFFNESS_VERSION
        && let Archetype::Quadruped(params) = &mut archetype
    {
        params.fur.take_stiffness(&mut payload)?;
        if version > PRE_COLOURS_VERSION {
            params.coat.colours = crate::plan::CoatColours::decode(&mut payload)?;
            params.ornaments.decode_over(&mut payload)?;
        }
    }
    if !payload.is_empty() {
        return Err(ShareCodeError::Trailing(payload.len()));
    }
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
    use crate::plan::{Family, HumanoidParams, QuadrupedParams};
    use crate::texture::SkinParams;

    /// The default person's code, as the tree before version 11 wrote it.
    const PERSON_V10: &str = "180XC-1M0G2-08104-0G208-1040G-20810-2N55C-R0WR0-XM";

    /// The default beast's code, as the tree before version 11 wrote it.
    const BEAST_V10: &str = "18148-0M0G2-08100-0ZW08-1040G-20810-40G00-81040-00080-00200-08000-00000-0000G-205AA-ASG1S-G1Y8";

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
    fn a_quadruped_s_family_and_cross_survive_a_code() {
        let original = QuadrupedParams {
            family: Family::Horse,
            hybrid: Some(Family::Cat),
            blend: 0.4,
            muzzle_length: 0.8,
            stance: 1.0,
            neck_carriage: 0.5,
            tail_carriage: -0.9,
            chest_depth: 0.5,
            muzzle_breadth: -0.6,
            stop: 0.7,
            eye_set: 0.9,
            pupil: crate::plan::Pupil::Slit,
            ear_size: 0.4,
            ear_point: -0.3,
            ear_carriage: -0.8,
            nose: crate::plan::Nose::Soft,
            feet: crate::plan::Feet::Hooves,
            whiskers: 0.7,
            ornaments: crate::plan::Ornaments {
                horn: Some(crate::plan::Ornament::UNICORN),
                ..Default::default()
            },
            ..Default::default()
        };
        let code = encode(
            &Archetype::Quadruped(original),
            &Composites::default(),
            &SkinParams::default(),
        );
        let (Archetype::Quadruped(back), _, _) = decode(&code).expect("decodes") else {
            panic!("archetype changed");
        };
        assert_eq!(back.family, Family::Horse);
        assert_eq!(back.hybrid, Some(Family::Cat));
        assert_eq!(
            back.pupil,
            crate::plan::Pupil::Slit,
            "a part is a byte, kept whole"
        );
        assert_eq!(back.nose, crate::plan::Nose::Soft);
        assert_eq!(back.feet, crate::plan::Feet::Hooves);
        assert_eq!(back.ornaments.horn, Some(crate::plan::Ornament::UNICORN));
        // A byte over each span: a signed axis's step is 6/255.
        for (name, was, now) in [
            ("blend", original.blend, back.blend),
            ("muzzle", original.muzzle_length, back.muzzle_length),
            ("stance", original.stance, back.stance),
            ("neck", original.neck_carriage, back.neck_carriage),
            ("tail", original.tail_carriage, back.tail_carriage),
            ("chest", original.chest_depth, back.chest_depth),
            ("breadth", original.muzzle_breadth, back.muzzle_breadth),
            ("stop", original.stop, back.stop),
            ("eye set", original.eye_set, back.eye_set),
            ("ear size", original.ear_size, back.ear_size),
            ("ear point", original.ear_point, back.ear_point),
            ("ear carriage", original.ear_carriage, back.ear_carriage),
            ("whiskers", original.whiskers, back.whiskers),
        ] {
            assert!((was - now).abs() < 0.013, "{name}: {was} came back {now}");
        }
        // And a body that is no cross says so.
        let code = encode(
            &Archetype::Quadruped(QuadrupedParams::default()),
            &Composites::default(),
            &SkinParams::default(),
        );
        let (Archetype::Quadruped(back), _, _) = decode(&code).expect("decodes") else {
            panic!("archetype changed");
        };
        assert_eq!(
            (back.family, back.hybrid, back.blend),
            (Family::Dog, None, 0.0)
        );
    }

    #[test]
    fn a_version_9_quadruped_code_reads_as_the_dog_it_named() {
        // Assembled as a v9 writer wrote it: the eight axes and nothing after
        // them. Read through today's layout, its family would come out of the
        // composites block and the complexion would run off the end.
        use crate::plan::{put_signed, put_span, put_unit};
        let mut payload = vec![9u8, 2]; // version 9, quadruped tag
        crate::plan::put_length(&mut payload, 0.9);
        let signed = crate::plan::explore_range(0.0, (-1.0, 1.0));
        let muscle = crate::plan::explore_range(0.0, (0.0, 1.0));
        put_span(&mut payload, 0.4, signed); // body length
        put_span(&mut payload, -0.3, signed); // build
        put_span(&mut payload, 0.5, muscle); // muscle
        for axis in [-0.5, 0.2, 0.1, 0.9] {
            put_span(&mut payload, axis, signed); // legs, neck, head, tail
        }
        Composites::default().encode(&mut payload);
        put_unit(&mut payload, 0.62); // melanin
        put_signed(&mut payload, -0.3); // undertone
        put_unit(&mut payload, 0.45); // blush
        put_unit(&mut payload, 0.20); // freckles
        payload.push(checksum(&payload));
        let code = group(&base32_encode(&payload));

        let (Archetype::Quadruped(back), _, skin) = decode(&code).expect("a v9 code decodes")
        else {
            panic!("archetype changed");
        };
        assert!((back.height - 0.9).abs() < 0.002);
        assert!((back.tail_length - 0.9).abs() < 0.013);
        assert_eq!(
            (back.family, back.hybrid, back.blend),
            (Family::Dog, None, 0.0)
        );
        assert_eq!(back.stance, 0.0, "an axis the code predates is its neutral");
        assert!((skin.freckles - 0.20).abs() < 0.01);
    }

    /// The version a code was written at.
    fn version_of(code: &str) -> u8 {
        base32_decode(code).expect("a code")[0]
    }

    #[test]
    fn a_person_and_a_default_beast_stay_at_version_10_byte_for_byte() {
        // Literal, minted by the tree before version 11 existed: a client that
        // knows only 10 reads these, and they must not move.
        let person = encode(
            &Archetype::default(),
            &Composites::default(),
            &SkinParams::default(),
        );
        assert_eq!(person, PERSON_V10);
        assert_eq!(version_of(&person), 10);
        let beast = Archetype::Quadruped(QuadrupedParams::default());
        let code = encode(&beast, &Composites::default(), &SkinParams::default());
        assert_eq!(code, BEAST_V10);
        assert_eq!(version_of(&code), 10);
        // A stiffness that lands on the default's byte says nothing either.
        let mut near = QuadrupedParams::default();
        near.fur.mane_stiffness = crate::plan::Fur::DEFAULT_MANE_STIFFNESS + 0.001;
        let code = encode(
            &Archetype::Quadruped(near),
            &Composites::default(),
            &SkinParams::default(),
        );
        assert_eq!(version_of(&code), 10);
    }

    #[test]
    fn a_version_10_beast_reads_as_the_default_stiffness() {
        let (Archetype::Quadruped(back), _, _) = decode(BEAST_V10).expect("a v10 code decodes")
        else {
            panic!("archetype changed");
        };
        assert_eq!(
            back.fur.mane_stiffness,
            crate::plan::Fur::DEFAULT_MANE_STIFFNESS
        );
    }

    #[test]
    fn a_stiff_mane_survives_a_code_at_version_11() {
        for stiffness in [0.0, 0.25, 0.8, 1.0] {
            let mut params = QuadrupedParams::default();
            params.fur.mane = 0.8;
            params.fur.mane_stiffness = stiffness;
            let skin = SkinParams {
                freckles: 0.2,
                ..SkinParams::default()
            };
            let code = encode(&Archetype::Quadruped(params), &Composites::default(), &skin);
            assert_eq!(version_of(&code), 11);
            let (Archetype::Quadruped(back), _, back_skin) = decode(&code).expect("decodes") else {
                panic!("archetype changed");
            };
            assert!(
                (back.fur.mane_stiffness - stiffness).abs() <= 0.5 / 255.0 + 0.0005,
                "{stiffness} came back {}",
                back.fur.mane_stiffness
            );
            assert!((back.fur.mane - 0.8).abs() < 0.013);
            assert!(
                (back_skin.freckles - 0.2).abs() < 0.01,
                "the complexion was read off the stiffness's byte"
            );
        }
    }

    /// A beast with `dark` and `tail` picked, at `stiffness`.
    fn coloured(stiffness: f32) -> QuadrupedParams {
        let mut params = QuadrupedParams::default();
        params.fur.mane_stiffness = stiffness;
        params.coat.colours.dark = Some([0.2, 0.4, 0.6]);
        params.coat.colours.tail = Some([1.0, 0.0, 0.0]);
        params.coat.sanitize();
        params.fur.sanitize();
        params
    }

    #[test]
    fn picked_colours_survive_a_code_at_version_12() {
        let default_stiffness = crate::plan::Fur::DEFAULT_MANE_STIFFNESS;
        for stiffness in [default_stiffness, 0.8] {
            let params = coloured(stiffness);
            let skin = SkinParams {
                melanin: 0.25,
                ..SkinParams::default()
            };
            let code = encode(&Archetype::Quadruped(params), &Composites::default(), &skin);
            assert_eq!(version_of(&code), 12);
            let (Archetype::Quadruped(back), _, read_skin) = decode(&code).expect("decodes") else {
                panic!("a quadruped");
            };
            assert!((read_skin.melanin - 0.25).abs() < 0.01);
            assert!((back.fur.mane_stiffness - stiffness).abs() <= 0.5 / 255.0 + 0.0005);
            assert!(back.coat.colours.light.is_none());
            for (read, wrote) in [
                (back.coat.colours.dark, params.coat.colours.dark),
                (back.coat.colours.tail, params.coat.colours.tail),
            ] {
                let (read, wrote) = (read.expect("came back"), wrote.expect("written"));
                for (read, wrote) in read.iter().zip(wrote) {
                    assert!((read - wrote).abs() < 0.003, "{wrote} came back {read}");
                }
            }
            // A code read and written again is the same code.
            let again = encode(
                &Archetype::Quadruped(back),
                &Composites::default(),
                &read_skin,
            );
            assert_eq!(again, code);
        }
    }

    #[test]
    fn codes_up_to_11_read_as_picking_nothing_and_go_on_being_written() {
        // Literals: `BEAST_V10` is above, and this one was minted by the tree
        // before version 12 existed, at a mane stiffness of 0.8.
        const BEAST_V11: &str = "1C148-0M0G2-08100-0ZW08-1040G-20810-40G00-81040-00080-00200-08000-00000-0000G-205AA-ASG1S-G1K06";
        for (code, version) in [(BEAST_V10, 10), (BEAST_V11, 11)] {
            assert_eq!(version_of(code), version);
            let (Archetype::Quadruped(back), composites, skin) = decode(code).expect("decodes")
            else {
                panic!("a quadruped");
            };
            assert!(back.coat.colours.is_empty());
            let again = encode(&Archetype::Quadruped(back), &composites, &skin);
            assert_eq!(
                again, code,
                "written back the same bytes at the same version"
            );
        }
        // A person never reads a colour off the end of a code.
        let (person, composites, skin) = decode(PERSON_V10).expect("decodes");
        assert_eq!(encode(&person, &composites, &skin), PERSON_V10);
    }

    #[test]
    fn a_unicorn_goes_on_writing_what_it_wrote() {
        // Literals, minted by the tree before ornaments existed: `horn` was a byte.
        const DOG: &str = "18148-0M0G2-08100-0ZW08-1040G-20810-40G00-81040-00080-08200-08000-00000-0000G-205AA-ASG1S-G05G";
        const HORSE: &str = "181AM-1ANG2-J7PTG-2ZW09-ZAMQA-TQR0X-V7MR1-7F8X8-04160-08200-08000-00000-0000G-205AA-ASG1S-G0H8";
        for code in [DOG, HORSE] {
            assert_eq!(version_of(code), 10);
            let (Archetype::Quadruped(back), composites, skin) = decode(code).expect("decodes")
            else {
                panic!("a quadruped");
            };
            assert_eq!(back.ornaments.horn, Some(crate::plan::Ornament::UNICORN));
            let again = encode(&Archetype::Quadruped(back), &composites, &skin);
            assert_eq!(again, code, "the same bytes at the same version");
        }
    }

    /// A beast wearing every kind, none of them what the `horn` byte can say.
    fn ornate() -> QuadrupedParams {
        let mut params = QuadrupedParams::default();
        for kind in crate::plan::OrnamentKind::ALL {
            params.ornaments.wear(kind, true);
        }
        params.ornaments.horn.as_mut().expect("worn").curve = 0.25;
        params.ornaments.tusks.as_mut().expect("worn").base = [0.1, 0.2, 0.3];
        let antlers = params.ornaments.antlers.as_mut().expect("worn");
        antlers.points = 7;
        antlers.fork = 0.8;
        params
    }

    #[test]
    fn ornaments_survive_a_code_at_version_12_after_the_colours() {
        let mut params = ornate();
        params.coat.colours.tail = Some([1.0, 0.0, 0.0]);
        let skin = SkinParams::default();
        let code = encode(&Archetype::Quadruped(params), &Composites::default(), &skin);
        assert_eq!(version_of(&code), 12);
        let (Archetype::Quadruped(back), composites, skin) = decode(&code).expect("decodes") else {
            panic!("a quadruped");
        };
        assert_eq!(back.coat.colours.tail.map(|tail| tail[0]), Some(1.0));
        let tusks = back.ornaments.tusks.expect("came back");
        assert!((tusks.base[1] - 0.2).abs() < 0.003);
        let antlers = back.ornaments.antlers.expect("came back");
        assert_eq!(antlers.points, 7);
        assert!((antlers.fork - 0.8).abs() < 0.003);
        assert!((back.ornaments.horn.expect("came back").curve - 0.25).abs() < 0.007);
        assert_eq!(
            encode(&Archetype::Quadruped(back), &composites, &skin),
            code
        );
        // With no colours picked, the mask of none goes before it.
        let code = encode(
            &Archetype::Quadruped(ornate()),
            &Composites::default(),
            &SkinParams::default(),
        );
        assert_eq!(version_of(&code), 12);
        let (Archetype::Quadruped(back), _, _) = decode(&code).expect("decodes") else {
            panic!("a quadruped");
        };
        assert!(back.coat.colours.is_empty());
        assert!(back.ornaments.antlers.is_some());
        // A unicorn beside a tusk keeps its byte and spends none of its own on the horn.
        let mut both = QuadrupedParams::default();
        both.ornaments.horn = Some(crate::plan::Ornament::UNICORN);
        both.ornaments.wear(crate::plan::OrnamentKind::Tusks, true);
        let code = encode(
            &Archetype::Quadruped(both),
            &Composites::default(),
            &SkinParams::default(),
        );
        let (Archetype::Quadruped(back), _, _) = decode(&code).expect("decodes") else {
            panic!("a quadruped");
        };
        assert_eq!(back.ornaments.horn, Some(crate::plan::Ornament::UNICORN));
        assert!(back.ornaments.tusks.is_some());
    }

    #[test]
    fn a_code_cut_short_inside_its_ornaments_is_an_error_not_a_panic() {
        let code = encode(
            &Archetype::Quadruped(ornate()),
            &Composites::default(),
            &SkinParams::default(),
        );
        let bytes = base32_decode(&code).expect("a code");
        let body = &bytes[..bytes.len() - 1];
        let mask_of_none_ends = encode_length_without_ornaments();
        for cut in mask_of_none_ends..body.len() {
            let mut cut_short = body[..cut].to_vec();
            cut_short.push(checksum(&cut_short));
            let read = decode(&base32_encode(&cut_short));
            if cut == mask_of_none_ends {
                let (Archetype::Quadruped(back), _, _) = read.expect("ends at the colours") else {
                    panic!("a quadruped");
                };
                assert!(back.ornaments.is_empty(), "wears nothing further");
            } else {
                assert!(read.is_err(), "cut at {cut} of {}", body.len());
            }
        }
        // Bits no kind has yet read as nothing, and then the bytes after them, which
        // only a newer layout would write, are the disagreement `Trailing` says.
        let mut unknown = body[..mask_of_none_ends].to_vec();
        unknown.push(0b1100_0000);
        unknown.push(checksum(&unknown));
        let (Archetype::Quadruped(back), _, _) = decode(&base32_encode(&unknown)).expect("reads")
        else {
            panic!("a quadruped");
        };
        assert!(back.ornaments.is_empty());
    }

    /// How long a default beast's code body runs at version 12, up to where its
    /// ornaments start: the version byte, the payload, the stiffness and the mask of
    /// no colours.
    fn encode_length_without_ornaments() -> usize {
        let mut payload = vec![SHARE_CODE_VERSION];
        Archetype::Quadruped(QuadrupedParams::default()).encode(&mut payload);
        Composites::default().encode(&mut payload);
        payload.len() + 4 + 1 + 1
    }

    #[test]
    fn codes_stay_short_and_grouped() {
        let code = encode(
            &Archetype::default(),
            &Composites::default(),
            &SkinParams::default(),
        );
        // Short enough to read aloud or fit a QR code comfortably. The ceiling
        // is room to grow rather than a measurement of today: the neutral code
        // is 50 characters at version 9, and each byte an appended axis costs
        // is 1.6 base32 digits plus its share of a hyphen — 1.9 characters, so
        // this leaves about three more axes.
        //
        // **It was 48 and version 9 went through it** (#289): the two
        // distribution axes are two bytes and took the code from 46 to 50.
        // Raised rather than worked around, because what the bound is FOR is a
        // string a person can read down a phone line and photograph, and fifty
        // Crockford digits in ten groups is still both. A code that had to drop
        // axes to stay under a number would be the one thing this format exists
        // not to do — a look that changes when it is passed between people.
        assert!(code.len() <= 56, "code is {} chars: {code}", code.len());
        assert!(code.contains('-'), "grouped for legibility: {code}");

        // Version 10 reshaped the QUADRUPED's payload alone (#366, #178): two
        // spans out, the family, its cross and seven spans in, and the head's
        // axes and parts after them (#367). A beast's body is nearly all in
        // its plan - a person's look is spread over the face, the hair and
        // the outfit too - so its code runs longer than a person's under a
        // ceiling of its own, still a string a person can read out: a hundred
        // characters, hyphens and all, which holds the coat's six bytes
        // (#368) and leaves the fur's (#369) room.
        let beast = encode(
            &Archetype::Quadruped(QuadrupedParams::default()),
            &Composites::default(),
            &SkinParams::default(),
        );
        assert!(beast.len() <= 100, "code is {} chars: {beast}", beast.len());
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
    fn a_version_6_code_reads_without_a_field_for_its_stubble_byte() {
        // **A byte that has nowhere to go still has to come off the payload**
        // (#212). The complexion lost `stubble` and a version-6 code still
        // carries it; the checksum covers the whole body, so an unconsumed
        // trailing byte is not an error — it is a byte that goes unnoticed until
        // the next field is appended after the complexion and reads one position
        // early, which is a body nobody asked for and a clean decode.
        //
        // Assembled as a v6 writer would have written it, with a stubble value
        // that is deliberately NOT zero: a version that dropped the byte
        // silently would still pass this if the byte were zero, because the
        // fields before it would decode identically and the checksum is over the
        // bytes rather than over the fields.
        use crate::plan::{put_signed, put_span, put_unit};
        let mut payload = vec![6u8, 1]; // version 6, humanoid tag
        crate::plan::put_length(&mut payload, 1.68);
        let signed = crate::plan::explore_range(0.0, (-1.0, 1.0));
        for axis in [-0.25, 0.1, 0.3, -0.4, 0.2, 0.6, -0.7, 0.15] {
            put_span(&mut payload, axis, signed);
        }
        Composites::default().encode(&mut payload);
        put_unit(&mut payload, 0.62); // melanin
        put_signed(&mut payload, -0.3); // undertone
        put_unit(&mut payload, 0.45); // blush
        put_unit(&mut payload, 0.20); // freckles
        put_unit(&mut payload, 0.90); // stubble, and it is not zero on purpose
        payload.push(checksum(&payload));
        let code = group(&base32_encode(&payload));

        let (Archetype::Humanoid(back), _, skin) = decode(&code).expect("a v6 code decodes") else {
            panic!("archetype changed");
        };
        assert!((back.height - 1.68).abs() < 0.002);
        // The complexion is the code's own and not shifted by the byte that
        // followed it: freckles is the field a mis-read would land on.
        assert!((skin.melanin - 0.62).abs() < 0.01);
        assert!((skin.blush - 0.45).abs() < 0.01);
        assert!(
            (skin.freckles - 0.20).abs() < 0.01,
            "freckles decoded to {}, so the stubble byte was read as something \
             else",
            skin.freckles
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
    fn a_code_carries_the_composites() {
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
    fn a_version_5_code_still_names_the_body_it_named() {
        // The guard on version 6's removal (#169), and the same shape of bug
        // the v4 test above guards: a v5 payload carries two dead bytes where
        // `build` and `muscle` were, and reading it with today's layout takes
        // the shoulders from one of them and runs every later axis — and then
        // the composites, and then the complexion — one span short. Assembled
        // as a v5 writer would have written it.
        use crate::plan::{put_signed, put_span, put_unit};
        let mut payload = vec![5u8, 1]; // version 5, humanoid tag
        crate::plan::put_length(&mut payload, 1.68);
        let signed = crate::plan::explore_range(0.0, (-1.0, 1.0));
        put_span(&mut payload, 0.0, signed); // the retired build slot
        put_span(&mut payload, 0.0, signed); // the retired muscle slot
        for axis in [-0.25, 0.1, 0.3, -0.4, 0.2, 0.6, -0.7, 0.15] {
            put_span(&mut payload, axis, signed);
        }
        // The composites block, which version 5 was the first to carry.
        Composites {
            femininity: 0.5,
            mass: 0.25,
            body_fat: 0.30,
            age: 55,
        }
        .encode(&mut payload);
        put_unit(&mut payload, 0.62); // melanin
        put_signed(&mut payload, -0.3); // undertone
        put_unit(&mut payload, 0.45); // blush
        put_unit(&mut payload, 0.0); // freckles
        put_unit(&mut payload, 0.0); // stubble
        payload.push(checksum(&payload));
        let code = group(&base32_encode(&payload));

        let (Archetype::Humanoid(back), composites, skin) =
            decode(&code).expect("a v5 code decodes")
        else {
            panic!("archetype changed");
        };
        assert!((back.height - 1.68).abs() < 0.002);
        assert!(
            (back.shoulder_width + 0.25).abs() < 0.03,
            "the first axis after the dead slots is where a mis-read starts: {}",
            back.shoulder_width
        );
        assert!((back.extremity_size - 0.15).abs() < 0.03);
        assert!((composites.femininity - 0.5).abs() < 0.03);
        assert!(composites.age.abs_diff(55) <= 1);
        assert!(
            (skin.melanin - 0.62).abs() < 0.01,
            "the complexion must not be read out of the retired slots"
        );
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

        // A well-formed code of the current version with one byte too many
        // (#212). It checksums, its version is known and every field it
        // declares reads back — and it is still a code this build cannot read,
        // because whoever wrote it was writing a layout with something more in
        // it. Silently ignoring the remainder is how a reader ends up one field
        // out on the version after next.
        // Re-checksummed after the spare byte is added, so a mistyped code is
        // not what this measures — those are a different failure and must not
        // be reported as this one.
        //
        // **Built through the real encoder rather than by hand**, and that is
        // #273's own lesson rather than tidiness: hand-written, the payload
        // listed eight signed axes, so when the chest added three the spare
        // byte was swallowed as one of them and this test passed by reading a
        // truncated code. A fixture that has to be updated whenever the layout
        // moves is a fixture that stops testing the layout.
        let mut payload = vec![SHARE_CODE_VERSION];
        Archetype::Humanoid(crate::plan::HumanoidParams {
            height: 1.7,
            ..Default::default()
        })
        .encode(&mut payload);
        Composites::default().encode(&mut payload);
        crate::plan::put_unit(&mut payload, 0.35); // melanin
        crate::plan::put_signed(&mut payload, 0.0); // undertone
        crate::plan::put_unit(&mut payload, 0.45); // blush
        crate::plan::put_unit(&mut payload, 0.0); // freckles
        payload.push(0); // the byte too many
        payload.push(checksum(&payload));
        assert!(
            matches!(
                decode(&base32_encode(&payload)),
                Err(ShareCodeError::Trailing(_))
            ),
            "a code with a byte left over was not reported as one"
        );
    }
}
