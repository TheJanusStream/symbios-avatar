//! Winged bodies' surfaces (#423): each family is painted from its own
//! defaults, a stored plumage or coat paints the body it belongs to, a crow is
//! dark and glossy, a parrot bright and an owl's disc paler than its ruff; the
//! flight feathers take the plumage the skin does; a rolled bird wears the
//! plumage of the kind of bird its head is drawn as; a record and a share code
//! from 0.21, which store no surface, read as their families' defaults, and a
//! surface rides a code only when it needs to; a change of family keeps both surfaces; and an unrolled
//! winged body looks out of its own family's eyes.

use symbios_avatar::plan::{Blended, BodyPlan, Lineage, Pigment};
use symbios_avatar::texture::coat_colour;
use symbios_avatar::{
    Archetype, Avatar, AvatarConfig, AvatarRecord, Coat, CoatChannel, EyeParams, Influence,
    Plumage, QuadrupedParams, SkinWeights, Vec3, WingedFamily, WingedParams, Zone,
};

/// The atlas the surfaces are read off: fine enough for an owl's ruff, which
/// is a few texels wide at the default's.
const ATLAS: u32 = 256;

/// The body built on [`ATLAS`].
fn built(params: WingedParams) -> Avatar {
    let record = AvatarRecord::new("Surfaces", Archetype::Winged(params));
    Avatar::build_with(
        &record,
        &AvatarConfig {
            atlas: ATLAS,
            ..AvatarConfig::default()
        },
    )
    .expect("a winged body builds")
}

/// A bird with its head's axes `head` and its plumage `plumage`.
fn bird(head: [f32; 3], plumage: Plumage) -> WingedParams {
    let mut params = WingedParams::default();
    [params.beak_length, params.beak_hook, params.facial_disc] = head;
    params.plumage = plumage;
    params
}

/// The body of `lineage` in its own surfaces.
fn of(lineage: Lineage<WingedFamily>) -> WingedParams {
    let mut params = WingedParams::default();
    params.set_lineage(lineage);
    params
}

/// The crow's head and the parrot's (docs/winged-anatomy.md "The axes").
const CROW_HEAD: [f32; 3] = [1.0, -1.0, -1.0];
const PARROT_HEAD: [f32; 3] = [0.3, 1.0, -1.0];

/// One vertex of the drawn body as the atlas paints it.
struct Painted {
    at: Vec3,
    normal: Vec3,
    zone: Zone,
    colour: Vec3,
    roughness: f32,
}

/// Every charted vertex of `avatar`'s body, with the texel its chart
/// coordinate lands on; vertices on an unpainted texel - a chart's edge - are
/// left out.
fn painted(avatar: &Avatar) -> Vec<Painted> {
    let body = &avatar.parts.body;
    let normals = body.vertex_normals();
    let unwrap = &avatar.parts.unwrap;
    let map = &avatar.skin;
    let (width, height) = (map.width as usize, map.height as usize);
    unwrap
        .uvs
        .iter()
        .zip(&unwrap.source)
        .filter_map(|(uv, &source)| {
            let x = ((uv.x * width as f32) as usize).min(width - 1);
            let y = ((uv.y * height as f32) as usize).min(height - 1);
            let at = (y * width + x) * 4;
            (map.albedo[at + 3] == 255).then(|| Painted {
                at: body.positions[source as usize],
                normal: normals[source as usize],
                zone: avatar.parts.zones[source as usize],
                colour: Vec3::new(
                    f32::from(map.albedo[at]),
                    f32::from(map.albedo[at + 1]),
                    f32::from(map.albedo[at + 2]),
                ) / 255.0,
                roughness: f32::from(map.roughness[at + 1]) / 255.0,
            })
        })
        .collect()
}

