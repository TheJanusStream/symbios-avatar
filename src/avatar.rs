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
//! ## One atlas, whole avatar
//!
//! Attached parts — a nose, an ear, a hand — are not part of the body mesh and
//! so are not part of its unwrap. They are nonetheless most of what a face is
//! judged on, so they are given regions of the **same** atlas: their sizes are
//! requested before the body is unwrapped, packed alongside its charts, and
//! rasterised into the same geometry buffer. By the time the painter runs there
//! is no difference between a nose and the cheek beside it, which is what lets
//! one complexion cover an avatar.
//!
//! One thing is still degenerate: eyelids. They do not exist when the atlas is
//! packed, because a blink is geometry rather than a pose until the face has a
//! rig, so there is nothing to reserve a region for. A lid samples one texel of
//! the face's chart, which at a lid's size is all it needs.

use glam::{Mat4, Vec2, Vec3};
use symbios_texture::generator::TextureMap;

use crate::anim::Pose;
use crate::cage::CageConfig;
use crate::dress::Outfit;
use crate::extremity::Extremities;
use crate::face::{self, Canon, Eyes, Features, Skull};
use crate::hair::{Hair, HairParams};
use crate::mesh::PolyMesh;
use crate::plan::{Limb, Zone};
use crate::record::AvatarRecord;
use crate::rig::{Rig, SkinConfig, SkinWeights, Surface, skin};
use crate::texture;
use crate::texture::SkinParams;
use crate::uv::{Rect, UvConfig, UvUnwrap, unwrap_with};

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
    /// Complexion replacing the record's, for the same reason. Named apart from
    /// `skin` above, which is how the mesh is bound rather than what colour it
    /// is. A complexion is judged by looking at it under light, not by reading a
    /// triple of numbers, and a body is the only place to do that.
    pub complexion: Option<SkinParams>,
}

