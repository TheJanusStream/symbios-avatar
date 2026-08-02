//! From a record to something a renderer can draw.
//!
//! Every other module here builds one thing well: a cage, a rig, a head of hair,
//! a pair of trousers. [`Avatar::build`] is the recipe that puts them in the
//! right order and hands back geometry, a texture, a skeleton and a bill.
//!
//! It exists because the recipe is long — thirteen calls whose order matters —
//! and a long recipe living in an example is a second implementation of the
//! crate. Two of them already existed here and had drifted apart: one dressed a
//! quadruped in a fringe and a pair of sleeves, because the rule about which
//! bodies wear clothes was written in the other one.
//!
//! ## What comes out
//!
//! Merged, skinned [`AvatarMesh`]es, grouped by the material they need, because
//! the WebGL2 budget is one draw per skinned mesh and a body made of ninety
//! separate locks of hair is ninety draws. Merging costs nothing in fidelity —
//! [`PolyMesh::colours`] carries what used to be a colour per draw.
//!
//! ```rust
//! use symbios_avatar::{Avatar, AvatarRecord};
//!
//! let avatar = Avatar::build(&AvatarRecord::default()).expect("a default body builds");
//! assert!(avatar.budget.tris > 0);
//! assert_eq!(avatar.budget.joints, avatar.rig.len());
//! // Everything drawn is skinned to the rig and mapped into the atlas.
//! for drawn in avatar.drawn(0.0) {
//!     assert!(drawn.mesh.channels_are_consistent());
//!     assert_eq!(drawn.mesh.skin.len(), drawn.mesh.vertex_count());
//! }
//! ```
//!
//! ## What is not done yet
//!
//! Attached parts — hands, feet, brows, lids — are generated with honest texture
//! coordinates of their own but have no region of the skin atlas to put them in,
//! because nothing rasterises or paints one. Until that exists they are mapped
//! to a single texel at the middle of the chart covering the body part they
//! attach to, so a hand takes the complexion of the arm it grows from. That is a
//! degenerate chart, not a missing one: when regions exist, the same call places
//! the same coordinates in them.

use glam::{Mat4, Vec2, Vec3};
use symbios_texture::generator::TextureMap;

use crate::anim::Pose;
use crate::cage::CageConfig;
use crate::dress::Outfit;
use crate::extremity::Extremities;
use crate::face::{Eyes, Features};
use crate::hair::{Hair, HairParams};
use crate::mesh::PolyMesh;
use crate::plan::{Limb, Zone};
use crate::record::AvatarRecord;
use crate::rig::{Rig, SkinConfig, SkinWeights, Surface, skin};
use crate::texture;
use crate::uv::{UvConfig, UvUnwrap, unwrap};

/// Which material a merged mesh needs.
///
/// The grouping *is* the draw-call budget: one entry per kind, and adding a kind
/// costs every avatar in the scene a draw.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MeshKind {
    /// Skin, in the widest sense: the body and everything made of it.
    Skin,
    /// Hair, which needs its own anisotropic shading.
    Hair,
    /// Cloth.
    Cloth,
    /// Eyeballs, which are the one glossy thing on a body.
    Eye,
}

impl MeshKind {
    /// A short name, for reports.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Skin => "skin",
            Self::Hair => "hair",
            Self::Cloth => "cloth",
            Self::Eye => "eye",
        }
    }
}

/// One merged, skinned, render-ready mesh.
#[derive(Clone, Debug, PartialEq)]
pub struct AvatarMesh {
    /// The material it needs.
    pub kind: MeshKind,
    /// Its geometry, in body space, carrying every channel a renderer wants.
    pub mesh: PolyMesh,
}

/// What one avatar costs.
///
/// Measured rather than asserted. The targets it is judged against — 1 to 3
/// skinned meshes and 15 to 30 thousand triangles on a WebGL2 tier — live in the
/// gate, not here, because a budget that enforces its own limits cannot be used
/// to find out how far over them something is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Budget {
    /// Triangles across every mesh drawn.
    pub tris: usize,
    /// How many draws a renderer needs for one avatar.
    pub meshes: usize,
    /// Joints in the rig, which sets the size of the skinning uniform.
    pub joints: usize,
    /// Bytes of texture the avatar carries.
    pub texture_bytes: usize,
}