/// The trunk's dorsal axis: square to the spine from the hips to the withers,
/// toward the sky - the back of a perching bird, which faces up and behind.
fn dorsal(avatar: &Avatar) -> Vec3 {
    let rig = &avatar.rig;
    let hips = rig.joints[rig.in_zone(Zone::Pelvis)[0]].position;
    let neck = rig.in_zone(Zone::Neck)[0];
    let withers = rig.joints[rig.joints[neck].parent.expect("a girdle")].position;
    let spine = (withers - hips).normalize();
    (Vec3::Y - spine * spine.y).normalize()
}

/// The trunk's vertices facing its back (`side` 1) or its belly (`side` -1):
/// the back behind the shoulders, where a fruit bat's mantle does not reach
/// (it lies over the neck and the shoulders, #423), and the whole belly.
fn trunk_side(avatar: &Avatar, side: f32) -> Vec<Painted> {
    let dorsal = dorsal(avatar);
    painted(avatar)
        .into_iter()
        .filter(|vertex| match vertex.zone {
            Zone::Pelvis | Zone::Abdomen => true,
            Zone::Chest => side < 0.0,
            _ => false,
        })
        .filter(|vertex| vertex.normal.dot(dorsal) * side > 0.7)
        .collect()
}

/// The median of each channel.
fn median(colours: impl Iterator<Item = Vec3>) -> Vec3 {
    let mut channels = [Vec::new(), Vec::new(), Vec::new()];
    for colour in colours {
        for (channel, value) in channels.iter_mut().zip(colour.to_array()) {
            channel.push(value);
        }
    }
    let middle = |values: &mut Vec<f32>| {
        assert!(!values.is_empty(), "nothing to take a median of");
        values.sort_by(f32::total_cmp);
        values[values.len() / 2]
    };
    Vec3::new(
        middle(&mut channels[0]),
        middle(&mut channels[1]),
        middle(&mut channels[2]),
    )
}

/// The median of a set of values.
fn middle(mut values: Vec<f32>) -> f32 {
    assert!(!values.is_empty(), "nothing to take a median of");
    values.sort_by(f32::total_cmp);
    values[values.len() / 2]
}

fn luminance(colour: Vec3) -> f32 {
    colour.dot(Vec3::new(0.2126, 0.7152, 0.0722))
}

fn near(a: Vec3, b: Vec3, within: f32) -> bool {
    (a - b).abs().max_element() <= within
}

/// The median colour of the back and of the belly.
fn sides(avatar: &Avatar) -> (Vec3, Vec3) {
    (
        median(trunk_side(avatar, 1.0).into_iter().map(|v| v.colour)),
        median(trunk_side(avatar, -1.0).into_iter().map(|v| v.colour)),
    )
}

#[test]
fn each_family_wears_its_own_surface_from_its_defaults() {
    // A bird in a little owl's plumage, brown above and buff below; a bat in
    // its family's near-black and a glider in its blue-grey, neither storing
    // a coat. Painted with W1's stand-in coats, the bird's back is a pale
    // grizzle and the glider a dark grizzle (a bat's stand-in was its black
    // already, so its clause holds the default rather than tells it apart).
    let (back, belly) = sides(&built(WingedParams::default()));
    let owl = Plumage::default();
    assert!(
        near(back, Vec3::from_array(owl.upper), 0.07),
        "an owl's back {back} against its upper {:?}",
        owl.upper
    );
    assert!(
        near(belly, Vec3::from_array(owl.under), 0.07),
        "an owl's breast {belly} against its under {:?}",
        owl.under
    );
    for family in [WingedFamily::Bat, WingedFamily::Glider] {
        let params = of(Lineage::pure(family));
        assert_eq!(params.coat, None, "{family:?} stores no coat of its own");
        let (back, _) = sides(&built(params));
        let dark = Vec3::from_array(coat_colour(&family.default_coat(), CoatChannel::Dark));
        assert!(
            near(back, dark, 0.07),
            "{family:?}'s back {back} against its family's coat {dark}"
        );
    }
}

