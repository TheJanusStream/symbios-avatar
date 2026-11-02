//! Sixteen seeds of people and beasts, one line a fact, for comparing two trees line by
//! line: `scripts/compare-published.sh` runs it on HEAD and on the working tree. Every
//! field of each built avatar by hash, its texture maps included, for a person and for a
//! beast at several stiffnesses; records and codes; and in full, for reading, what an
//! eye goes to first: a person's rig and meshes, a beast's rig and body, a beast's fur
//! without a mane and with a limp one, and the hair it draws without one, every channel:
//! the fur a beast keeps is not what it draws, which is shaded on its way there (#407).
use std::fmt::Write as _;
use symbios_avatar::{Archetype, Avatar, AvatarRecord, MeshKind, QuadrupedParams, TextureMap};

/// FNV-1a over what is written to it, so that a whole mesh costs one line.
struct Fnv(u64);

impl Fnv {
    fn new() -> Self {
        Self(0xcbf29ce484222325)
    }
    fn bytes(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(0x100000001b3);
        }
    }
}

impl std::fmt::Write for Fnv {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        self.bytes(text.as_bytes());
        Ok(())
    }
}

fn debug_hash(value: &impl std::fmt::Debug) -> u64 {
    let mut fnv = Fnv::new();
    let _ = write!(fnv, "{value:?}");
    fnv.0
}

fn map_hash(map: &TextureMap) -> u64 {
    let mut fnv = Fnv::new();
    fnv.bytes(&map.albedo);
    fnv.bytes(&map.normal);
    fnv.bytes(&map.roughness);
    fnv.bytes(map.emissive.as_deref().unwrap_or_default());
    let _ = write!(fnv, "{} {} {}", map.width, map.height, map.mip_level_count);
    fnv.0
}

/// Every field of a built avatar, each a row, so that a difference says where it is.
fn whole(out: &mut String, label: &str, a: &Avatar) {
    let _ = writeln!(out, "{label} rig-all {:x}", debug_hash(&a.rig));
    let _ = writeln!(out, "{label} meshes {:x}", debug_hash(&a.meshes));
    let _ = writeln!(out, "{label} skin-maps {:x}", map_hash(&a.skin));
    let cloth = a.cloth.as_ref().map(map_hash);
    let _ = writeln!(out, "{label} cloth-maps {cloth:x?}");
    let _ = writeln!(out, "{label} budget {:?}", a.budget);
    let _ = writeln!(out, "{label} far-hair {:x}", debug_hash(&a.far_hair));
    let _ = writeln!(out, "{label} parts {:x}", debug_hash(&a.parts));
}

fn main() {
    let path = std::env::args().nth(1).expect("out");
    let mut out = String::new();
    for seed in 0..16i64 {
        let mut person = AvatarRecord::new("P", Archetype::default());
        person.reroll(seed);
        let _ = writeln!(out, "person {seed} code {}", person.share_code());
        let _ = writeln!(
            out,
            "person {seed} json {}",
            serde_json::to_string(&person).unwrap()
        );
        let a = Avatar::build(&person).expect("builds");
        whole(&mut out, &format!("person {seed}"), &a);
        let _ = writeln!(out, "rig {:?}", a.rig.joints);
        let _ = writeln!(
            out,
            "body {:?} {:?}",
            a.parts.body.positions, a.parts.body.faces
        );
        let _ = writeln!(out, "fur {:?}", a.parts.fur);
        for m in &a.meshes {
            let _ = writeln!(
                out,
                "mesh {:?} {:?} {:?} {:?} {:?} {:?} {:?}",
                m.kind,
                m.mesh.positions,
                m.mesh.faces,
                m.mesh.uvs,
                m.mesh.normals,
                m.mesh.colours,
                m.mesh.skin
            );
        }
        let _ = writeln!(out, "skin {:?}", a.skin.albedo.len());
        let mut h: u64 = 0xcbf29ce484222325;
        for b in a.skin.albedo.iter() {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        let _ = writeln!(out, "albedo {h:x}");
        let mut beast = AvatarRecord::new("B", Archetype::Quadruped(QuadrupedParams::default()));
        beast.reroll(seed);
        for stiff in [
            None,
            Some(0.0f32),
            Some(0.2),
            Some(0.4),
            Some(0.73),
            Some(1.0),
        ] {
            let mut r = beast.clone();
            if let (Some(stiff), Archetype::Quadruped(p)) = (stiff, &mut r.archetype) {
                p.fur.mane_stiffness = stiff;
            }
            let _ = writeln!(out, "beast {seed} {stiff:?} code {}", r.share_code());
            let _ = writeln!(
                out,
                "beast {seed} {stiff:?} json {}",
                serde_json::to_string(&r).unwrap()
            );
            let _ = writeln!(out, "beast {seed} {stiff:?} stamp {:?}", r.generator);
            let b = Avatar::build(&r).expect("builds");
            whole(&mut out, &format!("beast {seed} {stiff:?}"), &b);
            let _ = writeln!(out, "beast {seed} {stiff:?} rig {:?}", b.rig.joints);
            let _ = writeln!(
                out,
                "beast {seed} {stiff:?} body {:?}",
                b.parts.body.positions
            );
        }
        // Every zone but the mane, and a limp mane: geometry that must not move.
        let mut limp = beast.clone();
        if let Archetype::Quadruped(p) = &mut limp.archetype {
            p.fur.mane = 0.0;
            p.fur.ruff = 0.7;
            p.fur.tail = 0.7;
            p.fur.feathering = 0.7;
            p.fur.tufts = 0.7;
        }
        let built = Avatar::build(&limp).expect("builds");
        let drawn = built.meshes.iter().find(|m| m.kind == MeshKind::Hair);
        let _ = writeln!(
            out,
            "nomane {seed} hair {:?}",
            drawn.map(|m| (
                &m.mesh.positions,
                &m.mesh.faces,
                &m.mesh.uvs,
                &m.mesh.normals,
                &m.mesh.colours,
                &m.mesh.skin
            ))
        );
        let f = built.parts.fur;
        let _ = writeln!(
            out,
            "nomane {seed} {:?}",
            f.map(|f| (f.mesh.positions, f.mesh.uvs, f.mesh.normals, f.mesh.faces))
        );
        let mut zero = beast.clone();
        if let Archetype::Quadruped(p) = &mut zero.archetype {
            p.fur.mane = 0.8;
            p.fur.mane_stiffness = 0.0;
        }
        let f = Avatar::build(&zero).expect("builds").parts.fur;
        let _ = writeln!(
            out,
            "limp {seed} {:?}",
            f.map(|f| (f.mesh.positions, f.mesh.uvs, f.mesh.normals, f.mesh.faces))
        );
    }
    std::fs::write(path, out).unwrap();
}
