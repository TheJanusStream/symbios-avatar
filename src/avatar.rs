//! From a record to something a renderer can draw.
//!
//! Every other module here builds one thing well: a cage, a rig, a head of hair,
//! a pair of trousers. [`Avatar::build`] is the recipe that puts them in the
//! right order and hands back geometry, a texture, a skeleton and a bill.
//!
//! It exists because the recipe is long — a couple of dozen calls whose order
//! matters — and a long recipe living in an example is a second implementation of the
//! crate. Two of them already existed here and had drifted apart: one dressed a
//! quadruped in a fringe and a pair of sleeves, because the rule about which
//! bodies wear clothes was written in the other one.
//!
//! ## What comes out
//!
//! Merged, skinned [`AvatarMesh`]es, grouped by the material they need, because
//! the WebGL2 budget is one draw per skinned mesh and a body made of ninety
//! separate clumps of hair is ninety draws. Merging costs nothing in fidelity —
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
//! One thing is still degenerate: eyelids. They are built after the atlas is
//! packed — a blink is a pose on the four lid joints, and the lid
//! surface rides in the skin's own draw — so nothing reserves a region for
//! them. A lid samples one texel of the face's chart, which at a lid's size is
//! all it needs.

use glam::{Mat4, Vec2, Vec3};
use symbios_texture::generator::TextureMap;

use crate::anim::Pose;
use crate::cage::CageConfig;
use crate::dress::Outfit;
use crate::extremity::Extremities;
use crate::face::{self, Canon, Eyes, Features, Skull};

use crate::hair::{Growth, HairRecord};
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
#[cfg_attr(feature = "serde-avatar", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde-avatar", derive(serde::Serialize, serde::Deserialize))]
pub struct AvatarMesh {
    /// The material it needs.
    pub kind: MeshKind,
    /// Its geometry, in body space, carrying every channel a renderer wants.
    pub mesh: PolyMesh,
}

/// What one avatar costs.
///
/// Measured rather than asserted. The targets it is judged against — four
/// skinned meshes and 30,000 triangles on a WebGL2 tier — live in
/// `tests/budget.rs`, not here, because a budget that enforces its own limits
/// cannot be used to find out how far over them something is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde-avatar", derive(serde::Serialize, serde::Deserialize))]
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
    pub hair: Option<HairRecord>,
    /// Complexion replacing the record's, for the same reason. Named apart from
    /// `skin` above, which is how the mesh is bound rather than what colour it
    /// is. A complexion is judged by looking at it under light, not by reading a
    /// triple of numbers, and a body is the only place to do that.
    pub complexion: Option<SkinParams>,
    /// Whether the body wears the outfit its record asks for.
    ///
    /// **The only way to see a body undressed, and it has to be asked for at
    /// BUILD time.** A dressed body does not emit the skin under its
    /// clothes, so dropping the cloth draw afterwards does not undress it — it
    /// opens a hole where the clothes were, a torso with its middle missing.
    /// That is the cost of suppression stated as plainly as it can be: what a
    /// garment covers stops existing, and only a body built without the
    /// garment has it.
    pub dressed: bool,
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
            dressed: true,
        }
    }
}

/// The structure the merge throws away.
///
/// Merging is for drawing. Everything that wants to *measure* a body — frame a
/// close-up on its head, check a hem, ask how thick an arm came out — needs the
/// parts as parts, so they are kept.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde-avatar", derive(serde::Serialize, serde::Deserialize))]
pub struct Parts {
    /// The body's surface, closed and un-split, before it was charted.
    ///
    /// Not carried across a `serde-avatar` round trip — see the feature's
    /// note in `Cargo.toml`: a sent avatar is drawable, not rebuildable.
    #[cfg_attr(feature = "serde-avatar", serde(skip))]
    pub body: PolyMesh,
    /// What holds each body vertex.
    #[cfg_attr(feature = "serde-avatar", serde(skip))]
    pub weights: SkinWeights,
    /// Which part of the body each body vertex belongs to.
    #[cfg_attr(feature = "serde-avatar", serde(skip))]
    pub zones: Vec<Zone>,
    /// How the body was charted into the atlas.
    #[cfg_attr(feature = "serde-avatar", serde(skip))]
    pub unwrap: UvUnwrap,
    /// The body as measured, which is what every attached part is fitted to.
    pub surface: Surface,
    /// Its eyes, if it has a head.
    pub eyes: Option<Eyes>,
    /// Its face, if it is the kind of body that wears one.
    #[cfg_attr(feature = "serde-avatar", serde(skip))]
    pub features: Option<Features>,
    /// Its hair, grown in its head joint's own space.
    #[cfg_attr(feature = "serde-avatar", serde(skip))]
    pub hair: Option<Growth>,
    /// Its hands and feet.
    #[cfg_attr(feature = "serde-avatar", serde(skip))]
    pub extremities: Extremities,
    /// What it is wearing.
    #[cfg_attr(feature = "serde-avatar", serde(skip))]
    pub outfit: Outfit,
    /// Whether this body carries limbs it does not stand on.
    ///
    /// See [`Avatar::build`] for why that is the question asked.
    pub handed: bool,
    /// The openable mouth's seam and pocket vertices, if one was cut.
    #[cfg_attr(feature = "serde-avatar", serde(skip))]
    pub mouth: Option<face::Mouth>,
}