#[test]
fn a_stored_coat_paints_a_bat_and_not_a_bird() {
    // `WingedParams::coat` reads the stored coat for a bat- or glider-led body
    // and a bird's own for a bird-led one, whose feathers are its plumage.
    let black = Coat {
        pigment: Pigment::Black,
        ..Coat::default()
    };
    let mut bat = of(Lineage::pure(WingedFamily::Bat));
    bat.coat = Some(black);
    let (back, _) = sides(&built(bat));
    let want = Vec3::from_array(coat_colour(&black, CoatChannel::Dark));
    assert!(near(back, want, 0.05), "a bat in a black coat: {back}");
    let mut owl = WingedParams {
        coat: Some(black),
        ..WingedParams::default()
    };
    owl.sanitize();
    let (back, _) = sides(&built(owl));
    assert!(
        near(back, Vec3::from_array(Plumage::default().upper), 0.07),
        "a bird carrying a black coat is still feathered: {back}"
    );
}

#[test]
fn a_crow_is_dark_and_glossy_a_parrot_bright_and_an_owl_s_disc_paler_than_its_ruff() {
    let owl = built(WingedParams::default());
    let crow = built(bird(CROW_HEAD, Plumage::CROW));
    let parrot = built(bird(PARROT_HEAD, Plumage::PARROT));

    // Dark all over, and its back glossed: smoother than an owl's.
    let body = |avatar: &Avatar| {
        painted(avatar)
            .into_iter()
            .filter(|vertex| matches!(vertex.zone, Zone::Pelvis | Zone::Abdomen | Zone::Chest))
            .collect::<Vec<_>>()
    };
    let dark = middle(body(&crow).iter().map(|v| luminance(v.colour)).collect());
    assert!(dark < 0.2, "a crow's body reads {dark}");
    let rough = |avatar: &Avatar| {
        middle(
            trunk_side(avatar, 1.0)
                .iter()
                .map(|v| v.roughness)
                .collect(),
        )
    };
    let (glossed, matte) = (rough(&crow), rough(&owl));
    assert!(
        glossed < matte - 0.2,
        "a crow's back is {glossed} rough, an owl's {matte}"
    );

    // Bright: far from grey on most of its body.
    let vivid = middle(
        body(&parrot)
            .iter()
            .map(|v| v.colour.max_element() - v.colour.min_element())
            .collect(),
    );
    assert!(vivid > 0.45, "a parrot's colour stands {vivid} off grey");

    // An owl's face inside its ruff, and the ruff, read off the head it
    // declared: the face pale, the rim round it dark.
    let head = AvatarRecord::new("Owl", Archetype::Winged(WingedParams::default()))
        .skeleton()
        .beast_head
        .expect("a winged body declares a beast's head");
    let (mut face, mut ruff, mut crown) = (Vec::new(), Vec::new(), Vec::new());
    for vertex in painted(&owl).iter().filter(|v| v.zone == Zone::Head) {
        let local = head.local(vertex.at);
        let (Some(dish), Some(off)) = (head.in_dish(local), head.on_ruff(local)) else {
            panic!("an owl's head has a disc");
        };
        if dish > 0.95 && off > 2.5 {
            face.push(luminance(vertex.colour));
        } else if off < 1.1 {
            ruff.push(luminance(vertex.colour));
        } else if dish < 0.05 && off > 3.0 {
            crown.push(luminance(vertex.colour));
        }
    }
    let (face, ruff, crown) = (middle(face), middle(ruff), middle(crown));
    assert!(
        face > ruff + 0.25,
        "an owl's disc reads {face} and its ruff {ruff}"
    );
    // And the ruff is a rim darker than the head round it, not the head's
    // own brown: what outlines the disc.
    assert!(
        ruff < crown - 0.07,
        "an owl's ruff reads {ruff} and its crown {crown}"
    );
}