/// How a body is built, for the tools that need to vary it.
#[derive(Clone, Debug, PartialEq)]
pub struct AvatarConfig {
    /// How the control cage is swept.
    pub cage: CageConfig,
    /// How many times that cage is subdivided.
    pub subdivisions: usize,
    /// How the mesh is bound to the rig.
    pub skin: SkinConfig,
    /// How the body is unwrapped.
    pub uv: UvConfig,
    /// Side of the square skin atlas, in texels.
    pub atlas: u32,
    /// The plane the body stands on.
    pub ground: f32,
    /// Hair parameters replacing the record's, for walking the axes by eye.
    pub hair: Option<HairParams>,
}

impl Default for AvatarConfig {
    fn default() -> Self {
        Self {
            cage: CageConfig::default(),
            subdivisions: 2,
            skin: SkinConfig::default(),
            uv: UvConfig::default(),
            atlas: 1024,
            ground: 0.0,
            hair: None,
        }
    }
}

/// The structure the merge throws away.
///
/// Merging is for drawing. Everything that wants to *measure* a body — frame a
/// close-up on its head, check a hem, ask how thick an arm came out — needs the
/// parts as parts, so they are kept.
#[derive(Clone, Debug, PartialEq)]
pub struct Parts {
    /// The body's surface, closed and un-split, before it was charted.
    pub body: PolyMesh,
    /// What holds each body vertex.
    pub weights: SkinWeights,
    /// Which part of the body each body vertex belongs to.
    pub zones: Vec<Zone>,
    /// How the body was charted into the atlas.
    pub unwrap: UvUnwrap,
    /// The body as measured, which is what every attached part is fitted to.
    pub surface: Surface,
    /// Its eyes, if it has a head.
    pub eyes: Option<Eyes>,
    /// Its face, if it is the kind of body that wears one.
    pub features: Option<Features>,
    /// Its hair, likewise.
    pub hair: Option<Hair>,
    /// Its hands and feet.
    pub extremities: Extremities,
    /// What it is wearing.
    pub outfit: Outfit,
    /// Whether this body carries limbs it does not stand on.
    ///
    /// See [`Avatar::build`] for why that is the question asked.
    pub handed: bool,
}

/// A record, built.
///
/// Deliberately not `Clone`, `Debug` or `PartialEq`: it owns several megabytes
/// of texture, and none of those three operations is one anybody should get by
/// accident on something that size. Compare [`Avatar::meshes`] and
/// [`Avatar::budget`], which are the parts worth comparing.
pub struct Avatar {
    /// The posable hierarchy everything is skinned to.
    pub rig: Rig,
    /// Merged geometry that does not change with expression.
    ///
    /// The eyes are not here: nothing rigs a lid yet, so a blink is geometry
    /// rather than a pose. [`Avatar::drawn`] returns the whole list.
    pub meshes: Vec<AvatarMesh>,
    /// The painted skin atlas.
    pub skin: TextureMap,
    /// What it all costs.
    pub budget: Budget,
    /// The parts the merge was made from.
    pub parts: Parts,
}

impl Avatar {
    /// Builds an avatar from a record, with the default settings.
    ///
    /// Returns `None` if the record describes a body that cannot be meshed or
    /// rigged — a plan whose limbs overlap at a joint, most often.
    #[must_use]
    pub fn build(record: &AvatarRecord) -> Option<Self> {
        Self::build_with(record, &AvatarConfig::default())
    }