/// A record, built.
///
/// Deliberately not `Clone`, `Debug` or `PartialEq`: it owns several megabytes
/// of texture, and none of those three operations is one anybody should get by
/// accident on something that size. Compare [`Avatar::meshes`] and
/// [`Avatar::budget`], which are the parts worth comparing.
#[cfg_attr(feature = "serde-avatar", derive(serde::Serialize, serde::Deserialize))]
pub struct Avatar {
    /// The posable hierarchy everything is skinned to.
    pub rig: Rig,
    /// Merged geometry that does not change with expression.
    ///
    /// The eye globes are not here: they want a glossy material and a pivot of
    /// their own. [`Avatar::drawn`] appends them and returns the whole list.
    pub meshes: Vec<AvatarMesh>,
    /// The painted skin atlas.
    #[cfg_attr(feature = "serde-avatar", serde(with = "crate::texture::atlas_serde"))]
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
    /// is a costume rather than a creature — creatures want fur, a muzzle and
    /// a harness of their own, none of which exist here yet. Asking it this
    /// way also gets a body
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
        let traits = face::HeadTraits::of(&record.composites);
        // The trunk's own read of the same axes, `HeadTraits`'s sibling, with
        // the record's own chest axes as offsets on top — `FaceParams::on`'s
        // shape and for its reason: everything downstream has to agree about
        // one chest. A plan that is not a humanoid has no chest axes and takes
        // the composites' answer unchanged, which is the refusal and needs no
        // special path.
        let chest_traits =
            crate::torso::ChestTraits::of(&record.composites).on(match &record.archetype {
                crate::plan::Archetype::Humanoid(params) => crate::torso::ChestAxes {
                    volume: params.chest_volume,
                    projection: params.chest_projection,
                    lift: params.chest_lift,
                    spacing: params.chest_spacing,
                    fullness: params.chest_fullness,
                },
                _ => crate::torso::ChestAxes::default(),
            });
        // The record's face axes are offsets on what the frame axis already
        // derives, so everything downstream reads this and not `record.face` —
        // the carve, the mouth cut and the built features all have to agree
        // about one mouth. See `FaceParams::on`.
        let face_params = record.face.on(&traits);
        let mut body =
            crate::build_body(&skeleton, &config.cage, config.subdivisions, &traits).ok()?;
        let mut rig = Rig::from_skeleton(&skeleton).ok()?;
        // A swelled chest earns its own refinement pass, here because the
        // chest's traits are this function's and `build_body` knows only the
        // head's — and before `Skull::measure`, so nothing has read the
        // surface yet (#311).
        body = crate::torso::refine_lobe(&body, &rig, &chest_traits);

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
        // The skull is kept rather than consumed: the follicle regions are cut
        // from it too, and `Skull::measure` is 61% of a geometry build (#89) —
        // so measuring a second one for the hair would be the single most
        // expensive line in this function.
        let skull = Skull::measure(&body, &rig);
        let canon = skull
            .as_ref()
            .map(|skull| Canon::measure(&rig, skull, &record.eyes));
        if let Some(canon) = &canon {
            face::carve_face(&mut body, &rig, canon, &face_params);
        }
        // The chest, on the same rule as the face and for the same reason: it
        // is a displacement of the body's own surface, so everything that
        // fits itself to that surface has to run after it (#272). Independent
        // of the face's canon — a body with no head still has a chest — so it
        // is outside the `canon` block rather than inside it.
        //
        // **The wall is remembered before the carve, and the skin is BOUND to
        // the wall** (#313). A breast is chest-wall skin displaced outward,
        // and it has to move with the wall it came off; bound where it
        // stands, a large lobe's crest sits two hundred millimetres from the
        // chest bone and one hundred from the upper arm's, so the falloff
        // handed it to the arm — vertex by vertex, wherever the two pulls
        // crossed — and the first pose that lowered the arms shredded the
        // lobe. The carve moves positions and adds no vertices, and the cut
        // below only appends, so the snapshot maps onto the bound mesh by
        // index; every other fitting (charts, the garment cut, the eyes)
        // still reads the carved surface, as it must.
        let wall = body.positions.clone();
        crate::torso::carve_chest(&mut body, &rig, &chest_traits);
        crate::torso::carve_taper(&mut body, &rig, &chest_traits);
        // The mouth is cut AFTER the carve — the parting follows the carved
        // groove — and before anything measures, binds or charts the surface,
        // so the pocket is simply more body to all of them (#154). A rig with
        // no jaw markers gets no cut and no change.
        let mouth = canon
            .as_ref()
            .and_then(|canon| face::mouth::open(&mut body, &rig, canon, &face_params));
        let body = body;
        let mut eyes = canon
            .as_ref()
            .map(|canon| Eyes::build(&rig, &body, canon, &record.eyes));