#[test]
fn the_flight_feathers_take_the_plumage() {
    // Coloured by the plumage's sampler, which W2's stand-in brown never was:
    // a parrot's flight feathers blue among its red coverts, a crow's every
    // card dark, an owl's none blue.
    let feathers = |params: WingedParams| -> Vec<Vec3> {
        let avatar = built(params);
        let wings = avatar.parts.wings.as_ref().expect("a bird grows wings");
        assert!(!wings.feathers.colours.is_empty(), "the cards are coloured");
        wings.feathers.colours.clone()
    };
    let share = |colours: &[Vec3], test: fn(Vec3) -> bool| {
        colours.iter().filter(|&&colour| test(colour)).count() as f32 / colours.len() as f32
    };
    let blue = |colour: Vec3| colour.z > colour.x + 0.3;
    let red = |colour: Vec3| colour.x > colour.z + 0.3;
    let parrot = feathers(bird(PARROT_HEAD, Plumage::PARROT));
    assert!(share(&parrot, blue) > 0.10, "{} blue", share(&parrot, blue));
    assert!(share(&parrot, red) > 0.5, "{} red", share(&parrot, red));
    let crow = feathers(bird(CROW_HEAD, Plumage::CROW));
    let lightest = crow.iter().map(|&c| luminance(c)).fold(0.0f32, f32::max);
    assert!(lightest < 0.25, "a crow's palest feather reads {lightest}");
    let owl = feathers(WingedParams::default());
    assert_eq!(share(&owl, blue), 0.0, "an owl has no blue feather");
}

/// Whether `plumage` is `palette` a little lighter or darker all over, as a
/// roll draws one.
fn drawn_from(plumage: &Plumage, palette: &Plumage) -> bool {
    let pairs = |p: &Plumage| {
        [
            p.head, p.upper, p.under, p.flight, p.tail, p.mark, p.disc, p.ruff,
        ]
    };
    let (asked, laid) = (pairs(plumage), pairs(palette));
    // The shade off the brightest channel a lighter shade did not clip.
    let brightest = (0..8)
        .flat_map(|colour| (0..3).map(move |part| (colour, part)))
        .filter(|&(colour, part)| asked[colour][part] < 0.995)
        .max_by(|a, b| laid[a.0][a.1].total_cmp(&laid[b.0][b.1]))
        .expect("a channel short of white");
    let shade = asked[brightest.0][brightest.1] / laid[brightest.0][brightest.1];
    (0.89..=1.11).contains(&shade)
        && plumage.pattern == palette.pattern
        && asked.iter().zip(&laid).all(|(a, l)| {
            a.iter()
                .zip(l)
                .all(|(a, l)| (a - (l * shade).min(1.0)).abs() < 0.004)
        })
}

#[test]
fn a_rolled_bird_wears_the_plumage_of_the_kind_its_head_is_drawn_as() {
    // The kind a roll drew the head about is not stored: the plumage roll asks
    // the head roll's streams again, so a parrot's beak comes with a parrot's
    // colours and a crow's with a crow's, and what a creator is handed reads
    // the kind back off the head - an owl's with a disc, else the disc-less
    // kind its beak is nearest. Drawn about the owl alone, every rolled bird
    // would wear an owl's.
    let mut kinds = [0usize; 4];
    let presets = [
        Plumage::OWL,
        Plumage::PARROT,
        Plumage::CROW,
        Plumage::SONGBIRD,
    ];
    let (mut birds, mut read_back) = (0usize, 0usize);
    for seed in 0..400 {
        let record = AvatarRecord::rolled("Roll", Archetype::Winged(WingedParams::default()), seed);
        let Archetype::Winged(params) = record.archetype else {
            panic!("still winged");
        };
        if params.lineage().leading() != WingedFamily::Bird {
            // A bat or a glider draws a coat and keeps the plumage it carries.
            assert!(params.coat.is_some(), "seed {seed}: a mammal rolls a coat");
            assert_eq!(params.plumage, Plumage::default(), "seed {seed}");
            continue;
        }
        assert_eq!(params.coat, None, "seed {seed}: a bird rolls no coat");
        birds += 1;
        // The kind whose palettes the plumage was drawn from.
        let kind = (0..4)
            .find(|&kind| {
                palettes_of(kind)
                    .iter()
                    .any(|palette| drawn_from(&params.plumage, palette))
            })
            .unwrap_or_else(|| panic!("seed {seed}: wears {:?}", params.plumage));
        kinds[kind] += 1;
        // What a creator is handed for the head: the same kind, but for a
        // head the roll drew nearer another kind's.
        let palettes = params.plumage_palettes();
        if palettes[0] == presets[kind] {
            read_back += 1;
        }
        if params.facial_disc > -0.75 {
            assert_eq!(
                kind, 0,
                "seed {seed}: an owl with a disc wears another kind's"
            );
            assert_eq!(palettes[0], Plumage::OWL, "seed {seed}");
        }
    }
    assert!(
        kinds.iter().all(|&count| count >= 3),
        "every kind of bird came up: {kinds:?}"
    );
    assert!(
        read_back * 100 >= birds * 95,
        "{read_back} of {birds} heads read back as the kind their plumage is"
    );
    // And the palettes are the head's: a parrot-beaked body's are a parrot's.
    assert_eq!(
        bird(PARROT_HEAD, Plumage::OWL).plumage_palettes()[0],
        Plumage::PARROT
    );
    assert_eq!(
        bird(CROW_HEAD, Plumage::OWL).plumage_for_head(),
        Plumage::CROW
    );
}