    /// Builds an avatar from a record.
    ///
    /// The order is the whole point of this function. The body is shaped before
    /// anything is bound, charted or fitted to it, because everything after that
    /// measures the surface rather than the plan; and the parts are grown before
    /// they are merged, because merging is what loses the ability to ask a part
    /// a question.
    ///
    /// **Which bodies wear a human face, human hair and human clothes** is
    /// decided here, once, rather than by whoever is drawing. The test is not
    /// what the record calls itself: it is whether the body has a limb it does
    /// not stand on. A quadruped carries its weight on all four, so it has no
    /// hands, and a fringe hanging off a muzzle over a pair of sleeved front legs
    /// is a costume rather than a creature — creatures get fur, a muzzle and a
    /// harness of their own, which is WS6. Asking it this way also gets a body
    /// nobody has planned yet right: something with four legs and two arms is
    /// dressed, and a six-legged walker is not.
    ///
    /// # Errors
    ///
    /// Returns `None` rather than an error type because every failure here is
    /// the same one — this record does not describe a body — and the plan that
    /// produced it is the thing to look at.
    #[must_use]
    pub fn build_with(record: &AvatarRecord, config: &AvatarConfig) -> Option<Self> {
        let skeleton = record.skeleton();
        let body = crate::build_body(&skeleton, &config.cage, config.subdivisions).ok()?;
        let rig = Rig::from_skeleton(&skeleton).ok()?;

        let weights = skin::bind(&body, &rig, &config.skin);
        let zones = weights.zone_map(&body, &rig);
        let charts = unwrap(&body, &rig, &zones, &config.uv);
        let geometry = texture::bake_geometry(&body, &charts, config.atlas);
        let painted = texture::paint_skin(&geometry, &rig, &record.skin);
        let surface = Surface::measure(&body, &rig);

        // A limb the body does not stand on is a hand, and a body with hands is
        // the kind that wears things. See the note above.
        let handed = rig.ground_contacts().len() < Limb::ALL.len();
        let hair_params = config.hair.unwrap_or(record.hair);

        // Cut from the body, so charted where the body is charted.
        let mut outfit = if handed {
            Outfit::wear(&body, &weights, &zones, &record.outfit)
        } else {
            Outfit::default()
        };
        let body_uvs = charts.by_source(body.vertex_count());
        for garment in &mut outfit.garments {
            garment.chart(&body_uvs);
        }

        let eyes = Eyes::build(&rig, &record.eyes);
        let parts = Parts {
            features: handed
                .then(|| {
                    eyes.as_ref()
                        .map(|eyes| Features::build(eyes, &record.face))
                })
                .flatten(),
            hair: handed
                .then(|| Hair::build(&body, &rig, &hair_params))
                .flatten(),
            extremities: Extremities::build(&rig, &surface, config.ground),
            outfit,
            eyes,
            handed,
            weights,
            zones,
            surface,
            unwrap: charts,
            body,
        };

        let mut avatar = Self {
            meshes: Vec::new(),
            budget: Budget::default(),
            skin: painted,
            rig,
            parts,
        };
        avatar.meshes = avatar.merge();
        avatar.budget = avatar.measure();
        Some(avatar)
    }

    /// Everything to draw, with the eyes at the given closure.
    ///
    /// `closure` runs `0` for open to `1` for shut.
    #[must_use]
    pub fn drawn(&self, closure: f32) -> Vec<AvatarMesh> {
        let mut drawn = self.meshes.clone();
        drawn.extend(self.eyes_at(closure));
        drawn
    }

    /// The eyes, at a given closure.
    ///
    /// Rebuilt rather than posed: a lid swings about the eye's pivot and no
    /// joint drives it, so until the face has a rig of its own (#35) a blink is
    /// a change of geometry. Globes and lids come back separately because they
    /// are made of different stuff — drawn in one colour, a shut eye is
    /// invisible, which is how a working blink first looked broken.
    #[must_use]
    pub fn eyes_at(&self, closure: f32) -> Vec<AvatarMesh> {
        let Some(eyes) = &self.parts.eyes else {
            return Vec::new();
        };
        let to_body = Mat4::from_translation(self.rig.joints[eyes.head].position);
        let joint = eyes.head as u16;

        let mut globes = PolyMesh::new();
        let mut lids = PolyMesh::new();
        for eye in [&eyes.left, &eyes.right] {
            let mut globe = eye.globe.clone();
            globe.set_colours(
                globe
                    .positions
                    .iter()
                    .map(|&point| iris_of(point, eye.pivot))
                    .collect(),
            );
            globes.append(&globe);
            lids.append(&eye.upper_lid.transformed(eye.lid_transform(closure, true)));
            lids.append(&eye.lower_lid.transformed(eye.lid_transform(closure, false)));
        }

        let mut built = Vec::with_capacity(2);
        for (kind, mut mesh) in [(MeshKind::Eye, globes), (MeshKind::Skin, lids)] {
            if mesh.faces.is_empty() {
                continue;
            }
            if kind == MeshKind::Skin {
                // Lids take the complexion of the face they sit in.
                mesh = self.charted(&mesh, Zone::Head);
                mesh.paint(Vec3::ONE);
            }
            mesh.set_normals(mesh.vertex_normals());
            let mut placed = mesh.transformed(to_body);
            placed.bind_rigidly(joint);
            built.push(AvatarMesh {
                kind,
                mesh: placed.split_uv_seams(),
            });
        }
        built
    }

