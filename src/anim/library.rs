//! A file of baked clips, and the reader and writer for it.
//!
//! [`PoseClip`] is what the retargeter produces and what [`Play`](super::Play)
//! consumes; this
//! is how a set of them survives being written down. The artifact this crate
//! carries is at `assets/clips.bin`, baked by `examples/bakeclips` from the CC0
//! reference library — see `docs/clips.md` for which clip came from which file
//! under which licence.
//!
//! # Why not serde
//!
//! [`PoseClip`] already derives `Serialize`, so JSON was free and the choice was
//! measured rather than argued about. Baked to JSON the twelve-clip set is
//! **3.2 times** the size it is here — 639.8 KiB against 199.7 — and the reason
//! is that the payload is almost entirely `i16` quadruplets. A rotation costs 8
//! bytes here; as JSON it averages 26, because every component is written out as
//! digits with a sign, two brackets and three commas around them. Two thirds of
//! a checked-in artifact being punctuation is a bad trade for a repository.
//!
//! JSON's real argument is that a human can edit it, and nobody hand-edits
//! quantised quaternions. So: a little-endian binary layout, in character with
//! [`crate::gltf`], costing no dependency this crate did not already have.
//!
//! # The layout
//!
//! Little-endian throughout. Sizes are what the file holds, not what is in
//! memory.
//!
//! ```text
//! magic      8   "SYMBCLIP"
//! version    2   u16, currently 1
//! clips      2   u16
//! per clip:
//!   name     2   u16 length, then that many UTF-8 bytes
//!   rate     4   f32, frames per second
//!   frames   4   u32
//!   looping  1   u8, 0 or 1
//!   tracks   2   u16
//!   roots    4   u32, root positions, 0 for a clip that stays put
//!   per track:
//!     zone   1   u8, `Zone::index`
//!     slot   1   u8, the ordinal within that zone
//!     kind   1   u8, 0 held or 1 sampled
//!     value  8   i16 × 4 for a held curve
//!            8n  i16 × 4 × `frames` for a sampled one
//!   per root:
//!     xyz   12   f32 × 3
//! ```
//!
//! **A sampled track carries exactly `frames` rotations and the file does not
//! repeat the count.** That is the one redundancy worth removing: at 65 tracks a
//! per-track length would cost a quarter of a kilobyte per clip to say the same
//! thing 65 times, and a track of a different length than its clip is not a
//! thing [`PoseClip`] can represent anyway. [`ClipLibrary::read`] therefore
//! treats a short file as corrupt rather than as a short track.
//!
//! # What it does not do
//!
//! No compression, and this one is a judgement rather than a measurement going
//! one way: `gzip -9` takes the twelve-clip artifact to 117.9 KiB, which is 59%
//! of it, so the saving is real. It is declined because both places the file
//! actually lives already compress it — git stores a blob deflated, and any
//! server sends it under `content-encoding` — so an in-format compressor would
//! pay a dependency and a wasm decompressor to do a third time what is already
//! being done twice.
//!
//! No forward compatibility beyond the version number. A reader that meets a
//! version it does not know refuses the file rather than guessing, because the
//! failure mode of guessing is a body that moves wrongly rather than one that
//! does not move.

use std::collections::HashMap;

use glam::Vec3;

use crate::anim::pose_clip::{Curve, JointTrack, PoseClip, Slot};
use crate::plan::Zone;

/// What every clip file starts with.
const MAGIC: &[u8; 8] = b"SYMBCLIP";

/// The layout this build writes, and the only one it reads.
const VERSION: u16 = 1;

/// The checked-in artifact, embedded.
///
/// Bytes rather than a parsed library, because parsing costs a few hundred
/// microseconds and a consumer that wants the bytes — to hand to an asset
/// system, or to hash — should not have to re-serialise them to get them back.
#[cfg(feature = "builtin-clips")]
pub const BUILTIN: &[u8] = include_bytes!("../../assets/clips.bin");