        let mut weights = {
            let mut on_the_wall = body.clone();
            on_the_wall.positions[..wall.len()].copy_from_slice(&wall);
            skin::bind(&on_the_wall, &rig, &config.skin)
        };
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
                // The slit's outer edges give the mouth CORNERS back their
                // share (#216). This overwrite runs after `skin::bind`, so
                // without it the corner field would be wiped exactly at the
                // lip's free edge — the patch around the seam would smile
                // while the seam stayed, which is a tear. Blended, the two
                // edges still differ (head against jaw, so the mouth still
                // parts) and both converge to corner-held at the commissure,
                // so a smile carries the seam's endpoint as one thing. The
                // pocket — teeth, roof, floor — stays solely held: a corner
                // field that caught interior vertices would make the teeth
                // smile, and nothing about a tooth does.
                let corners = crate::rig::skin::mouth_corners(&rig);
                let held_with_corner = |base: usize, position: Vec3| {
                    let mut held = solely(base);
                    if let Some((share, corner)) = corners
                        .iter()
                        .map(|&corner| {
                            (
                                crate::rig::skin::corner_hold_at(&rig, corner, position),
                                corner.0,
                            )
                        })
                        .max_by(|a, b| a.0.total_cmp(&b.0))
                        .filter(|&(share, _)| share > 0.0)
                    {
                        held[0].weight = 1.0 - share;
                        held[1] = crate::rig::Influence {
                            joint: corner as u16,
                            weight: share,
                        };
                    }
                    held
                };
                for &vertex in mouth.upper.iter().chain(&mouth.overlip) {
                    weights.vertices[vertex as usize] =
                        held_with_corner(head, body.positions[vertex as usize]);
                }
                for &vertex in mouth.roof.iter().chain(&mouth.teeth) {
                    weights.vertices[vertex as usize] = solely(head);
                }
                for &vertex in mouth.lower.iter().chain(&mouth.lip) {
                    weights.vertices[vertex as usize] =
                        held_with_corner(pivot, body.positions[vertex as usize]);
                }
                for &vertex in &mouth.floor {
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
        let hair_record = config.hair.unwrap_or(record.hair);

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
        // And the lids get joints, which is what turns a blink from a rebuild
        // into a pose and retires the draw the shells used to cost (#118).
        //
        // **Here and not earlier**: `skin::bind` above has already run, and a
        // joint inside the skull offered to its falloff would take cheek and
        // brow vertices with it every time the eye shut (#136). `Surface` is
        // measured above for the same reason from the other side — it asks what
        // lies under a point, and a lid joint stands for no surface at all.
        if let Some(eyes) = &mut eyes {
            eyes.rig(&mut rig);
        }

        // The attached parts exist BEFORE the body is unwrapped, so the packer
        // can reserve them regions of the same atlas rather than being asked
        // afterwards for whatever the body did not want. The parts are the half
        // that needs the texels — a face is judged on its nose, its mouth and
        // its ears, and none of those is part of the body mesh.
        let mut wanted: Vec<Vec2> = attached_meshes(&features, &extremities, &eyes)
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
        place_charts(&mut features, &mut extremities, &mut eyes, &reserved);

        // Cut from the body, so charted where the body is charted.
        let mut outfit = if handed && config.dressed {
            Outfit::wear(&body, &rig, &weights, &zones, &record.outfit)
        } else {
            Outfit::default()
        };
        let body_uvs = charts.by_source(body.vertex_count());
        for garment in &mut outfit.garments {
            garment.chart(&body_uvs);
        }

        // Baked in body space, which is where a painter wants them: a nose is
        // then painted by the same complexion arithmetic as the cheek beside it.
        let placed = attached_in_body(&rig, &features, &extremities, &eyes);
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
        // The body's composition reaches the painter here, and #165 is where
        // that plumbing arrived: until then the skin knew a complexion and
        // nothing about the body wearing it, so a lean body and a heavy one
        // were painted identically. `Condition` is the skin's derived read of
        // the composites, the way `HeadTraits` is the skull's.
        // Where hair may grow, which the painted layer needs and the grown one
        // will take over from the old shell at #209.
        let follicles = skull.as_ref().zip(canon.as_ref()).map(|(skull, canon)| {
            crate::hair::Follicles::of(&rig, skull, canon, &hair_record.regions)
        });
        let complexion = config.complexion.unwrap_or(record.skin);
        let painted_hair = hair_record.painted();
        let layer = follicles.as_ref().map(|follicles| texture::PaintedLayer {
            follicles,
            painted: &painted_hair,
        });
        let painted = texture::paint_skin(
            &geometry,
            &rig,
            &complexion,
            &texture::Condition::of(&record.composites),
            layer.as_ref(),
        );

        // **Grown from the record, region by region** (#202). A body with no
        // hands is not the kind of body that wears a haircut — the same rule
        // the shell era used — and one with no measured head has nowhere to
        // put one.
        let hair = follicles.as_ref().filter(|_| handed).map(|follicles| {
            let bed = crate::hair::clump::Bed {
                weights: &weights,
                body: &body,
                rig: &rig,
                follicles,
            };
            // **Tiered to fit** (#209): the counts each style asks for are a
            // request, and `grow_head` grants what the budget holds. The loop
            // itself lives with the clump engine rather than here, because
            // `tests/budget.rs` grows heads of hair too and a second copy of it
            // is a second opinion about the one thing that has to match.
            let sown: Vec<_> = crate::hair::Follicle::ALL
                .into_iter()
                .filter_map(|follicle| {
                    hair_record
                        .sowing(follicle, follicles)
                        .map(|sown| (follicle, sown))
                })
                .collect();
            let sowings: Vec<_> = sown
                .iter()
                .map(|(follicle, sown)| crate::hair::clump::Sowing {
                    follicle: *follicle,
                    count: sown.clumps,
                    shape: sown.shape.as_ref(),
                    roots: Vec3::from_array(sown.roots),
                    tips: Vec3::from_array(sown.tips),
                })
                .collect();
            crate::hair::clump::grow_head(
                &bed,
                &sowings,
                record.seed,
                crate::hair::clump::MAX_TRIANGLES,
            )
        });
        // A region that grew nothing leaves no part behind. The merge keys off
        // this, and a `Some` holding an empty mesh would be a part whose draw
        // call went missing — the half-removal `a_hair_length_of_zero_grows_no_hair_at_all`
        // exists to catch.
        let hair = hair.filter(|growth| growth.mesh.face_count() > 0);

        let parts = Parts {
            hair,
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

    /// Everything to draw: the merged skinned meshes and the eye globes.
    ///
    /// `closure` runs `0` for open to `1` for shut, and is passed through to
    /// [`Self::eyes_at`] — which ignores it, because a blink is a pose the lid
    /// joints carry rather than geometry. See that method for why the parameter
    /// is kept.
    #[must_use]
    pub fn drawn(&self, closure: f32) -> Vec<AvatarMesh> {
        let mut drawn = self.meshes.clone();
        drawn.extend(self.eyes_at(closure));
        drawn
    }

    /// The eye globes.
    ///
    /// **The lids are not here.** A lid swings about its eye's pivot, but it
    /// is driven by four lid joints on the rig and rides in the skin's own
    /// draw, so a blink is [`Eyes::blink`] writing a pose and this
    /// returns the same geometry whatever it is passed.
    ///
    /// `closure` is kept in the signature and ignored, so that a caller which
    /// hands over a blink phase still compiles and still gets the right picture
    /// — and so that the one thing that still varies with closure, should
    /// anything ever be added back, has somewhere to arrive.
    #[must_use]
    pub fn eyes_at(&self, closure: f32) -> Vec<AvatarMesh> {
        let Some(eyes) = &self.parts.eyes else {
            return Vec::new();
        };
        let to_body = Mat4::from_translation(self.rig.joints[eyes.head].position);

        let _ = closure;
        let mut globes = PolyMesh::new();
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
                    .map(|&point| face::eye::iris_of(point - eye.pivot, &eyes.params))
                    .collect(),
            );
            // **Bound to its OWN joint, before the append** (#235). Both globes
            // used to bind rigidly to the head, which is what welded the baked
            // iris to the skull: a body could not glance without turning its
            // whole face. Each globe binds to the joint hung on its own pivot,
            // so `Eyes::look` turns it and the baked colour rides along —
            // exactly as the lids' geometry rides their joints.
            //
            // Per globe rather than once over the pair, because after this they
            // are two different bindings inside one mesh, and `bind_rigidly`
            // assigns every vertex it is given.
            globe.bind_rigidly(eye.globe_joint.unwrap_or(eyes.head) as u16);
            globes.append(&globe);
        }

        if globes.faces.is_empty() {
            return Vec::new();
        }
        globes.set_normals(globes.vertex_normals());
        let placed = globes.transformed(to_body);
        vec![AvatarMesh {
            kind: MeshKind::Eye,
            mesh: placed.split_uv_seams(),
        }]
    }