    /// The body posed, with every mesh deformed to match.
    ///
    /// The one call an integration needs per frame once the geometry is up.
    #[must_use]
    pub fn posed(&self, pose: &Pose, closure: f32) -> Vec<AvatarMesh> {
        let posed = pose.forward(&self.rig);
        self.drawn(closure)
            .into_iter()
            .map(|drawn| AvatarMesh {
                kind: drawn.kind,
                mesh: posed.deform_mesh(&self.rig, &drawn.mesh),
            })
            .collect()
    }

    /// Merges every static part into one mesh per material.
    fn merge(&self) -> Vec<AvatarMesh> {
        let mut skin = self.charted_body();
        for part in self
            .parts
            .extremities
            .hands
            .iter()
            .chain(&self.parts.extremities.feet)
        {
            let mut mesh = self.charted(&part.mesh, Zone::Extremity(part.limb));
            mesh.set_normals(mesh.vertex_normals());
            mesh.paint(Vec3::ONE);
            let mut placed =
                mesh.transformed(Mat4::from_translation(self.rig.joints[part.joint].position));
            placed.bind_rigidly(part.joint as u16);
            skin.append(&placed.split_uv_seams());
        }
        if let Some(features) = &self.parts.features {
            let mut mesh = self.charted(&features.assembled(), Zone::Head);
            mesh.set_normals(mesh.vertex_normals());
            mesh.paint(Vec3::ONE);
            let mut placed = mesh.transformed(Mat4::from_translation(
                self.rig.joints[features.head].position,
            ));
            placed.bind_rigidly(features.head as u16);
            skin.append(&placed.split_uv_seams());
        }

        let mut merged = vec![AvatarMesh {
            kind: MeshKind::Skin,
            mesh: skin,
        }];

        if let Some(hair) = &self.parts.hair {
            let to_body = Mat4::from_translation(self.rig.joints[hair.head].position);
            let tone = Vec3::from_array(hair.colour);
            let mut locks = PolyMesh::new();
            for (index, group) in hair.groups.iter().enumerate() {
                let mut lock = group.mesh();
                lock.set_normals(lock.vertex_normals());
                // A walk over brightness, lock by lock. As one solid in one
                // colour a head of hair reads as a helmet at close range, and
                // carrying the walk per vertex is what lets it be one draw.
                let step = (index as f32 * 0.618_034).fract();
                lock.paint(tone * (0.74 + 0.5 * step));
                locks.append(&lock.split_uv_seams());
            }
            let mut placed = locks.transformed(to_body);
            placed.bind_rigidly(hair.head as u16);
            merged.push(AvatarMesh {
                kind: MeshKind::Hair,
                mesh: placed,
            });
        }

        if !self.parts.outfit.is_empty() {
            let mut cloth = PolyMesh::new();
            for garment in &self.parts.outfit.garments {
                let mut worn = garment.mesh.clone();
                worn.set_normals(worn.vertex_normals());
                cloth.append(&worn);
            }
            merged.push(AvatarMesh {
                kind: MeshKind::Cloth,
                mesh: cloth,
            });
        }

        merged
    }

    /// The body, unwrapped into the atlas and ready to draw.
    fn charted_body(&self) -> PolyMesh {
        let charts = &self.parts.unwrap;
        // Normals over the body's own topology, then gathered through the
        // unwrap. Derived from the unwrapped copy they would split every seam.
        let normals = self.parts.body.vertex_normals();
        let mut mesh = PolyMesh {
            positions: charts.gather(&self.parts.body.positions),
            faces: charts.faces.clone(),
            ..Default::default()
        };
        mesh.set_uvs(charts.uvs.clone());
        mesh.set_normals(charts.gather(&normals));
        mesh.set_skin(charts.gather(&self.parts.weights.vertices));
        mesh.paint(Vec3::ONE);
        mesh
    }

    /// Maps a part's own coordinates into the atlas region covering `zone`.
    ///
    /// The region is a point rather than a rectangle, because nothing paints
    /// attached parts into the atlas yet — see the note in the module
    /// documentation. Everything else about the call is already final.
    fn charted(&self, mesh: &PolyMesh, zone: Zone) -> PolyMesh {
        let middle = self
            .parts
            .unwrap
            .charts
            .iter()
            .find(|chart| chart.zone == zone)
            .map(|chart| chart.rect.lerp(Vec2::splat(0.5)))
            .unwrap_or(Vec2::splat(0.5));
        mesh.uvs_within(middle, Vec2::ZERO)
    }