/// The palettes of the kind of bird `kind` counts, in an owl, a parrot, a
/// crow and a songbird's order: those a creator handed that kind's head is.
fn palettes_of(kind: usize) -> &'static [Plumage] {
    let heads = [[0.0, 0.0, 0.0], PARROT_HEAD, CROW_HEAD, [0.2, -1.3, -1.0]];
    bird(heads[kind], Plumage::OWL).plumage_palettes()
}

/// The record below's share code, written by the published 0.21.0 (#423): a
/// bat-led cross with a crow's head, version 13, its head appended and no
/// surfaces.
const CODE_V13: &str = "1M1VR-0M0G2-08105-A0404-V040A-MMNK0-3K02N-5ANCP";

/// The default bird's code, written by the published 0.21.0: version 10,
/// nothing appended.
const CODE_V10: &str = "181NG-0M0G2-08104-003ZG-1040A-MMNK0-3K02V-0";

#[test]
fn a_0_21_record_and_code_read_as_their_family_s_defaults_fields_absent() {
    // A bat-led cross as 0.21.0 wrote it: no plumage, no coat, both fields
    // absent and read as the family's defaults. Not as the stand-ins 0.21
    // painted: a glider's default and a bird's plumage are colours 0.21 never
    // showed (docs/winged-anatomy.md "The wire").
    let json = r#"{"$type":"network.symbios.avatar.defs#winged","size":700,"wingLength":0,
        "neckLength":0,"legLength":0,"tailLength":0,"headSize":0,"appeal":1000,
        "beakLength":1000,"beakHook":-1000,"facialDisc":-1000,"family":"bat",
        "hybrid":"bird","blend":300}"#;
    let Archetype::Winged(read) = serde_json::from_str::<Archetype>(json).expect("reads") else {
        panic!("still winged");
    };
    assert_eq!(read.plumage, Plumage::default(), "{read:?}");
    assert_eq!(read.coat, None, "{read:?}");
    // And written back, it is written as it was: neither surface is a key.
    let written = serde_json::to_value(Archetype::Winged(read)).expect("writes");
    for key in ["plumage", "coat"] {
        assert!(written.get(key).is_none(), "{key} written: {written}");
    }
    // Its code is 0.21.0's, byte for byte, and so is the default bird's.
    let code = AvatarRecord::new("Old", Archetype::Winged(read)).share_code();
    assert_eq!(
        code, CODE_V13,
        "a body in its family's surfaces moved its code"
    );
    let owl = AvatarRecord::new("Owl", Archetype::Winged(WingedParams::default())).share_code();
    assert_eq!(owl, CODE_V10, "the default bird's code moved");
    for old in [CODE_V13, CODE_V10] {
        let back = AvatarRecord::from_share_code("Old", old).expect("a 0.21 code decodes");
        let Archetype::Winged(back) = back.archetype else {
            panic!("still winged");
        };
        assert_eq!(back.plumage, Plumage::default());
        assert_eq!(back.coat, None);
    }
}