    /// The body posed, with every mesh deformed to match.
    ///
    /// The one call an integration needs per frame once the geometry is up.
    ///
    /// **`closure` reaches the lids through the POSE**, not through a rebuild
    /// of their geometry. The blink is written onto a copy of
    /// what the caller handed over, so a caller's own pose is not edited behind
    /// its back and a pose that already carries a walk keeps it — the four lid
    /// joints are the only ones touched.
    #[must_use]
    pub fn posed(&self, pose: &Pose, closure: f32) -> Vec<AvatarMesh> {
        let mut blinking;
        let pose = match &self.parts.eyes {
            Some(eyes) => {
                blinking = pose.clone();
                eyes.blink(&mut blinking, closure);
                &blinking
            }
            None => pose,
        };
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
                skin.append(&placed.split_uv_seams());
            } else {
                // A hand is WELDED, not appended (#297): the arm's stub
                // surface is cut out of the body and the hand's open weld
                // ring is bridged into the hole, so the wrist is one surface
                // rather than two nested ones.
                crate::extremity::weld(
                    &mut skin,
                    &placed.split_uv_seams(),
                    &self.rig,
                    part.limb,
                    part.joint,
                );
            }
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

        if let Some(eyes) = &self.parts.eyes {
            // The lids, into the same draw as the face they sit in — which is
            // the whole of #118's first numeric win. Bound rigidly to their own
            // joints rather than to the head's, so a blink is those four joints
            // rotating and nothing else moves.
            let to_body = Mat4::from_translation(self.rig.joints[eyes.head].position);
            for (lid, joint) in eyes.lids() {
                let mut mesh = lid.clone();
                mesh.set_normals(mesh.vertex_normals());
                mesh.paint(Vec3::ONE);
                let mut placed = mesh.transformed(to_body);
                placed.bind_rigidly(joint as u16);
                skin.append(&placed.split_uv_seams());
            }
        }

        let mut merged = vec![AvatarMesh {
            kind: MeshKind::Skin,
            mesh: skin,
        }];