impl Default for AvatarConfig {
    fn default() -> Self {
        Self {
            cage: CageConfig::default(),
            subdivisions: crate::BODY_SUBDIVISIONS,
            skin: SkinConfig::default(),
            uv: UvConfig::default(),
            atlas: 1024,
            ground: 0.0,
            hair: None,
            complexion: None,
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
    /// The openable mouth's seam and pocket vertices, if one was cut (#154).
    pub mouth: Option<face::Mouth>,
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
        // The head reads the frame axis from here on (#166). Derived at the
        // top of the build rather than inside `build_body`, because a caller
        // holding a skeleton and no record — every test and probe in this crate
        // — is entitled to the neutral head without inventing composites.
        let dimorphism = face::Dimorphism::of(&record.composites);
        // The record's face axes are offsets on what the frame axis already
        // derives, so everything downstream reads this and not `record.face` —
        // the carve, the mouth cut and the built features all have to agree
        // about one mouth. See `FaceParams::on`.
        let face_params = record.face.on(&dimorphism);
        let mut body =
            crate::build_body(&skeleton, &config.cage, config.subdivisions, &dimorphism).ok()?;
        let mut rig = Rig::from_skeleton(&skeleton).ok()?;

        // The face is carved into the body's own surface, so it has to happen
        // before ANY of what follows: skin weights, texture charts, the garment
        // cut and every attached part are all fitted to the mesh in hand, and a
        // nose that appears afterwards is a nose none of them knows about (#59).
        //
        // **Measure, carve, then seat**, and that order is the fix for the eyes
        // (#76). It used to be place-then-carve, because the carve read the eye
        // line, the pupil spacing and the proportion unit out of a built `Eyes`
        // — so an eye could not be placed against the face it belonged in
        // without a cycle. None of those three needs a globe: they are the
        // measured skull's, and `Canon` is where they live now. Which leaves the
        // eye free to be seated last, against the orbit-carved surface it will
        // actually be seen against. `half_width` at the eye line and `chin` are
        // bit-identical before and after the carve on every body measured, so
        // reading the canon from the uncarved head costs nothing.
        let canon =
            Skull::measure(&body, &rig).map(|skull| Canon::measure(&rig, &skull, &record.eyes));
        if let Some(canon) = &canon {
            face::carve_face(&mut body, &rig, canon, &face_params);
        }
        // The mouth is cut AFTER the carve — the parting follows the carved
        // groove — and before anything measures, binds or charts the surface,
        // so the pocket is simply more body to all of them (#154). A rig with
        // no jaw markers gets no cut and no change.
        let mouth = canon
            .as_ref()
            .and_then(|canon| face::mouth::open(&mut body, &rig, canon, &face_params));
        let body = body;
        let eyes = canon
            .as_ref()
            .map(|canon| Eyes::build(&rig, &body, canon, &record.eyes));

        let mut weights = skin::bind(&body, &rig, &config.skin);
        // The seam's own vertices leave the field's blend: the parting's two
        // edges are coincident at rest, so the field hands them identical
        // weights and they would never part. The upper edge and the pocket's
        // roof are the skull's outright; the lower edge and the floor are the
        // jaw's; the welds keep their blend, which is what lets a mouth corner
        // stretch (#154).
        if let Some(mouth) = &mouth {
            let jaw = (0..rig.len()).find_map(|tip| {
                let pivot = rig.joints[tip].parent?;
                (rig.joints[tip].marker && rig.joints[pivot].marker).then_some(pivot)
            });
            let head = rig.in_zone(Zone::Head).first().copied();
            if let (Some(pivot), Some(head)) = (jaw, head) {
                let solely = |joint: usize| {
                    let mut held = [crate::rig::Influence::default(); crate::rig::MAX_INFLUENCES];
                    held[0] = crate::rig::Influence {
                        joint: joint as u16,
                        weight: 1.0,
                    };
                    held
                };
                for &vertex in mouth
                    .upper
                    .iter()
                    .chain(&mouth.roof)
                    .chain(&mouth.teeth)
                    .chain(&mouth.overlip)
                {
                    weights.vertices[vertex as usize] = solely(head);
                }
                for &vertex in mouth.lower.iter().chain(&mouth.floor).chain(&mouth.lip) {
                    weights.vertices[vertex as usize] = solely(pivot);
                }
            }
        }
        let weights = weights;
        let zones = weights.zone_map(&body, &rig);
        let surface = Surface::measure(&body, &rig);

        // A limb the body does not stand on is a hand, and a body with hands is
        // the kind that wears things. See the note above.
        let handed = rig.ground_contacts().len() < Limb::ALL.len();
        let hair_params = config.hair.unwrap_or(record.hair);

        // Measured from the body that was built, not from the plan that asked
        // for it: the two differ by about a third at the head, and by a
        // different third on every body. Measured again here rather than reused
        // from above, because an ear is conformed to the surface it sits on and
        // that surface now has a face carved into it.
        let skull = Skull::measure(&body, &rig);
        let mut features = handed
            .then(|| {
                let canon = canon.as_ref()?;
                let skull = skull.as_ref()?;
                Some(Features::build(canon, skull, &face_params))
            })
            .flatten();
        // `&mut rig`: a hand brings its own bones, and they have to
        // land on the rig this avatar is going to carry. The body's own binding
        // is already done above and is unaffected — digit joints are
        // `Role::Digit`, which `skin::bind` and `nearest_bone` both skip.
        let mut extremities = Extremities::build(&mut rig, &surface, config.ground);

        // The attached parts exist BEFORE the body is unwrapped, so the packer
        // can reserve them regions of the same atlas rather than being asked
        // afterwards for whatever the body did not want. The parts are the half
        // that needs the texels — a face is judged on its nose, its mouth and
        // its ears, and none of those is part of the body mesh.
        let mut wanted: Vec<Vec2> = attached_meshes(&features, &extremities)
            .map(|(mesh, zone)| chart_request(mesh, zone, &config.uv))
            .collect();
        // One more region when there is a mouth: its pocket cannot share the
        // face's chart — a projection that flattens a face cannot also
        // flatten a fold hidden behind it — so the interior is packed like an
        // attached part and re-charted below (#155).
        if mouth.is_some() {
            wanted.push(Vec2::new(0.06, 0.04));
        }
        let (mut charts, mut reserved) = unwrap_with(&body, &rig, &zones, &config.uv, &wanted);
        if let Some(mouth) = &mouth
            && let Some(rect) = reserved.pop()
        {
            face::mouth::chart_interior(&mut charts, mouth, &body, rect);
        }
        let charts = charts;
        place_charts(&mut features, &mut extremities, &reserved);

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

        // Baked in body space, which is where a painter wants them: a nose is
        // then painted by the same complexion arithmetic as the cheek beside it.
        let placed = attached_in_body(&rig, &features, &extremities);
        let borrowed: Vec<(&PolyMesh, Zone)> =
            placed.iter().map(|(mesh, zone)| (mesh, *zone)).collect();
        // The mouth's interior, as a per-vertex scalar the painter can read:
        // nothing derivable from a position can tell the inside of a closed
        // fold from the lip a millimetre outside it, so the surgery's own
        // classes are the channel (#154).
        let inside = {
            let mut inside = vec![0.0f32; body.vertex_count()];
            if let Some(mouth) = &mouth {
                for &vertex in mouth.roof.iter().chain(&mouth.floor) {
                    inside[vertex as usize] = 1.0;
                }
                for &vertex in &mouth.teeth {
                    inside[vertex as usize] = 0.5;
                }
            }
            inside
        };
        let geometry = texture::bake(&body, &charts, &borrowed, &inside, config.atlas);
        let painted =
            texture::paint_skin(&geometry, &rig, &config.complexion.unwrap_or(record.skin));

        let parts = Parts {
            hair: handed
                .then(|| Hair::build(&body, &rig, &hair_params))
                .flatten(),
            features,
            extremities,
            outfit,
            eyes,
            handed,
            weights,
            zones,
            surface,
            unwrap: charts,
            body,
            mouth,
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
            // Baked per vertex rather than evaluated per pixel: an iris is a
            // property of the geometry here, and carrying it as colour is what
            // lets both eyes be one draw. The globe's latitude rings are placed
            // to straddle the angles this thresholds at, so the boundaries land
            // between adjacent rings rather than across them (#81).
            let mut globe = eye.globe.clone();
            globe.set_colours(
                globe
                    .positions
                    .iter()
                    .map(|&point| face::eye::iris_of(point - eye.pivot))
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
        for part in self.parts.extremities.all() {
            let mut mesh = part.mesh.clone();
            mesh.set_normals(mesh.vertex_normals());
            mesh.paint(Vec3::ONE);
            let mut placed =
                mesh.transformed(Mat4::from_translation(self.rig.joints[part.joint].position));
            if placed.skin.is_empty() {
                // A foot rides its ankle whole. A hand does not: it arrives
                // already skinned to its own twenty-one bones, and binding it
                // rigidly here would throw them away and glue every finger shut
                // (#113).
                placed.bind_rigidly(part.joint as u16);
            }
            skin.append(&placed.split_uv_seams());
        }
        if let Some(features) = &self.parts.features {
            let mut mesh = features.assembled();
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
            // The sculpted mass first, then the locks that break its edge.
            let mut locks = hair.shell.painted(tone);
            locks.set_normals(locks.vertex_normals());
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

    /// Maps a part's own coordinates onto one texel of the chart covering `zone`.
    ///
    /// A degenerate region, and deliberately the last one left. Eyelids are the
    /// only attached geometry that does not exist at build time — they are
    /// rebuilt per blink, because nothing rigs a lid yet (#35) — so there is
    /// nothing to reserve a region *for* when the atlas is packed. A lid takes
    /// the complexion of the face it sits in, which at a lid's size is all it
    /// needs; when the face has a rig, a lid becomes ordinary geometry and gets
    /// an ordinary chart.
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

/// Every attached part's mesh and the zone it belongs to, in one fixed order.
///
/// One walk, used three times — to size the regions, to hand them out, and to
/// bake them. Two hand-written lists in the same order would agree until the
/// first time somebody added a feature to one of them.
fn attached_meshes<'a>(
    features: &'a Option<Features>,
    extremities: &'a Extremities,
) -> impl Iterator<Item = (&'a PolyMesh, Zone)> {
    features
        .iter()
        .flat_map(|face| face.meshes().map(|mesh| (mesh, Zone::Head)))
        .chain(
            extremities
                .all()
                .map(|part| (&part.mesh, Zone::Extremity(part.limb))),
        )
}

/// How much atlas a part asks for, in the units the body's charts are sized in.
///
/// Its two largest sides, so a long thin ear asks for a long thin region rather
/// than a square one, weighted by the same density the zone earns elsewhere —
/// otherwise a face's features would be charted at body density while the cheek
/// beside them got four times the texels.
fn chart_request(mesh: &PolyMesh, zone: Zone, config: &UvConfig) -> Vec2 {
    let (lo, hi) = mesh.bounds();
    let mut sides = (hi - lo).to_array();
    sides.sort_by(f32::total_cmp);
    let density = match zone {
        Zone::Head => config.head_density,
        Zone::Extremity(_) => config.extremity_density,
        _ => 1.0,
    };
    Vec2::new(sides[2], sides[1]).max(Vec2::splat(1e-4)) * density.sqrt()
}

/// Moves each part's own coordinates into the region reserved for it.
fn place_charts(features: &mut Option<Features>, extremities: &mut Extremities, reserved: &[Rect]) {
    let meshes = features
        .iter_mut()
        .flat_map(|face| face.meshes_mut())
        .chain(extremities.all_mut().map(|part| &mut part.mesh));
    for (mesh, rect) in meshes.zip(reserved) {
        *mesh = mesh.uvs_within(rect.min, rect.size());
    }
}

/// Every attached part, moved into body space so a painter can reach it.
///
/// The parts are built in their joint's local space, which is what lets a
/// renderer parent them; a texel, though, has to say where on the *body* it
/// sits, because that is what the complexion is a function of.
fn attached_in_body(
    rig: &Rig,
    features: &Option<Features>,
    extremities: &Extremities,
) -> Vec<(PolyMesh, Zone)> {
    let mut placed = Vec::new();
    if let Some(face) = features {
        let to_body = Mat4::from_translation(rig.joints[face.head].position);
        for mesh in face.meshes() {
            placed.push((mesh.transformed(to_body), Zone::Head));
        }
    }
    for part in extremities.all() {
        let to_body = Mat4::from_translation(rig.joints[part.joint].position);
        placed.push((part.mesh.transformed(to_body), Zone::Extremity(part.limb)));
    }
    placed
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
    fn the_mouth_is_cut_shut_at_rest_and_parts_when_the_jaw_opens() {
        // #154's whole contract in one assertion chain: the surgery ran, the
        // seam is invisible at rest, the body stayed one closed solid, and a
        // jaw open separates the two edges rather than stretching one sheet.
        // Swept over seeds because a cut that only lands on the default body
        // is a cut that will decline silently on the first re-roll.
        use crate::anim::Pose;
        use glam::Quat;

        for seed in [1i64, 3, 7, 21, 33] {
            let avatar = biped(seed);
            let mouth = avatar
                .parts
                .mouth
                .as_ref()
                .unwrap_or_else(|| panic!("seed {seed}: no mouth was cut"));
            assert!(
                mouth.upper.len() >= 4 && mouth.upper.len() == mouth.lower.len(),
                "seed {seed}: a seam of {} over {}",
                mouth.upper.len(),
                mouth.lower.len()
            );
            let body = &avatar.parts.body;
            assert!(
                body.is_closed_manifold(),
                "seed {seed}: the cut opened the solid: {:?}",
                body.manifold_report()
            );
            for (&u, &l) in mouth.upper.iter().zip(&mouth.lower) {
                let apart = body.positions[u as usize].distance(body.positions[l as usize]);
                assert!(
                    apart < 0.0012,
                    "seed {seed}: the resting seam gapes {:.2} mm — the mouth reads open",
                    apart * 1000.0
                );
            }

            let rig = &avatar.rig;
            let tip = (0..rig.len())
                .find(|&tip| {
                    rig.joints[tip].marker
                        && rig.joints[tip]
                            .parent
                            .is_some_and(|pivot| rig.joints[pivot].marker)
                })
                .expect("a humanoid has a jaw");
            let pivot = rig.joints[tip].parent.expect("the tip hangs off the pivot");
            let mut pose = Pose::rest(rig);
            pose.rotations[pivot] = Quat::from_rotation_x(20f32.to_radians());
            let moved = pose
                .forward(rig)
                .deform(rig, &body.positions, &avatar.parts.weights);
            let widest = mouth
                .upper
                .iter()
                .zip(&mouth.lower)
                .map(|(&u, &l)| moved[u as usize].distance(moved[l as usize]))
                .fold(0.0f32, f32::max);
            assert!(
                widest > 0.008,
                "seed {seed}: a 20-degree open parts the seam by only {:.1} mm — the mouth \
                 is still one sheet",
                widest * 1000.0
            );
            // The owner's #155 report, pinned: the lower lip's OUTER skin
            // travels with the jaw, not only the seam's own edge. The field's
            // blend has no business below the parting, and this is what holds
            // that.
            let tip = mouth
                .lip
                .iter()
                .map(|&v| moved[v as usize].distance(body.positions[v as usize]))
                .sum::<f32>()
                / mouth.lip.len().max(1) as f32;
            assert!(
                tip > 0.006,
                "seed {seed}: the lower lip's tip travelled {:.1} mm mean under a                  20-degree open — the lip is stuck on the skull again",
                tip * 1000.0
            );
            let welds = mouth.welds;
            let corner = moved[welds[0] as usize].distance(moved[welds[1] as usize]);
            let rest =
                body.positions[welds[0] as usize].distance(body.positions[welds[1] as usize]);
            assert!(
                (corner - rest).abs() < rest * 0.25,
                "seed {seed}: the mouth's width went {:.1} to {:.1} mm under a jaw open — a \
                 corner should stretch, not fly",
                rest * 1000.0,
                corner * 1000.0
            );
        }
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

    /// Grows a body with one hair length and nothing else changed.
    fn at_length(length: f32) -> Avatar {
        let record = AvatarRecord::new("Built", Archetype::default());
        Avatar::build_with(
            &record,
            &AvatarConfig {
                hair: Some(HairParams {
                    length,
                    ..HairParams::default()
                }),
                ..Default::default()
            },
        )
        .expect("builds")
    }

    #[test]
    fn a_hair_override_replaces_the_record_and_nothing_else() {
        // **`cropped` was 0.0 and is 0.1, because zero is no longer a length**
        // (#124). This test is about the override path, not about the bottom of
        // the axis, and it was asserting on a value that now grows nothing —
        // which would have failed at `.expect("hair")` rather than saying so.
        let cropped = at_length(0.1);
        let long = at_length(1.0);
        assert!(
            long.parts.hair.expect("hair").drop() > cropped.parts.hair.expect("hair").drop() * 2.0
        );
    }

    #[test]
    fn a_hair_length_of_zero_grows_no_hair_at_all() {
        // **The whole set, not a shorter one** (#124). The fall is only part of
        // what a head of hair costs here — the mass is a sculpted shell — so a
        // length of zero used to build a bucket hat: 3,656 triangles and a draw
        // call, on a record asking for none.
        //
        // Asserted three ways because "no hair" has three meanings and the
        // cheap one would pass on its own: nothing in `parts`, nothing drawn,
        // and a draw call fewer than the same body with hair. A part that is
        // `None` while its mesh is still in the merge is exactly the sort of
        // half-removal this crate has caught before.
        let bald = at_length(0.0);
        assert!(
            bald.parts.hair.is_none(),
            "a record asked for no hair and got some"
        );
        let drawn: Vec<MeshKind> = bald.drawn(0.0).iter().map(|item| item.kind).collect();
        assert!(
            !drawn.contains(&MeshKind::Hair),
            "no hair was grown and one was still drawn: {drawn:?}"
        );

        let haired = at_length(0.5);
        assert!(
            bald.budget.meshes < haired.budget.meshes,
            "a bald body costs {} draws against a haired body's {}, so the hair's \
             draw call did not go with it",
            bald.budget.meshes,
            haired.budget.meshes
        );
        assert!(
            bald.budget.tris < haired.budget.tris,
            "a bald body costs {} triangles against a haired body's {}",
            bald.budget.tris,
            haired.budget.tris
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