/// Why a clip file could not be read.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum LibraryError {
    /// It does not begin with the eight bytes `SYMBCLIP`.
    #[error("not a clip library: it does not start with SYMBCLIP")]
    NotAClipLibrary,
    /// It is a version this build does not know.
    #[error("clip library version {found}, but this build reads {VERSION}")]
    WrongVersion {
        /// The version the file declared.
        found: u16,
    },
    /// It ended in the middle of something.
    #[error("clip library ends after {read} bytes, in the middle of {what}")]
    Truncated {
        /// What was being read when it ran out.
        what: &'static str,
        /// How many bytes had been consumed.
        read: usize,
    },
    /// A field held something that is not a value of its type.
    #[error("clip library holds {found} where a {what} belongs")]
    NotAValue {
        /// Which field.
        what: &'static str,
        /// What was there.
        found: u32,
    },
    /// A clip's name is not UTF-8.
    #[error("a clip name is not UTF-8")]
    NameIsNotText,
}

/// A set of baked clips, addressed by name.
///
/// Order is the order they were baked in, which is the order `bakeclips` lists
/// them, so a file round-trips to an equal library rather than to a merely
/// equivalent one.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ClipLibrary {
    /// Every clip, in the order the file holds them.
    pub clips: Vec<PoseClip>,
}

impl ClipLibrary {
    /// An empty library.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The twelve clips this crate carries, read from the embedded artifact.
    ///
    /// **Behind the `builtin-clips` feature, which is off.** The artifact is
    /// 200 KiB and a consumer that only builds bodies should not carry it —
    /// least of all a wasm one, where every byte of the library is a byte of
    /// the download. A consumer that would rather fetch `assets/clips.bin` at
    /// run time reads it with [`Self::read`] and pays nothing.
    ///
    /// # Errors
    ///
    /// Returns [`LibraryError`] if the embedded artifact does not parse, which
    /// would mean this build's reader and the checked-in file have gone out of
    /// step — `tests/clips.rs` exists to make that a test failure rather than a
    /// run-time one.
    #[cfg(feature = "builtin-clips")]
    pub fn builtin() -> Result<Self, LibraryError> {
        Self::read(BUILTIN)
    }