#[test]
fn a_surface_rides_a_code_only_when_it_needs_to() {
    // A body storing exactly its family's own coat writes it (#431): the same
    // look as storing none while a bat leads, but a stored coat goes with the
    // body across a change of family, and one read back as none wore the
    // glider's grey made a glider where the bat it was read from wore black.
    let bat = of(Lineage::pure(WingedFamily::Bat));
    let code =
        |params: WingedParams| AvatarRecord::new("Code", Archetype::Winged(params)).share_code();
    let own = WingedParams {
        coat: Some(WingedFamily::Bat.default_coat()),
        ..bat
    };
    assert_ne!(code(own), code(bat), "a stored coat dropped from its code");
    let back = AvatarRecord::from_share_code("Back", &code(own)).expect("decodes");
    let Archetype::Winged(mut back) = back.archetype else {
        panic!("still winged");
    };
    // Read back a byte a span off, as every code is: the bat's own coat.
    let read = back.coat.expect("the stored coat read back");
    let bat_own = WingedFamily::Bat.default_coat();
    assert_eq!(
        (read.pigment, read.dilute, read.agouti, read.colours),
        (
            bat_own.pigment,
            bat_own.dilute,
            bat_own.agouti,
            bat_own.colours
        ),
        "{read:?}"
    );
    assert_eq!(code(back), code(own), "and written again as it was read");
    // Made a glider, both still wear the bat's black, not the glider's grey.
    back.set_kind(WingedFamily::Glider, None, 0.0);
    let read = back.coat.expect("the coat goes with the body");
    assert!(!read.dilute && read.colours.dark.is_none(), "{read:?}");
    // A body storing none writes none, as 0.21 wrote it.
    let read = AvatarRecord::from_share_code("Back", &code(bat)).expect("decodes");
    let Archetype::Winged(read) = read.archetype else {
        panic!("still winged");
    };
    assert_eq!(read.coat, None);
    // Anything else does, and comes back: a crow's plumage on a bird, a black
    // coat with a picked light on a bat.
    let crow = bird(CROW_HEAD, Plumage::CROW);
    let back = AvatarRecord::from_share_code("Back", &code(crow)).expect("decodes");
    let Archetype::Winged(back) = back.archetype else {
        panic!("still winged");
    };
    assert_eq!(back.plumage.pattern, Plumage::CROW.pattern);
    assert!((back.plumage.gloss - Plumage::CROW.gloss).abs() < 0.005);
    for (a, b) in back.plumage.flight.iter().zip(Plumage::CROW.flight) {
        assert!((a - b).abs() < 0.003, "{:?}", back.plumage);
    }
    assert!((back.beak_length - 1.0).abs() < 0.03, "its head rides too");
    let mut coat = Coat {
        pigment: Pigment::Black,
        marks: 4242,
        ..Coat::default()
    };
    coat.colours.light = Some([0.9, 0.5, 0.1]);
    let mut dressed = WingedParams {
        coat: Some(coat),
        ..bat
    };
    dressed.sanitize();
    assert_ne!(code(dressed), code(bat));
    let back = AvatarRecord::from_share_code("Back", &code(dressed)).expect("decodes");
    let Archetype::Winged(back) = back.archetype else {
        panic!("still winged");
    };
    let read = back.coat.expect("the coat rides the code");
    assert_eq!((read.pigment, read.marks), (Pigment::Black, 4242));
    let light = read.colours.light.expect("its picked colour too");
    assert!((light[0] - 0.9).abs() < 0.003, "{light:?}");
    // And the record writes integers, as the protocol has no floats.
    let record = AvatarRecord::new(
        "Wire",
        Archetype::Winged(WingedParams {
            plumage: Plumage::CROW,
            ..dressed
        }),
    );
    let json = serde_json::to_string(&record).expect("writes");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parses");
    let mut stack = vec![value];
    while let Some(value) = stack.pop() {
        match value {
            serde_json::Value::Number(number) => {
                assert!(
                    number.is_i64() || number.is_u64(),
                    "a float: {number} in {json}"
                );
            }
            serde_json::Value::Array(items) => stack.extend(items),
            serde_json::Value::Object(map) => stack.extend(map.into_iter().map(|(_, v)| v)),
            _ => {}
        }
    }
}