        // One mesh for every region of hair, carrying its own colours per
        // vertex — so the walk over brightness the shell needed to stop reading
        // as a helmet is now the root-to-tip fade a record asked for.
        if let Some(growth) = self
            .parts
            .hair
            .as_ref()
            .filter(|growth| growth.mesh.face_count() > 0)
        {
            let to_body = Mat4::from_translation(self.rig.joints[growth.head].position);
            let mut placed = growth.mesh.transformed(to_body);
            placed.set_normals(placed.vertex_normals());
            // **Not bound rigidly to the head, which is what it used to be**
            // (#207). Every clump already carries the binding of the skin it
            // grew out of, so a moustache rides the upper lip, a chin beard
            // rides the JAW, and a flank hair crossing the jawline blends across
            // it the way the skin under it does. Rigid to the head, a beard
            // stayed where the closed mouth was while the chin dropped 44.7 mm
            // out from under it.
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
    ///
    /// **Minus the skin under the clothes.** A garment is the body
    /// pushed outward, so it stands over every face it was cut from — outer
    /// shell above, inner shell below, rim across the hem — and cloth deforms
    /// with the skin beneath it, so it stands there in every pose too. That
    /// skin is dropped here rather than hidden at draw time, which is what
    /// `plan.md` specified and what `plan::zone` and `SkinWeights::zones` have
    /// always claimed: poke-through is avoided by not emitting the geometry.
    ///
    /// Not the whole claim — [`Garment::hidden`](crate::Garment::hidden), which
    /// gives back the row of faces the hem runs through, since a smoothed hem
    /// no longer follows their edges. Measured with `cargo run --release
    /// --example garmentaudit`: 1,216 triangles on the default body and 1,236
    /// at the dearest, from 360 in the barest cut (bare sleeves and shorts) to
    /// 1,344 in the fullest. Without the drop, the whole avatar's headroom
    /// under `TRIANGLE_CEILING` would be 82.
    ///
    /// Only `parts.body` is cut; `Parts` keeps the whole surface, because
    /// everything that measures a body — a hem, a chin, a foot's contact — needs
    /// the skin that is there whether or not it is drawn.
    fn charted_body(&self) -> PolyMesh {
        let charts = &self.parts.unwrap;
        // Normals over the body's own topology, then gathered through the
        // unwrap. Derived from the unwrapped copy they would split every seam.
        let normals = self.parts.body.vertex_normals();
        let hidden = self.parts.outfit.covered(self.parts.body.face_count());
        let faces: Vec<Vec<u32>> = charts
            .faces
            .iter()
            .zip(&charts.source_face)
            .filter(|&(_, &from)| !hidden.get(from as usize).copied().unwrap_or(false))
            .map(|(face, _)| face.clone())
            .collect();

        // The vertices the surviving faces still use, renumbered in place. A
        // dropped face leaves its corners behind, and an unwrapped body vertex
        // is carried four ways — position, chart coordinate, normal, four
        // influences — so leaving them would keep most of the cost of the
        // geometry that was just deleted.
        let mut used = vec![false; charts.vertex_count()];
        for face in &faces {
            for &corner in face {
                used[corner as usize] = true;
            }
        }
        let mut renumbered = vec![0u32; charts.vertex_count()];
        let mut kept = 0u32;
        for (vertex, &used) in used.iter().enumerate() {
            renumbered[vertex] = kept;
            kept += u32::from(used);
        }
        /// Drops the entries of a per-vertex channel whose vertex went with a
        /// face. A closure cannot do this: it is used at four different types.
        fn keep<T>(attribute: Vec<T>, used: &[bool]) -> Vec<T> {
            attribute
                .into_iter()
                .zip(used)
                .filter_map(|(value, &used)| used.then_some(value))
                .collect()
        }

        let mut mesh = PolyMesh {
            positions: keep(charts.gather(&self.parts.body.positions), &used),
            faces: faces
                .into_iter()
                .map(|face| {
                    face.into_iter()
                        .map(|corner| renumbered[corner as usize])
                        .collect()
                })
                .collect(),
            ..Default::default()
        };
        mesh.set_uvs(keep(charts.uvs.clone(), &used));
        mesh.set_normals(keep(charts.gather(&normals), &used));
        mesh.set_skin(keep(charts.gather(&self.parts.weights.vertices), &used));
        mesh.paint(Vec3::ONE);
        mesh
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
    eyes: &'a Option<Eyes>,
) -> impl Iterator<Item = (&'a PolyMesh, Zone)> {
    features
        .iter()
        .flat_map(|face| face.meshes().map(|mesh| (mesh, Zone::Head)))
        .chain(
            extremities
                .all()
                .map(|part| (&part.mesh, Zone::Extremity(part.limb))),
        )
        // The lids LAST, so adding them left every region that was already
        // being handed out where it was. They are ordinary attached geometry
        // now that a joint carries them, which is what the old `charted`
        // degenerate mapping said would happen when the face got a rig.
        .chain(
            eyes.iter()
                .flat_map(|eyes| eyes.lids().map(|(mesh, _)| (mesh, Zone::Head))),
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
fn place_charts(
    features: &mut Option<Features>,
    extremities: &mut Extremities,
    eyes: &mut Option<Eyes>,
    reserved: &[Rect],
) {
    let meshes = features
        .iter_mut()
        .flat_map(|face| face.meshes_mut())
        .chain(extremities.all_mut().map(|part| &mut part.mesh))
        .chain(eyes.iter_mut().flat_map(Eyes::lids_mut));
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
    eyes: &Option<Eyes>,
) -> Vec<(PolyMesh, Zone)> {
    let mut placed = Vec::new();
    if let Some(face) = features {
        let to_body = Mat4::from_translation(rig.joints[face.head].position);
        for mesh in face.meshes() {
            placed.push((mesh.transformed(to_body), Zone::Head));
        }
    }
    if let Some(eyes) = eyes {
        // Head-local, like a feature and unlike an extremity, so the head's own
        // joint is what carries them into body space. The lid JOINT is not the
        // right transform here: a texel has to say where on the body it sits,
        // and at rest a lid joint and the head agree about that anyway.
        let to_body = Mat4::from_translation(rig.joints[eyes.head].position);
        for (mesh, _) in eyes.lids() {
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
    fn a_dressed_body_does_not_draw_the_skin_under_its_clothes() {
        // #46, and the sentence `plan::zone` and `SkinWeights::zones` had been
        // making for months before it was true. The saving is asserted as an
        // exact identity rather than as a threshold: every claimed face, and
        // only those, and nothing else in the merge moved.
        for seed in [1i64, 7, 42] {
            let dressed = biped(seed);
            let hidden = dressed
                .parts
                .outfit
                .covered(dressed.parts.body.face_count());
            let owed: usize = hidden
                .iter()
                .enumerate()
                .filter(|&(_, &hidden)| hidden)
                .map(|(face, _)| dressed.parts.body.faces[face].len().saturating_sub(2))
                .sum();
            assert!(
                owed > 500,
                "seed {seed}: only {owed} triangles were covered"
            );

            let mut bare = AvatarRecord::new("Built", Archetype::default());
            bare.reroll(seed);
            bare.outfit.sleeve = crate::dress::Sleeve::Bare;
            let undressed = Avatar::build(&bare).expect("a biped builds");
            let skin = |avatar: &Avatar| {
                avatar
                    .drawn(0.0)
                    .into_iter()
                    .filter(|mesh| mesh.kind == MeshKind::Skin)
                    .map(|mesh| mesh.mesh.triangulated().len())
                    .sum::<usize>()
            };
            // Bare sleeves claim less, so the same body draws MORE skin: the
            // difference is the two sleeves, and it is only a difference of
            // suppression because everything else about the two bodies is one
            // record apart.
            assert!(
                skin(&undressed) > skin(&dressed),
                "seed {seed}: bare sleeves drew {} skin triangles against {}",
                skin(&undressed),
                skin(&dressed)
            );
        }
    }

    #[test]
    fn a_body_built_undressed_keeps_every_face_of_its_skin() {
        // What `AvatarConfig::dressed` is for, and the guard on the trap it was
        // added for: once a dressed body stops emitting the skin under its
        // clothes, dropping the cloth draw does not undress it, it holes it.
        // `examples/render --bare` was doing exactly that.
        let mut record = AvatarRecord::new("Built", Archetype::default());
        record.reroll(4);
        let dressed = Avatar::build(&record).expect("a biped builds");
        let undressed = Avatar::build_with(
            &record,
            &AvatarConfig {
                dressed: false,
                ..Default::default()
            },
        )
        .expect("a biped builds");

        assert!(undressed.parts.outfit.is_empty());
        assert!(!dressed.parts.outfit.is_empty());
        let skin = |avatar: &Avatar| {
            avatar
                .drawn(0.0)
                .into_iter()
                .find(|mesh| mesh.kind == MeshKind::Skin)
                .expect("a body draws skin")
                .mesh
                .clone()
        };
        // Whole: every face the unwrap made, and the attached parts on top.
        assert!(skin(&undressed).face_count() > undressed.parts.unwrap.faces.len());
        assert!(
            skin(&undressed).triangulated().len() > skin(&dressed).triangulated().len() + 1_000,
            "undressed drew {} skin triangles against a dressed {}",
            skin(&undressed).triangulated().len(),
            skin(&dressed).triangulated().len()
        );
        assert!(
            !undressed
                .drawn(0.0)
                .iter()
                .any(|mesh| mesh.kind == MeshKind::Cloth)
        );
    }

    #[test]
    fn suppressing_the_covered_skin_leaves_no_vertex_behind() {
        // A dropped face leaves its corners in the list, and an unwrapped body
        // vertex carries a position, a chart coordinate, a normal and four
        // influences. Keeping them would keep most of the cost of the geometry
        // that was just deleted, and would ship a mesh most of whose points no
        // triangle refers to.
        let avatar = biped(7);
        let skin = avatar
            .drawn(0.0)
            .into_iter()
            .find(|mesh| mesh.kind == MeshKind::Skin)
            .expect("a body draws skin");
        assert!(skin.mesh.channels_are_consistent());
        let mut used = vec![false; skin.mesh.vertex_count()];
        for face in &skin.mesh.faces {
            for &corner in face {
                used[corner as usize] = true;
            }
        }
        assert!(
            used.iter().all(|&used| used),
            "{} of {} skin vertices are used by no face",
            used.iter().filter(|&&used| !used).count(),
            used.len()
        );
    }

    #[test]
    fn materials_are_merged_to_one_mesh_each() {
        let avatar = biped(7);
        let mut kinds: Vec<MeshKind> = avatar.drawn(0.0).iter().map(|m| m.kind).collect();
        let all = kinds.len();
        kinds.sort_unstable();
        kinds.dedup();
        // **One draw per material, with no exception left.** Skin used to appear
        // twice — the body and the lids — because a lid moved and nothing
        // rigged it; #118 gave the four lids joints and folded their shells into
        // the skin's own mesh, so this is now a plain equality and the gate's
        // criterion 1 is the thing that holds it (`tests/budget.rs`).
        assert_eq!(all, kinds.len(), "{kinds:?} out of {all} draws");
        assert!(kinds.contains(&MeshKind::Hair));
        assert!(kinds.contains(&MeshKind::Cloth));
        assert!(kinds.contains(&MeshKind::Eye));
    }

    #[test]
    fn hair_keeps_a_shade_per_lock_through_the_merge() {
        // Merging is only free if nothing is lost by it. One solid in one colour
        // reads as a helmet, so the walk over brightness has to survive.
        let mut record = AvatarRecord::new("Built", Archetype::default());
        record.reroll(3);
        let avatar = Avatar::build(&record).expect("a biped builds");
        let hair = avatar
            .drawn(0.0)
            .into_iter()
            .find(|m| m.kind == MeshKind::Hair)
            .expect("a biped has hair");
        // **Measured as the SPREAD between the record's two colours, not as a
        // count of distinct ones** (#205). Counting quantised tones read
        // whatever set of station fractions the sampler happened to produce:
        // index-based sampling gave a clump of N stations the fractions
        // `k/(N-1)`, so a head with clumps of four, five and six stations
        // carried a wide assortment of values and the count was comfortably
        // over eight. Sampling by travel makes those fractions dyadic — 0, ½,
        // ¼, ¾ — which is CORRECT (a station a quarter of the way down the hair
        // should be a quarter of the way down the gradient) and which drops the
        // count to exactly eight without changing anything about whether hair
        // reads as one solid.
        //
        // So it asks the question the name asks: does the merged mesh still walk
        // from the roots' colour to the tips'? A helmet would be one point on
        // that line; hair is spread along it.
        let (roots, tips) = (
            Vec3::from_array(record.hair.scalp.roots),
            Vec3::from_array(record.hair.scalp.tips),
        );
        let span = tips - roots;
        assert!(
            span.length() > 1e-3,
            "this seed rolled one colour for both ends, so there is no gradient to lose"
        );
        let alongs: Vec<f32> = hair
            .mesh
            .colours
            .iter()
            .map(|colour| (*colour - roots).dot(span) / span.length_squared())
            .collect();
        let (low, high) = alongs.iter().fold((f32::MAX, f32::MIN), |span, at| {
            (span.0.min(*at), span.1.max(*at))
        });
        assert!(
            high - low > 0.8,
            "{} locks span only {:.2} of the way from the roots' colour to the tips', so the \
             merge has flattened the gradient",
            avatar.parts.hair.as_ref().map_or(0, Growth::clumps),
            high - low
        );
        let between = alongs
            .iter()
            .filter(|along| (0.1..=0.9).contains(*along))
            .count();
        assert!(
            between * 4 > alongs.len(),
            "only {between} of {} hair vertices are anywhere between the two colours: the \
             gradient is two flat ends rather than a walk",
            alongs.len()
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
        // Through `posed` rather than `eyes_at`, because a blink is a POSE now
        // (#118) and `eyes_at` returns the globes whatever it is passed. The
        // lids are inside the skin's own draw, so the mesh this reads for them
        // is the one the face is in.
        let avatar = biped(5);
        let at = |closure: f32| {
            let pose = Pose::rest(&avatar.rig);
            avatar
                .posed(&pose, closure)
                .into_iter()
                .map(|m| (m.kind, m.mesh))
                .collect::<Vec<_>>()
        };
        let open = at(0.0);
        let shut = at(1.0);
        assert_eq!(open.len(), 4, "the lids are still a draw of their own");
        let globes = open.iter().position(|(kind, _)| *kind == MeshKind::Eye);
        let globes = globes.expect("a biped has eye globes");
        assert_eq!(
            open[globes].1, shut[globes].1,
            "the globes moved during a blink"
        );
        let skin = open
            .iter()
            .position(|(kind, _)| *kind == MeshKind::Skin)
            .expect("a body has skin");
        assert_ne!(open[skin].1, shut[skin].1, "the lids did not move");
    }

    #[test]
    fn a_glance_moves_the_globes_and_leaves_the_head_alone() {
        // **The integration half of #235.** `Eyes::look` writing a rotation
        // onto a joint proves nothing on its own: the globes must be BOUND to
        // those joints for the pose to reach the drawn mesh, and before this
        // both were bound rigidly to the head. Each globe is bound before the
        // pair is appended into one mesh, so the binding has to survive that
        // append — which is precisely the step a single `bind_rigidly` over the
        // finished pair would have flattened.
        let avatar = biped(7);
        let eyes = avatar.parts.eyes.as_ref().expect("a biped has eyes");
        let head = avatar.rig.joints[eyes.head].position;

        let globes_of = |target: Vec3| {
            let mut pose = Pose::rest(&avatar.rig);
            eyes.look(&avatar.rig, &mut pose, target);
            avatar
                .posed(&pose, 0.0)
                .into_iter()
                .find(|m| m.kind == MeshKind::Eye)
                .expect("a biped draws its globes")
                .mesh
        };
        let ahead = globes_of(head + Vec3::new(0.0, 0.0, 3.0));
        let aside = globes_of(head + Vec3::new(1.2, 0.0, 3.0));

        let moved = ahead
            .positions
            .iter()
            .zip(&aside.positions)
            .map(|(a, b)| a.distance(*b))
            .fold(0.0f32, f32::max);
        println!("a glance moved a globe vertex {:.2} mm", moved * 1000.0);
        assert!(
            moved > 0.001,
            "a glance moved the globes {:.3} mm — the pose is not reaching the mesh, so the \
             baked iris is still welded to the skull",
            moved * 1000.0
        );

        // And it is the GLOBES that moved rather than the whole head: the skin
        // is untouched by a glance, which is the difference between looking and
        // turning to look.
        let skin_of = |target: Vec3| {
            let mut pose = Pose::rest(&avatar.rig);
            eyes.look(&avatar.rig, &mut pose, target);
            avatar
                .posed(&pose, 0.0)
                .into_iter()
                .find(|m| m.kind == MeshKind::Skin)
                .expect("a body has skin")
                .mesh
        };
        assert_eq!(
            skin_of(head + Vec3::new(0.0, 0.0, 3.0)).positions,
            skin_of(head + Vec3::new(1.2, 0.0, 3.0)).positions,
            "a glance moved the face as well as the eyes"
        );
    }

    #[test]
    fn a_posed_lid_lands_where_the_rebuilt_one_did() {
        // **The equivalence #118's lid slice rests on.** A joint sitting exactly
        // on its eye's pivot skins by `T(p) · R · T(−p)`, which is what
        // `Eye::lid_transform` computes — so binding a shell to that joint and
        // rotating it by `Eye::lid_rotation` has to reproduce, vertex for
        // vertex, the geometry the old rebuild handed over. If that ever stops
        // being true the lids will still draw and will simply be in the wrong
        // place, which is exactly the kind of defect a render flatters.
        //
        // Asserted against the transform rather than against remembered
        // positions: the shells' own shape is free to change.
        let avatar = biped(5);
        let eyes = avatar.parts.eyes.as_ref().expect("a biped has eyes");
        let origin = avatar.rig.joints[eyes.head].position;
        for closure in [0.0, 0.35, 1.0] {
            let mut pose = Pose::rest(&avatar.rig);
            eyes.blink(&mut pose, closure);
            let posed = pose.forward(&avatar.rig);
            for (eye, upper) in [
                (&eyes.left, true),
                (&eyes.left, false),
                (&eyes.right, true),
                (&eyes.right, false),
            ] {
                let (lid, joint) = if upper {
                    (&eye.upper_lid, eye.upper_joint)
                } else {
                    (&eye.lower_lid, eye.lower_joint)
                };
                let joint = joint.expect("a built pair is rigged");
                let rebuilt = lid.transformed(eye.lid_transform(closure, upper));
                let skinning =
                    Mat4::from_rotation_translation(posed.rotations[joint], posed.positions[joint])
                        * Mat4::from_translation(-avatar.rig.joints[joint].position);
                for (source, expected) in lid.positions.iter().zip(&rebuilt.positions) {
                    let drawn = skinning.transform_point3(*source + origin);
                    assert!(
                        (drawn - (*expected + origin)).length() < 1e-5,
                        "at closure {closure} a posed lid vertex landed {:.4} mm from the \
                         rebuilt one",
                        (drawn - (*expected + origin)).length() * 1000.0
                    );
                }
            }
        }
    }

    #[test]
    fn a_lid_joint_is_a_marker_and_holds_no_body_skin() {
        // #136's contract, which #118's own comment predicted every new skull
        // joint would erode. A lid joint stands for no surface — it is a hinge
        // inside the head — so nothing asking what lies under a point may find
        // one, and `skin::bind` must never have offered it a cheek.
        let avatar = biped(5);
        let eyes = avatar.parts.eyes.as_ref().expect("a biped has eyes");
        let lids: Vec<usize> = eyes.lids().map(|(_, joint)| joint).collect();
        assert_eq!(lids.len(), 4, "a pair of eyes has four lids");
        for &joint in &lids {
            assert!(
                avatar.rig.joints[joint].marker,
                "lid joint {joint} is not a marker"
            );
            assert!(
                !avatar.rig.surfaced().any(|surfaced| surfaced == joint),
                "lid joint {joint} answers a surface query"
            );
        }
        for (vertex, influences) in avatar.parts.weights.vertices.iter().enumerate() {
            for influence in influences {
                assert!(
                    !lids.contains(&(influence.joint as usize)) || influence.weight == 0.0,
                    "body vertex {vertex} is held by a lid joint"
                );
            }
        }
    }

    #[test]
    fn posing_moves_every_mesh_together() {
        use glam::Quat;

        let avatar = biped(13);
        let mut pose = Pose::rest(&avatar.rig);
        pose.translation = Vec3::new(0.0, 0.5, 0.0);
        pose.rotations.fill(Quat::IDENTITY);

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

    /// A body whose every region is styled `None`: the record's way of saying
    /// bald.
    fn bald() -> Avatar {
        let record = AvatarRecord::new("Bald", Archetype::default());
        Avatar::build_with(
            &record,
            &AvatarConfig {
                hair: Some(HairRecord::bald()),
                ..Default::default()
            },
        )
        .expect("builds")
    }

    /// Grows a body with one hair length and nothing else changed.
    fn at_length(length: f32) -> Avatar {
        let record = AvatarRecord::new("Built", Archetype::default());
        Avatar::build_with(
            &record,
            &AvatarConfig {
                hair: Some(HairRecord {
                    scalp: crate::hair::Tress {
                        style: crate::hair::ScalpStyle::Crop,
                        cut: crate::hair::Cut {
                            length,
                            ..crate::hair::Cut::default()
                        },
                        ..Default::default()
                    },
                    ..HairRecord::default()
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
        //
        // Measured as how far the hair reaches below the head joint rather than
        // as a `drop` the shell used to report: the clump engine has no such
        // number, and the thing the override is supposed to change is how long
        // the hair is.
        let reach = |avatar: &Avatar| {
            let hair = avatar.parts.hair.as_ref().expect("hair");
            hair.mesh
                .positions
                .iter()
                .map(|point| -point.y)
                .fold(f32::MIN, f32::max)
        };
        // **A length axis moves what HANGS, not what the card covers** (#204).
        // A scalp card runs from the crown to the hairline whatever the record
        // asks for — that is what covers a head — and the axis is how far it
        // carries on past it. So the ratio between a short crop and a long one is
        // no longer the ratio of their axes: measured, 46.1 mm against 64.9, where
        // this asked for half again. Asserted as a real difference in millimetres
        // instead, which is what the override is for.
        let cropped = at_length(0.1);
        let long = at_length(1.0);
        assert!(
            reach(&long) > reach(&cropped) + 0.010,
            "a long crop reaches {:.1} mm below the head joint against a short one\'s {:.1}",
            reach(&long) * 1000.0,
            reach(&cropped) * 1000.0
        );
    }

    #[test]
    fn a_record_that_asks_for_no_hair_grows_none() {
        // **The whole set, not a shorter one** (#124). The fall is only part of
        // what a head of hair costs here — the mass was a sculpted shell — so a
        // length of zero used to build a bucket hat: 3,656 triangles and a draw
        // call, on a record asking for none.
        //
        // **And "no hair" is a style rather than a length now** (#202). #124
        // had to make the bottom of the length axis a cliff — `0.001` a full
        // hood and `0.000` nothing — because there was no other way for a
        // record to say bald, and its own docstring called that out as the
        // price of the arrangement. Every region has a `None` style now, so the
        // cliff is gone: a length of zero is short hair, and none is none.
        //
        // Asserted three ways because "no hair" has three meanings and the
        // cheap one would pass on its own: nothing in `parts`, nothing drawn,
        // and a draw call fewer than the same body with hair. A part that is
        // `None` while its mesh is still in the merge is exactly the sort of
        // half-removal this crate has caught before.
        let bald = bald();
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

    /// Everything a renderer reads survives a worker boundary, and the
    /// intermediates deliberately do not.
    #[cfg(feature = "serde-avatar")]
    #[test]
    fn a_built_avatar_crosses_a_worker_boundary_drawable() {
        let mut record = AvatarRecord::new("Sent", Archetype::default());
        record.reroll(23);
        let built = Avatar::build_with(
            &record,
            &AvatarConfig {
                atlas: 64,
                ..AvatarConfig::default()
            },
        )
        .expect("builds");

        // Round-tripped through JSON rather than the worker's own msgpack:
        // this crate should not take a codec dependency to prove a contract
        // about its own types, and serde's model is the same either way.
        let wire = serde_json::to_string(&built).expect("serialises");
        let back: Avatar = serde_json::from_str(&wire).expect("deserialises");

        // What a renderer draws.
        assert_eq!(back.meshes, built.meshes, "geometry must survive");
        assert_eq!(back.rig, built.rig, "the rig must survive");
        assert_eq!(back.budget, built.budget);
        assert_eq!(back.skin.width, built.skin.width);
        assert_eq!(
            back.skin.albedo, built.skin.albedo,
            "the atlas must survive"
        );
        assert_eq!(back.skin.mip_level_count, built.skin.mip_level_count);
        // What it queries: eyes blink, and every attachment is fitted to the
        // measured surface.
        assert_eq!(back.parts.eyes, built.parts.eyes, "eyes must survive");
        assert_eq!(
            back.parts.surface, built.parts.surface,
            "the measured surface must survive"
        );
        assert_eq!(back.parts.handed, built.parts.handed);

        // And what it does not: the merge's own inputs are dropped, which is
        // the whole reason the payload is affordable.
        assert!(
            back.parts.body.positions.is_empty(),
            "the un-charted body mesh rode along and doubled the payload"
        );
        assert!(back.parts.zones.is_empty());
        assert!(back.parts.features.is_none());
    }

    /// A truncated atlas is refused where it can still be reported, rather
    /// than uploaded to a GPU that reads past the end of it.
    #[cfg(feature = "serde-avatar")]
    #[test]
    fn a_short_atlas_buffer_is_refused() {
        let mut record = AvatarRecord::new("Short", Archetype::default());
        record.reroll(1);
        let built = Avatar::build_with(
            &record,
            &AvatarConfig {
                atlas: 64,
                ..AvatarConfig::default()
            },
        )
        .expect("builds");
        let mut wire: serde_json::Value = serde_json::to_value(&built).expect("serialises");
        wire["skin"]["albedo"] = serde_json::json!([0u8, 1, 2, 3]);
        // Matched rather than `expect_err`: `Avatar` withholds `Debug` on
        // purpose, and the Ok arm here is the failure being tested for.
        match serde_json::from_value::<Avatar>(wire) {
            Ok(_) => panic!("a short atlas buffer was accepted"),
            Err(error) => assert!(
                error.to_string().contains("albedo"),
                "the refusal should name the buffer: {error}"
            ),
        }
    }
}