    /// How many clips it holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.clips.len()
    }

    /// Whether it holds none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.clips.is_empty()
    }

    /// The clip of that name, if it has one.
    ///
    /// A linear scan, deliberately: a curated set is a dozen clips and a map
    /// over a dozen strings costs more to build than every lookup it saves.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&PoseClip> {
        self.clips.iter().find(|clip| clip.name == name)
    }

    /// Every clip's name, in file order.
    #[must_use]
    pub fn names(&self) -> Vec<&str> {
        self.clips.iter().map(|clip| clip.name.as_str()).collect()
    }

    /// The whole library as bytes, in the layout the module documents.
    #[must_use]
    pub fn write(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.bytes());
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&VERSION.to_le_bytes());
        out.extend_from_slice(&(self.clips.len() as u16).to_le_bytes());
        for clip in &self.clips {
            let name = clip.name.as_bytes();
            out.extend_from_slice(&(name.len() as u16).to_le_bytes());
            out.extend_from_slice(name);
            out.extend_from_slice(&clip.rate.to_le_bytes());
            out.extend_from_slice(&(clip.frames as u32).to_le_bytes());
            out.push(u8::from(clip.looping));
            out.extend_from_slice(&(clip.tracks.len() as u16).to_le_bytes());
            out.extend_from_slice(&(clip.root.len() as u32).to_le_bytes());
            for track in &clip.tracks {
                out.push(track.slot.zone.index());
                out.push(track.slot.index);
                match &track.rotation {
                    Curve::Held(value) => {
                        out.push(0);
                        put_quat(&mut out, *value);
                    }
                    Curve::Sampled(values) => {
                        out.push(1);
                        for value in values {
                            put_quat(&mut out, *value);
                        }
                    }
                }
            }
            for at in &clip.root {
                out.extend_from_slice(&at.x.to_le_bytes());
                out.extend_from_slice(&at.y.to_le_bytes());
                out.extend_from_slice(&at.z.to_le_bytes());
            }
        }
        out
    }

    /// How many bytes [`Self::write`] will produce.
    ///
    /// Computed rather than measured, so a size can be reported without holding
    /// the bytes. It is the same walk `write` makes, which is why they are next
    /// to each other: two walks that can disagree eventually do.
    #[must_use]
    pub fn bytes(&self) -> usize {
        let mut total = MAGIC.len() + 2 + 2;
        for clip in &self.clips {
            total += 2 + clip.name.len() + 4 + 4 + 1 + 2 + 4;
            for track in &clip.tracks {
                total += 3 + track.rotation.bytes();
            }
            total += clip.root.len() * 12;
        }
        total
    }

    /// Reads a library back.
    ///
    /// # Errors
    ///
    /// Returns [`LibraryError`] for anything that is not a clip library this
    /// build writes: wrong magic, wrong version, a truncated file, a zone byte
    /// that names no zone, or a name that is not UTF-8.
    pub fn read(bytes: &[u8]) -> Result<Self, LibraryError> {
        // Built once and shared by every track: `Zone::all()` is the inverse of
        // `Zone::index`, and calling it per track would allocate a vector for
        // every one of the eight hundred a library holds.
        let zones: HashMap<u8, Zone> = Zone::all().into_iter().map(|z| (z.index(), z)).collect();

        let mut at = 0usize;
        let mut take = |what: &'static str, want: usize| -> Result<&[u8], LibraryError> {
            let end = at.checked_add(want).ok_or(LibraryError::Truncated {
                what,
                read: bytes.len(),
            })?;
            let slice = bytes.get(at..end).ok_or(LibraryError::Truncated {
                what,
                read: bytes.len(),
            })?;
            at = end;
            Ok(slice)
        };

        if take("the magic", 8)? != MAGIC {
            return Err(LibraryError::NotAClipLibrary);
        }
        let version = u16le(take("the version", 2)?);
        if version != VERSION {
            return Err(LibraryError::WrongVersion { found: version });
        }

        let count = u16le(take("the clip count", 2)?) as usize;
        let mut clips = Vec::with_capacity(count);
        for _ in 0..count {
            let length = u16le(take("a clip name's length", 2)?) as usize;
            let name = std::str::from_utf8(take("a clip name", length)?)
                .map_err(|_| LibraryError::NameIsNotText)?
                .to_string();
            let rate = f32le(take("a clip's rate", 4)?);
            let frames = u32le(take("a clip's frame count", 4)?) as usize;
            let looping = take("a clip's loop flag", 1)?[0] != 0;
            let tracks = u16le(take("a clip's track count", 2)?) as usize;
            let roots = u32le(take("a clip's root count", 4)?) as usize;

            let mut baked = Vec::with_capacity(tracks);
            for _ in 0..tracks {
                let head = take("a track's header", 3)?;
                let (zone, index, kind) = (head[0], head[1], head[2]);
                let zone = *zones.get(&zone).ok_or(LibraryError::NotAValue {
                    what: "zone",
                    found: u32::from(zone),
                })?;
                let rotation = match kind {
                    0 => Curve::Held(quat(take("a held rotation", 8)?)),
                    1 => {
                        let mut values = Vec::with_capacity(frames);
                        for _ in 0..frames {
                            values.push(quat(take("a sampled rotation", 8)?));
                        }
                        Curve::Sampled(values)
                    }
                    other => {
                        return Err(LibraryError::NotAValue {
                            what: "curve kind",
                            found: u32::from(other),
                        });
                    }
                };
                baked.push(JointTrack {
                    slot: Slot::new(zone, index),
                    rotation,
                });
            }

            let mut root = Vec::with_capacity(roots);
            for _ in 0..roots {
                let xyz = take("a root position", 12)?;
                root.push(Vec3::new(
                    f32le(&xyz[0..4]),
                    f32le(&xyz[4..8]),
                    f32le(&xyz[8..12]),
                ));
            }

            clips.push(PoseClip {
                name,
                rate,
                frames,
                looping,
                tracks: baked,
                root,
            });
        }

        Ok(Self { clips })
    }
}

/// Writes one packed rotation.
fn put_quat(out: &mut Vec<u8>, value: [i16; 4]) {
    for component in value {
        out.extend_from_slice(&component.to_le_bytes());
    }
}

/// Reads one packed rotation from exactly eight bytes.
fn quat(bytes: &[u8]) -> [i16; 4] {
    let at = |n: usize| i16::from_le_bytes([bytes[n], bytes[n + 1]]);
    [at(0), at(2), at(4), at(6)]
}

/// Reads a little-endian `u16` from exactly two bytes.
fn u16le(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([bytes[0], bytes[1]])
}