#[test]
fn a_change_of_family_keeps_both_surfaces() {
    // The rule (`WingedParams`'s `hand_over`): a plumage rides a bat or a
    // glider unseen and comes back with the bird; a stored coat goes with the
    // body, and an absent one reads as whichever family leads.
    let mut params = bird(CROW_HEAD, Plumage::CROW);
    params.set_kind(WingedFamily::Bat, None, 0.0);
    assert_eq!(params.plumage, Plumage::CROW, "a crow made a bat");
    assert_eq!(params.coat, None, "and wears a bat's own coat");
    let black = Coat {
        pigment: Pigment::Black,
        ..Coat::default()
    };
    params.coat = Some(black);
    params.set_kind(WingedFamily::Glider, Some(WingedFamily::Bird), 0.4);
    assert_eq!(
        params.coat,
        Some(black),
        "a black bat made a glider-led cross"
    );
    params.set_kind(WingedFamily::Bird, None, 0.0);
    assert_eq!(params.plumage, Plumage::CROW, "and a crow again");
    assert_eq!(params.coat, Some(black), "carrying its coat unseen");
}

#[test]
fn an_unrolled_winged_body_looks_out_of_its_family_s_eyes() {
    // An owl's gold, a bat's and a glider's dark: the plan's own iris, where
    // every winged record's eyes were a person's blue until rolled. A person's
    // and a beast's stay exactly what they were.
    let person = EyeParams::default();
    for family in WingedFamily::ALL {
        let archetype = Archetype::Winged(of(Lineage::pure(family)));
        let eyes = AvatarRecord::new("Eyes", archetype.clone()).eyes;
        let iris = Vec3::from_array(archetype.iris(0.33));
        assert!(
            near(Vec3::from_array(eyes.inner), iris, 0.002),
            "{family:?}: {:?} against its iris {iris}",
            eyes.inner
        );
        assert_ne!(eyes.inner, person.inner, "{family:?} wears a person's eyes");
        assert!(luminance(Vec3::from_array(eyes.ring)) < luminance(iris));
    }
    let gold = Vec3::from_array(
        AvatarRecord::new("Owl", Archetype::Winged(WingedParams::default()))
            .eyes
            .inner,
    );
    assert!(gold.x > gold.z + 0.4, "an owl's eyes are gold: {gold}");
    for archetype in [
        Archetype::default(),
        Archetype::Quadruped(QuadrupedParams::default()),
    ] {
        assert_eq!(AvatarRecord::new("Same", archetype).eyes, person);
    }
}

/// **Each kind of bird looks out of its own eyes** (the owner's call at W4's
/// review): an owl's gold, a parrot's pale, a crow's and a songbird's dark
/// brown, read off the head its record carries.
#[test]
fn each_kind_of_bird_has_its_own_eyes() {
    let eyes = |head: [f32; 3]| {
        let mut params = WingedParams::default();
        [params.beak_length, params.beak_hook, params.facial_disc] = head;
        Vec3::from_array(
            AvatarRecord::new("Bird", Archetype::Winged(params))
                .eyes
                .inner,
        )
    };
    let owl = eyes([0.0, 0.0, 0.0]);
    let parrot = eyes([0.3, 1.0, -1.0]);
    let crow = eyes([1.0, -1.0, -1.0]);
    let songbird = eyes([0.2, -1.3, -1.0]);
    assert!(owl.x > owl.z + 0.4, "an owl's eyes are gold: {owl}");
    for (name, dark) in [("crow", crow), ("songbird", songbird)] {
        assert!(luminance(dark) < 0.2, "a {name}'s eyes are dark: {dark}");
    }
    assert!(
        luminance(parrot) > luminance(owl) + 0.1,
        "a parrot's eyes are paler than an owl's: {parrot} against {owl}"
    );
}