    /// Counts what the built avatar costs.
    fn measure(&self) -> Budget {
        let drawn = self.drawn(0.0);
        Budget {
            tris: drawn
                .iter()
                .map(|mesh| mesh.mesh.triangulated().len())
                .sum(),
            meshes: drawn.len(),
            joints: self.rig.len(),
            texture_bytes: self.skin.albedo.len()
                + self.skin.normal.len()
                + self.skin.roughness.len()
                + self.skin.emissive.as_ref().map_or(0, Vec::len),
        }
    }
}

/// A pale globe with a dark iris facing forward, so an eye reads as an eye.
///
/// Baked per vertex rather than evaluated per pixel: it is a property of the
/// geometry, and carrying it as colour is what lets both eyes be one draw.
fn iris_of(point: Vec3, pivot: Vec3) -> Vec3 {
    let forward = (point - pivot).normalize_or(Vec3::Z).z;
    if forward > 0.78 {
        Vec3::new(0.05, 0.06, 0.08)
    } else if forward > 0.50 {
        Vec3::new(0.24, 0.38, 0.46)
    } else {
        Vec3::new(0.93, 0.92, 0.90)
    }
}

/// A default humanoid, for tools with nothing better to show.
#[must_use]
pub fn demo() -> Option<Avatar> {
    Avatar::build(&AvatarRecord::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{Archetype, QuadrupedParams};

    fn biped(seed: i64) -> Avatar {
        let mut record = AvatarRecord::new("Built", Archetype::default());
        record.reroll(seed);
        Avatar::build(&record).expect("a biped builds")
    }

    fn quadruped() -> Avatar {
        let record = AvatarRecord::new("Beast", Archetype::Quadruped(QuadrupedParams::default()));
        Avatar::build(&record).expect("a quadruped builds")
    }

    #[test]
    fn every_drawn_mesh_is_skinned_mapped_and_consistent() {
        // The whole point of the type: what comes out is drawable without the
        // caller having to assemble anything.
        for avatar in [biped(1), quadruped()] {
            let drawn = avatar.drawn(0.0);
            assert!(!drawn.is_empty());
            for part in &drawn {
                let mesh = &part.mesh;
                assert!(mesh.channels_are_consistent(), "{:?}", part.kind);
                assert_eq!(mesh.skin.len(), mesh.vertex_count(), "{:?}", part.kind);
                assert_eq!(mesh.uvs.len(), mesh.vertex_count(), "{:?}", part.kind);
                assert_eq!(mesh.normals.len(), mesh.vertex_count(), "{:?}", part.kind);
                assert_eq!(mesh.colours.len(), mesh.vertex_count(), "{:?}", part.kind);
                for influences in &mesh.skin {
                    assert!(
                        influences
                            .iter()
                            .all(|i| (i.joint as usize) < avatar.rig.len()),
                        "{:?} names a joint that does not exist",
                        part.kind
                    );
                    let total: f32 = influences.iter().map(|i| i.weight).sum();
                    assert!(
                        (total - 1.0).abs() < 1e-3,
                        "{:?} is unnormalised",
                        part.kind
                    );
                }
            }
        }
    }

    #[test]
    fn a_quadruped_is_bare_and_a_biped_is_dressed() {
        // The gate that had drifted between two examples, stated once. A body
        // wears things when it has a limb it does not stand on.
        let beast = quadruped();
        assert!(!beast.parts.handed);
        assert!(beast.parts.hair.is_none(), "a quadruped grew a fringe");
        assert!(beast.parts.features.is_none());
        assert!(beast.parts.outfit.is_empty(), "a quadruped got dressed");
        assert!(
            beast.parts.extremities.hands.is_empty(),
            "a quadruped grew hands"
        );
        assert!(
            !beast.meshes.iter().any(|m| m.kind != MeshKind::Skin),
            "a quadruped should be skin and eyes only"
        );

        let person = biped(1);
        assert!(person.parts.handed);
        assert!(person.parts.hair.is_some());
        assert!(person.parts.features.is_some());
        assert_eq!(person.parts.outfit.len(), 2);
    }

    #[test]
    fn materials_are_merged_to_one_mesh_each() {
        let avatar = biped(7);
        let mut kinds: Vec<MeshKind> = avatar.drawn(0.0).iter().map(|m| m.kind).collect();
        let all = kinds.len();
        kinds.sort_unstable();
        kinds.dedup();
        // Skin appears twice — the body and the lids — because a lid moves and
        // nothing rigs it yet. Everything else is exactly one draw.
        assert_eq!(all - kinds.len(), 1, "{kinds:?} out of {all} draws");
        assert!(kinds.contains(&MeshKind::Hair));
        assert!(kinds.contains(&MeshKind::Cloth));
        assert!(kinds.contains(&MeshKind::Eye));
    }

    #[test]
    fn hair_keeps_a_shade_per_lock_through_the_merge() {
        // Merging is only free if nothing is lost by it. One solid in one colour
        // reads as a helmet, so the walk over brightness has to survive.
        let avatar = biped(3);
        let hair = avatar
            .drawn(0.0)
            .into_iter()
            .find(|m| m.kind == MeshKind::Hair)
            .expect("a biped has hair");
        let mut tones: Vec<[u32; 3]> = hair
            .mesh
            .colours
            .iter()
            .map(|c| {
                [
                    (c.x * 4096.0) as u32,
                    (c.y * 4096.0) as u32,
                    (c.z * 4096.0) as u32,
                ]
            })
            .collect();
        tones.sort_unstable();
        tones.dedup();
        assert!(
            tones.len() > 8,
            "{} locks came out in {} shades",
            avatar.parts.hair.map_or(0, |h| h.groups.len()),
            tones.len()
        );
    }

    #[test]
    fn the_budget_counts_what_is_actually_drawn() {
        let avatar = biped(11);
        let drawn = avatar.drawn(0.0);
        assert_eq!(avatar.budget.meshes, drawn.len());
        assert_eq!(
            avatar.budget.tris,
            drawn
                .iter()
                .map(|m| m.mesh.triangulated().len())
                .sum::<usize>()
        );
        assert_eq!(avatar.budget.joints, avatar.rig.len());
        // Three RGBA8 maps of the configured size.
        let side = AvatarConfig::default().atlas as usize;
        assert_eq!(avatar.budget.texture_bytes, side * side * 4 * 3);
    }

    #[test]
    fn a_blink_moves_the_lids_and_leaves_the_globes_alone() {
        let avatar = biped(5);
        let eyes = |closure: f32| {
            avatar
                .eyes_at(closure)
                .into_iter()
                .map(|m| (m.kind, m.mesh))
                .collect::<Vec<_>>()
        };
        let open = eyes(0.0);
        let shut = eyes(1.0);
        assert_eq!(open.len(), 2);
        assert_eq!(open[0].0, MeshKind::Eye);
        assert_eq!(open[0].1, shut[0].1, "the globes moved during a blink");
        assert_ne!(open[1].1, shut[1].1, "the lids did not move");
    }

    #[test]
    fn posing_moves_every_mesh_together() {
        use glam::Quat;

        let avatar = biped(13);
        let mut pose = Pose::rest(&avatar.rig);
        pose.translation = Vec3::new(0.0, 0.5, 0.0);
        for rotation in &mut pose.rotations {
            *rotation = Quat::IDENTITY;
        }

        for (rest, moved) in avatar.drawn(0.0).iter().zip(avatar.posed(&pose, 0.0)) {
            assert_eq!(rest.kind, moved.kind);
            for (before, after) in rest.mesh.positions.iter().zip(&moved.mesh.positions) {
                assert!(
                    (*after - *before - Vec3::new(0.0, 0.5, 0.0)).length() < 1e-3,
                    "{:?} did not travel with the body",
                    rest.kind
                );
            }
        }
    }

    #[test]
    fn a_hair_override_replaces_the_record_and_nothing_else() {
        let record = AvatarRecord::new("Built", Archetype::default());
        let cropped = Avatar::build_with(
            &record,
            &AvatarConfig {
                hair: Some(HairParams {
                    length: 0.0,
                    ..HairParams::default()
                }),
                ..Default::default()
            },
        )
        .expect("builds");
        let long = Avatar::build_with(
            &record,
            &AvatarConfig {
                hair: Some(HairParams {
                    length: 1.0,
                    ..HairParams::default()
                }),
                ..Default::default()
            },
        )
        .expect("builds");
        assert!(
            long.parts.hair.expect("hair").drop() > cropped.parts.hair.expect("hair").drop() * 2.0
        );
    }

    #[test]
    fn building_is_reproducible() {
        let mut record = AvatarRecord::new("Built", Archetype::default());
        record.reroll(23);
        let once = Avatar::build(&record).expect("builds");
        let twice = Avatar::build(&record).expect("builds");
        assert_eq!(once.meshes, twice.meshes);
        assert_eq!(once.budget, twice.budget);
    }
}