/// Reads a little-endian `u32` from exactly four bytes.
fn u32le(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

/// Reads a little-endian `f32` from exactly four bytes.
fn f32le(bytes: &[u8]) -> f32 {
    f32::from_bits(u32le(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::Limb;

    /// Two clips with one of everything the format can hold.
    fn library() -> ClipLibrary {
        ClipLibrary {
            clips: vec![
                PoseClip {
                    name: "Held".into(),
                    rate: 30.0,
                    frames: 3,
                    looping: true,
                    tracks: vec![JointTrack {
                        slot: Slot::new(Zone::Chest, 1),
                        rotation: Curve::Held([1, -2, 3, 32767]),
                    }],
                    root: Vec::new(),
                },
                PoseClip {
                    name: "Sampled and travelling".into(),
                    rate: 24.0,
                    frames: 2,
                    looping: false,
                    tracks: vec![
                        JointTrack {
                            slot: Slot::new(Zone::Extremity(Limb::ForeLeft), 20),
                            rotation: Curve::Sampled(vec![[0, 0, 0, 32767], [-1, 5, -9, 32000]]),
                        },
                        JointTrack {
                            slot: Slot::new(Zone::Head, 0),
                            rotation: Curve::Held([0, 0, 0, 32767]),
                        },
                    ],
                    root: vec![Vec3::new(0.0, 1.5, -2.25), Vec3::new(0.125, 1.5, -2.0)],
                },
            ],
        }
    }

    #[test]
    fn a_library_round_trips_through_its_own_bytes() {
        let before = library();
        let bytes = before.write();
        assert_eq!(
            bytes.len(),
            before.bytes(),
            "the size this reports is not the size it writes"
        );
        assert_eq!(ClipLibrary::read(&bytes).expect("it reads back"), before);
    }

    #[test]
    fn a_file_that_is_not_a_clip_library_is_refused() {
        assert_eq!(
            ClipLibrary::read(b"not a clip library at all"),
            Err(LibraryError::NotAClipLibrary)
        );
        assert!(matches!(
            ClipLibrary::read(b"SYMB"),
            Err(LibraryError::Truncated { .. })
        ));

        let mut wrong = library().write();
        wrong[8] = 99;
        assert_eq!(
            ClipLibrary::read(&wrong),
            Err(LibraryError::WrongVersion { found: 99 })
        );
    }

    #[test]
    fn a_truncated_file_is_refused_rather_than_read_short() {
        // **The reason a sampled track carries no length of its own.** A file
        // cut anywhere has to fail, because the alternative to failing is a
        // track that reads its neighbour's bytes as rotations and produces a
        // body that moves wrongly rather than one that does not move.
        let whole = library().write();
        for cut in 1..whole.len() {
            assert!(
                ClipLibrary::read(&whole[..cut]).is_err(),
                "a file cut to {cut} of {} bytes was read as whole",
                whole.len()
            );
        }
    }

    #[test]
    fn a_zone_byte_that_names_no_zone_is_refused() {
        let mut bad = library().write();
        // The first track's zone byte: past the header, the name, and the
        // clip's five scalar fields.
        let at = 12 + 2 + "Held".len() + 4 + 4 + 1 + 2 + 4;
        bad[at] = 200;
        assert_eq!(
            ClipLibrary::read(&bad),
            Err(LibraryError::NotAValue {
                what: "zone",
                found: 200
            })
        );
    }

    #[test]
    fn an_empty_library_is_a_header_and_nothing_else() {
        let empty = ClipLibrary::new();
        assert!(empty.is_empty());
        assert_eq!(empty.bytes(), 12);
        assert_eq!(ClipLibrary::read(&empty.write()).expect("reads"), empty);
    }

    #[cfg(feature = "builtin-clips")]
    #[test]
    fn the_embedded_artifact_is_the_one_on_disk() {
        // `include_bytes!` is resolved at compile time, so a stale build could
        // carry an artifact that no longer matches the file `bakeclips` wrote.
        // This is the only place that can notice.
        let on_disk = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/clips.bin"))
            .expect("assets/clips.bin is checked in");
        assert_eq!(
            BUILTIN,
            on_disk.as_slice(),
            "the embedded artifact is stale"
        );

        let library = ClipLibrary::builtin().expect("the embedded artifact parses");
        assert_eq!(library.len(), 12);
        assert!(library.get("Walk").is_some());
    }

    #[test]
    fn clips_are_found_by_name() {
        let library = library();
        assert_eq!(library.len(), 2);
        assert_eq!(library.names(), vec!["Held", "Sampled and travelling"]);
        assert_eq!(library.get("Held").map(|clip| clip.frames), Some(3));
        assert!(library.get("Walk").is_none());
    }
}