/// **A sugar glider is a mid grey** (the owner's call at W4's review): its
/// family's diluted black read nearly white, so its default coat picks a mid
/// grey, lightening toward the belly as every coat does.
#[test]
fn a_glider_wears_a_mid_grey_coat() {
    let (back, belly) = sides(&built(of(Lineage::pure(WingedFamily::Glider))));
    assert!(
        (0.2..0.4).contains(&luminance(back)),
        "a glider's back is a mid grey: {back}"
    );
    assert!(
        luminance(belly) > luminance(back),
        "a glider's belly is lighter than its back: {belly} against {back}"
    );
}

/// **A bird's rump is its trunk's, not its tail's** (#426): a bird's bony
/// tail is a stub of markers under its rump that draws nothing of its own -
/// its visible tail is the fan - so no skin the pelvis holds is zoned the
/// tail, however much of the rump it holds, on an owl, a crow, a songbird and
/// a cross a bird leads, and no vertex of their bodies as built; a bat's and a
/// glider's tails are drawn, and the pelvis still hands the skin over their
/// docks to them, where a stored coat paints a tail's colour.
///
/// Read with every body vertex held by the pelvis alone, as well as as
/// built: a built bird's rump is held by its abdomen, whose bone runs to no
/// tail, so as built the handover reaches none of these birds today, and the
/// zone map is what any binding of their rump is read through. Before, the
/// pelvis's skin past about 74% of the way to a marker dock was handed to the
/// tail: held by the pelvis, 20 vertices of a bird-led cross's rump (a pure
/// bird's dock is a short drop under its pelvis, which none lies along).
#[test]
fn a_birds_rump_is_its_trunks_not_its_tails() {
    use WingedFamily::{Bat, Bird, Glider};
    // How many of the body's vertices are zoned the tail as built, and with
    // every one of them held by the pelvis, of how many.
    let tail = |params: WingedParams| -> (usize, usize, usize) {
        let record = AvatarRecord::new("Rump", Archetype::Winged(params));
        let avatar = Avatar::build_for_shape(&record).expect("a winged body builds");
        let (rig, parts) = (&avatar.rig, &avatar.parts);
        let pelvis = u16::try_from(rig.in_zone(Zone::Pelvis)[0]).expect("a joint");
        let held = SkinWeights {
            vertices: vec![
                [
                    Influence {
                        joint: pelvis,
                        weight: 1.0,
                    },
                    Influence::default(),
                    Influence::default(),
                    Influence::default(),
                ];
                parts.body.positions.len()
            ],
        };
        let tails = |zones: &[Zone]| zones.iter().filter(|&&zone| zone == Zone::Tail).count();
        (
            tails(&parts.zones),
            tails(&held.zone_map(&parts.body, rig)),
            parts.body.positions.len(),
        )
    };
    let bird_led = of(Lineage::cross(Glider, Bird, 0.7));
    for (name, params) in [
        ("owl", of(Lineage::pure(Bird))),
        ("crow", bird(CROW_HEAD, Plumage::default())),
        ("songbird", bird([0.2, -1.3, -1.0], Plumage::default())),
        ("glider x bird 0.7", bird_led),
    ] {
        let (built, held, all) = tail(params);
        assert_eq!(
            (built, held),
            (0, 0),
            "a {name}'s body zones its tail's {built} of {all} vertices as built, {held} held \
             by the pelvis"
        );
    }
    for (name, family) in [("bat", Bat), ("glider", Glider)] {
        let (built, held, all) = tail(of(Lineage::pure(family)));
        assert!(
            built > all / 100 && held > 0,
            "a {name}'s drawn tail keeps its zone: {built} of {all} vertices as built, {held} \
             held by the pelvis"
        );
    }
}
