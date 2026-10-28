# Quadruped morphology at rest: dog, cat, horse

*A point-in-time research dossier gathered for the quadruped refinement (#372, R0 #373, 2026-10-28). Kept as the record of what was found and why the refinement's decisions were made. Every figure carries its own confidence; a value recalled from a paywalled paper, or derived here, is marked so and needs checking before anything hard-codes it.*

Research for the symbios-avatar quadruped rig: bone lengths, rest angles, joint heights and trunk girth. The owner reports bodies that look too thin in the mid-section, so the trunk numbers come first.

Conventions used throughout:
- **WH** means withers height: ground to the top of the withers, standing square. Every length here is a fraction of WH unless the row says otherwise.
- **Interior angle** is the angle between two bone axes measured at the joint. A straight joint is 180 deg.
- **Flexion angle** is 180 minus the interior angle; 0 means straight.
- **Inclination** is the angle of one bone axis against the horizontal unless the row says otherwise.
- **Confidence**: H means a measured value from a peer-reviewed study read in this session. M means measured but with a caveat about method or sample, or a breed standard. L means a rule of thumb, a figure derived here from other rows (marked "derived"), or background textbook knowledge not verified in this session (marked "bg").
- **Source keys** (S1, S2, ...) point to section 4.

---

## 1. Summary: ten facts that most change how convincing a body looks

1. **The trunk carries about 62% of body mass.** In German Shepherds the thorax (C7/T1 to T13/L1) is 38% and the abdomen plus pelvis (T13/L1 to tail base) is 24%. Head plus neck is 14%, and each whole limb is only 4 to 7% (S8, H). A trunk modelled as a tube between the shoulders and hips looks starved.
2. **Heart girth against WH:** dog 1.07 to 1.15, horse 1.14 to 1.22, cat about 1.3 to 1.5. The cat is the roundest of the three (S7, S9, S3, S4, S14).
3. **Chest depth is about half of WH:** 0.45 to 0.50 in dogs and horses. The sternum floor and the elbow sit at about 0.50 to 0.52 WH, so the free leg below the chest is only half the animal's height (S4, S11, S6, S7).
4. **At the girth the chest cross-section is a tall ellipse.** Depth to width is about 2:1 in the horse and 1.3:1 to 2.5:1 in the dog (Labrador about 1.3:1 to 1.5:1, sighthound 2.5:1). The cat is close to round, about 1.1:1 to 1.3:1. An ellipse with the measured depth and width reproduces the measured girth to within about 3% (derived from S7 and S9).
5. **The waist varies widely by breed.**
   - Sighthound: waist girth is 0.72 x chest girth (S9).
   - Labrador: "little or no tuck-up" (S11).
   - Cat: the girth over L7 is *larger* than the chest girth (1.04 to 1.13x), so a cat has no dog-style tuck-up (S14).
6. **Trunk length against WH.**
   - Point of shoulder to point of buttock: 1.00 to 1.05 for Labrador, Malinois and riding horses, and 1.10 to 1.17 for the German Shepherd (S11, S12, S4).
   - Withers to the lumbosacral joint is 1.01 to 1.04 for Labrador, GSD, Bulldog and Border Collie (S10).
7. **Long-bone lengths.**
   - Dog: each long bone is about 0.30 to 0.34 WH (humerus 0.30, radius 0.31, femur 0.33, tibia 0.34) (S5).
   - Cat: the same bones are longer, 0.39 to 0.44 WH, because the cat stands more crouched and its hindlimb is longer than its forelimb (S2, S16).
   - Horse: the proximal bones are short (humerus 0.19, femur 0.22) and the distal ones long (tibia 0.32 to 0.34, cannon 0.145) (S4).
8. **Measured standing joint angles (interior).**
   - Dog: stifle 145 +/- 8 deg and hock 134 +/- 9 deg on radiographs (S6). Shoulder 107 deg (Labrador) and 120 deg (GSD) from skin markers (S13).
   - Horse: shoulder joint 100 to 105, elbow 133 to 137, hip 87 to 89, stifle 117 to 126 and hock 141 to 147 deg (S4). Holmstrom's hock values for Swedish Warmbloods are 155 to 159 deg (S20).
9. **The real scapula is steeper than "45 deg layback".** Measured horses: scapular spine 58 to 61 deg from horizontal (S4). Dogs: the shoulder interior angle measures 107 to 120 deg, against an "ideal" of 90 (S12, S13).
10. **A puppy is not a short-legged dog.** At 45 days, against WH:

    | Quantity | Puppy (45 d) | Adult |
    |---|---|---|
    | Head length | 0.61 | 0.39 |
    | Body length | 1.44 | 1.04 |
    | Chest girth | 1.58 | 1.15 |
    | Leg length (S7's own leg measure, definition not stated) | 0.61 | 0.61 |

    The leg fraction does not change with age (S7). The "cute" signal is a big head and a long, round barrel on normal legs. Shortened legs are a toy-design choice, not anatomy.

---

## 2. Tables per family

### 2.1 Dog

Reference breeds: Labrador (WH 55 to 62 cm, S11), Malinois and GSD (GSD 55 to 65 cm, S12), Beagle (33 to 41 cm, bg), Greyhound (66 to 76 cm, S15), Bulldog (about 31 to 38 cm, bg).

#### (a) Skeletal proportions

The zooarchaeological factors are the inverse of a bone's greatest length (GL) against WH, so bone/WH = 1/factor.

| Quantity | Value | Normalisation | Source | Conf. |
|---|---|---|---|---|
| Scapula GL | 0.246 (factor 4.06) | /WH | S5 (Lasota-Moskalewska factors, reproduced by Baranowski 2025) | M |
| Humerus GL | 0.297 (factor 3.37). Harcourt regression gives 0.305 for a 57 cm Labrador, 0.312 for a 37 cm Beagle and 0.302 for a 72 cm Greyhound | /WH | S5 (Harcourt 1974: WH = 3.43 x GL - 26.54 mm) | M |
| Radius GL | 0.311 (factor 3.22). Harcourt gives 0.304 (Labrador) and 0.298 (Beagle) | /WH | S5 (WH = 3.18 x GL + 19.51) | M |
| Ulna GL (includes the olecranon) | 0.375 (factor 2.67) | /WH | S5 | M |
| Femur GL | 0.332 (factor 3.01). Harcourt gives 0.326 (Labrador) and 0.330 (Beagle) | /WH | S5 (WH = 3.14 x GL - 12.96) | M |
| Tibia GL | 0.342 (factor 2.92). Harcourt gives 0.337 (Labrador) and 0.334 (Beagle) | /WH | S5 (WH = 2.92 x GL + 9.41) | M |
| Metacarpus ("pastern") | about 1/3 of forearm length, so about 0.10 | /WH | S12 (GSD standard) | M |
| Forefoot digits, standing height | about 0.04 to 0.06 | /WH | derived, bg | L |
| Metatarsus | about 0.15 to 0.18 ("hocks well let down" means short) | /WH | derived, bg | L |
| Hindfoot digits | about 0.04 to 0.05 | /WH | bg | L |
| Os coxae (pelvis) GL | about 0.27 to 0.30. Not measured in a dog source this session; the cat value is 0.32 | /WH | bg | L |
| Forelimb segment shares of total length (scapula + humerus + radius + metacarpus + digits) | 0.24 / 0.29 / 0.31 / 0.10 / 0.06 | /sum | derived from rows above | M-L |
| Hindlimb segment shares (femur + tibia + metatarsus + digits, without pelvis) | 0.38 / 0.39 / 0.18 / 0.06 | /sum | derived | M-L |
| Indices | brachial (radius/humerus) about 1.05; crural (tibia/femur) about 1.03; intermembral (humerus + radius)/(femur + tibia) about 0.90 | ratio | derived from S5 | M |
| Chondrodystrophic range | mean humerus 108 mm in chondrodystrophic dogs against 183 to 184 mm in others. Short legs come from short long bones; the trunk is unchanged | absolute | search summary of a radiographic humerus study (not read in full) | L-M |
| Vertebral formula | C7 T13 L7 S3 Cd20-23 | count | S17 | H |
| Regional share of the presacral column | C about 25%, T about 40%, L about 33%. Lumbar vertebrae are the longest; the T:L length ratio is about 1.2 to 1.5 | /presacral length | bg (estimated from typical vertebral body lengths; no measured source found) | L |
| Withers to lumbosacral (L7-S1) length | Labrador 1.03, GSD 1.04, Bulldog 1.04, Border Collie 1.01, Cavalier 1.10, Basset 1.40, Miniature Dachshund 1.51 to 1.66 | /WH | S10 (Packer 2013, n = 700) | H |
| Head length | 0.40 (GSD, standard), 0.39 (Taigan sighthound, measured), 0.39 (Kars, 12 months) | /WH | S12, S9, S7 | H/M |
| Head length against body length | about 0.37 | /point of shoulder to point of buttock | derived | M |
| Skull, standard proportions | Labrador skull and foreface of "approximately equal length"; GSD standard has head length about 40% of WH | - | S11, S12 | M |

#### (b) Standing posture

| Quantity | Value | Convention | Source | Conf. |
|---|---|---|---|---|
| Shoulder (scapulohumeral) | Labrador 107 +/- 9, GSD 120 +/- 9 | interior. Skin markers: scapular spine top, greater tubercle, lateral epicondyle | S13 (flexion 73.2 and 59.8 converted) | H (markers) |
| Shoulder, breed standard | GSD "ideal 90, as a rule 110"; Labrador "approximately 90" | interior | S12, S11 | M (an ideal, not measured) |
| Scapula slope | about 55 to 65 from horizontal. The "45 deg layback" is a show-ring ideal | inclination of the spine | bg, consistent with S13 shoulder angles and the horse data in S4 | L |
| Elbow | 130 to 145 (the arthrodesis recommendation of 130 to 140 is derived from standing angles). Elbows sit "directly under the withers" | interior | S18 (VCOT 2003 retrospective, snippet); S11 | M |
| Carpus | 190 to 200 dorsal angle (10 to 20 deg hyperextended). The metacarpus is 20 to 22 deg from the forearm line in the GSD; Labrador pasterns "slope slightly" | dorsal interior; deviation from the forearm axis | S12, S11; bg | M |
| Metacarpophalangeal joint | hyperextended; changes significantly with weight-bearing (S19) | - | S19 snippet | M |
| Pelvis | croup about 23 deg (GSD standard). The S1 to greater trochanter line is 66 +/- 9 deg in the Labrador and 79 +/- 8 in the GSD | from horizontal | S12; S13 | M; H (markers) |
| Hip | anatomical (ilium axis to femur) about 95 to 110 (bg, L). The marker angle S1-GT-LFC is 150 (Labrador) and 190 (GSD) | interior | bg; S13 | L; H (markers, not anatomical) |
| Stifle | 145.3 +/- 7.9 (range 129.5 to 156.6), on mechanical axes, 21 dogs of 12 breeds. The older figure of 135 came from two crossbreed groups. Markers give 148.5 (Labrador) and 136 (GSD) | interior | S6; S13 | H |
| Stifle, GSD standard | upper and lower thigh "form an angle of approximately 120 deg" | interior | S12 | M |
| Hock (tibiotarsal) | 134 +/- 9.1 (range 122.5 to 147.7), radiographs. Markers give 154 (Labrador) and 122.5 (GSD) | interior | S6; S13 | H |
| Metatarsus | "vertical from the hock joint" (GSD). In the radiograph protocol the paw lands directly below the hip. Labrador rear toes are "slightly behind the point of the rump" | from vertical | S12, S6, S11 | M |
| Elbow height | 0.50 WH ("elbow to ground equals one half of the height at withers"); the brisket reaches the elbow | /WH | S11 | M |
| Point of shoulder height | about 0.65 to 0.72 | /WH | derived from scapula and humerus angles | L |
| Hip joint height | about 0.72 to 0.80 | /WH | derived; cat value in S21 is higher | L |
| Stifle position | under the flank fold, at about 0.45 to 0.52 WH, forward of the hip joint | /WH | derived, bg | L |
| Hock (point of calcaneus) height | about 0.25 to 0.30 | /WH | derived, bg | L |
| Foot posture | digitigrade: stands on the digital pads of digits II to V plus the metacarpal (or metatarsal) pad. The carpal pad is off the ground. Digit I (dewclaw) does not touch | - | bg | M |
| Forelimb attachment | no bony attachment to the trunk. The serratus ventralis is a sling hanging the trunk from the tops of the forelimbs (synsarcosis). The clavicle is absent or a rod of about 1 cm inside the brachiocephalicus | - | S22, S23 | H |
| Weight distribution, standing | forelimbs 69 +/- 5% (Labrador), 62 +/- 2% (GSD) | % body weight | S13 | H |
| Neck carriage | about 45 deg to the horizontal trunk (GSD) | inclination | S12 | M |

#### (c) Trunk shape

| Quantity | Value | Normalisation | Source | Conf. |
|---|---|---|---|---|
| Chest depth | Labrador 0.50 (brisket to elbow); GSD 0.45 to 0.48; Taigan 0.47; Kars at 12 months 0.38 (not fully mature) | /WH | S11, S12, S9, S7 | H/M |
| Chest width | Taigan 0.19; Kars (12 months) 0.29. Labrador "moderately wide", "neither slab-sided nor barrel" | /WH | S9, S7, S11 | H |
| Chest girth | Taigan 1.07; Kars 1.15 | /WH | S9, S7 | H |
| Waist girth (narrowest point) | Taigan 0.77 WH, which is 0.72 x chest girth | /WH; /girth | S9 | H |
| Tuck-up by breed | Labrador: underline "almost straight, little or no tuck-up". Greyhound: "well cut up in the flanks". Bulldog: belly tucked up under a very broad chest (bg). Suggested waist/chest-girth ratios: Labrador about 0.85 to 0.9, sighthound 0.65 to 0.75 | ratio | S11, S15, S9; values outside the Taigan row are L | L-M |
| Segment mass shares (GSD) | thorax 38.1%, abdomen plus pelvis 24.2%, head 7.7%, neck 6.6%, tail 0.8%. Per limb: brachium 2.4%, antebrachium 1.4%, manus 0.7%, thigh 4.5%, crus 1.5%, pes 0.8% | % body mass | S8 (6 cadavers) | H |
| Ribcage shape | the spring of ribs widens from a narrow forechest between the elbows to a maximum at about ribs 7 to 10, then tapers to the last rib (T13). Labrador: "tapering between the front legs" | qualitative | S11; bg | M |
| Topline | Labrador: level withers to croup. GSD: sloping, with a croup of about 23 deg. Rump height about equal to WH (Taigan 1.007, Kars 0.99) | /WH | S11, S12, S9, S7 | H/M |
| Body length (point of shoulder to point of buttock) | Labrador 1.00 to 1.05; GSD 1.10 to 1.17; Taigan 0.99; Kars 1.04; Greyhound about 1.0 to 1.1 (bg) | /WH | S11, S12, S9, S7 | H/M |
| Range breeds | Bulldog back length 1.04 WH (as long as a Labrador relative to height, but much wider). Basset 1.40, Dachshund 1.5 to 1.66 | /WH | S10 | H |

### 2.2 Cat (domestic shorthair)

WH about 25 to 27 cm: 24.7 +/- 2.4 cm in females and 27.1 +/- 2.2 cm in males, 37 radiographed cats (S2).

#### (a) Skeletal proportions

| Quantity | Value | Normalisation | Source | Conf. |
|---|---|---|---|---|
| Humerus GL | 10.06 +/- 0.70 cm, which is 0.387 | /WH (25.98 cm) | S2 | H |
| Radius GL | 9.74 +/- 0.67 cm, 0.375 | /WH | S2 | H |
| Ulna GL | 11.43 +/- 0.72 cm, 0.440 | /WH | S2 | H |
| Os coxae GL | 8.43 +/- 0.50 cm, 0.324 | /WH | S2 | H |
| Femur GL | 10.81 +/- 0.83 cm, 0.416 | /WH | S2 | H |
| Tibia GL | 11.38 +/- 0.65 cm, 0.438 | /WH | S2 | H |
| Calcaneus GL (heel lever) | 2.87 cm, 0.110 | /WH | S2 | H |
| Talus GL | 1.63 cm, 0.063 | /WH | S2 | H |
| Scapula | about 0.28 to 0.32 | /WH | bg | L |
| Metacarpus III | about 0.11 to 0.13 | /WH | bg | L |
| Metatarsus III | about 0.20 to 0.23. A long metatarsus puts the hock high | /WH | bg | L |
| Indices (n = 55) | intermembral 89.8 +/- 3.2; humerofemoral 94.4 +/- 5.5; brachial 96.7 +/- 3.5; crural 106.6 +/- 6.6. The hindlimb is longer than the forelimb | x100 | S16 (Kartal 2026; agrees with S2) | H |
| Vertebral formula | C7 T13 L7 S3 Cd18-24 (often 20 to 23) | count | S17 | H |
| Regional share of the presacral column | C about 22%, T about 41%, L about 37%. The lumbar region is relatively longer and more flexible than the dog's | /presacral | bg (estimated) | L |
| Head length (nose to occiput) | about 0.35 to 0.40 | /WH | bg | L |
| Face (frontal) | face width / head length 0.98 (adult) and 1.15 (kitten); forehead/head length 0.56 and 0.59; nose length/head length 0.29 and 0.23 | ratios | S24 (20 photographs) | M |

#### (b) Standing posture

| Quantity | Value | Convention | Source | Conf. |
|---|---|---|---|---|
| Hip height in normal walking | about 23 cm, about 0.88 WH. In crouched walking it drops 30% to 16 cm, with the trunk held level | /WH | S21 (8 cats, treadmill) | M |
| Rump against withers | rump at or above withers (Van cat: rump 28.8 cm, shoulder 28.0 cm), because the hindlimb is longer | /WH | S25 snippet | M |
| Joint angles at mid-stance | hip about 100 to 115, stifle about 110 to 125, hock about 110 to 130, elbow about 120 to 140. A cat stands more flexed than a dog; this is why its bones are longer for the same WH | interior | bg (Goslow et al. 1973 walking-cat kinematics); no quiet-stance study found | L |
| Foot posture | digitigrade, on digital and metacarpal/metatarsal pads. Claws are retracted at rest. The forefoot is compact and round | - | bg | M |
| Forelimb attachment | clavicle present but vestigial: a slender bone of about 2 cm inside the brachiocephalicus, with no bony articulation. The scapula slides on the thorax in a muscular sling | - | S23 | H |
| Hock (point of calcaneus) height | about 0.30 to 0.35 | /WH | derived (long metatarsus) | L |

#### (c) Trunk shape

| Quantity | Value | Normalisation | Source | Conf. |
|---|---|---|---|---|
| Nose to sacrococcygeal joint, along the back | 47 to 51 cm, about 1.8 to 1.95 | /WH | S14 (with WH from S2) | M |
| Thoracic circumference at the 7th intercostal space | 34.0 to 35.6 cm after a 20% weight loss (BCS 6 to 6.7); 39.6 to 41.3 cm when obese | absolute; about 1.3 to 1.4 /WH lean | S14 | M |
| Pelvic circumference over the last lumbar vertebra | 36.3 to 36.9 cm (same cats), which is 1.04 to 1.07 x thoracic; 1.07 to 1.13x when obese | ratio | S14 | M |
| Breast girth (Angora) | 34.6 cm (female) and 39 cm (male) at WH 22.6 and 23.2 cm, so 1.53 to 1.68 | /WH | search summary of an Ankara Zoo Angora study | L-M |
| Cross-section | close to round at the girth (depth:width about 1.1:1 to 1.3:1). Narrow between the forelimbs, so the feet track near the midline | ratio | derived from S14 girth, bg | L |
| Belly | primordial pouch: a loose ventral fold of skin and fat between the last ribs and the stifles. It swings in gait and is present in lean cats too | qualitative | bg | M |
| Body length (point of shoulder to point of buttock) | about 1.3 to 1.5 (Van cat 36.7 / 28.0 = 1.31; definition unconfirmed) | /WH | S25 snippet | L |
| Topline | level to rising toward the rump; very mobile lumbar arch | qualitative | S16, S25 | M |

### 2.3 Horse

Reference values come from S4 (Matsuura 2008, 35 horses, photo landmarks after Holmstrom). Groups: TN (Thoroughbred-type, narrow, WH 158 cm), TW (tall and wide, Quarter Horse, Warmblood-like, WH 164 cm) and SN (small, 137 cm). The pony range comes from S3 (Terceira Pony, WH 127 cm, zoometric stick and tape). These are functional segment lengths between joint landmarks on the live horse, which is what a rig wants, not dry-bone GL.

#### (a) Skeletal proportions

| Quantity | TN 158 cm | TW 164 cm | SN 137 cm | Pony 127 cm | Normalisation | Source | Conf. |
|---|---|---|---|---|---|---|---|
| Scapula (spine top to greater tubercle) | 0.311 | 0.328 | 0.337 | 0.398 | /WH | S4, S3 | H (landmarks) |
| Humerus | 0.189 | 0.190 | 0.186 | 0.190 | /WH | S4, S3 | H |
| Radius (elbow to distal radius) | 0.242 | 0.246 | 0.235 | 0.256 | /WH | S4, S3 | H |
| Metacarpus (cannon) | 0.146 | 0.145 | 0.147 | 0.154 | /WH | S4, S3 | H |
| Front phalanges (pastern, landmark) | 0.061 | 0.061 | 0.053 | 0.084 | /WH | S4, S3 | M (definitions differ) |
| Pelvis (ilium landmark to trochanter) | 0.226 | 0.225 | 0.219 | 0.218 | /WH | S4, S3 | M |
| Femur | 0.221 | 0.217 | 0.247 | 0.243 | /WH | S4, S3 | M (femur correlates only r = 0.44 with WH) |
| Tibia | 0.316 | 0.337 | 0.327 | 0.299 | /WH | S4, S3 | H |
| Metatarsus (hind cannon) | 0.182 | 0.195 | 0.181 | 0.207 | /WH | S4, S3 | H |
| Hind phalanges | 0.059 | 0.058 | 0.056 | 0.084 | /WH | S4, S3 | M |

Further horse rows:

| Quantity | Value | Normalisation | Source | Conf. |
|---|---|---|---|---|
| Forelimb shares (TN): scapula / humerus / radius / cannon / phalanx | 0.33 / 0.20 / 0.25 / 0.15 / 0.06 | /sum | derived from S4 | H |
| Hindlimb shares (TN, without pelvis): femur / tibia / metatarsus / phalanx | 0.28 / 0.41 / 0.23 / 0.08 | /sum | derived from S4 | H |
| Dry-bone factors (Kiesewalter 1888) | humerus 4.87 (0.205), radius 4.34 (0.230), ulna 3.4, femur 3.51 (0.285); recalled, not verified: MC III 6.41 (0.156), MT III 5.33 (0.188), tibia 4.36 (0.229) | WH = factor x GL | S26 (Whitby table snippet) | M; L for the recalled ones |
| Cursorial proportion | proximal bones short, metapodials long, so a factor estimate is biased by build | qualitative | S27 (Eisenmann) | M |
| Indices | brachial (radius/humerus) about 1.28; crural (tibia/femur) about 1.4 | ratio | derived from S4 | M |
| Vertebral formula | C7 T18 L6(5) S5 Cd15-21 | count | S17 | H |
| Regional share of the presacral column | C about 35 to 40%, T about 43 to 48%, L about 15 to 18%. Long neck, long and stiff thorax, short loin | /presacral | bg (estimated) | L |
| Head length | 0.39 (pony 50 cm / 127 cm); riding horse about 0.37 to 0.40 | /WH | S3; bg | M |
| Neck length | 0.53 (pony) | /WH | S3 | M |
| Foal growth | about 60% of mature WH at birth. Cannon bones add only about 20% length after birth, while the humerus and femur roughly double. Thoroughbreds reach 84% of mature WH at 6 months and 94% at 12 months | ratio | search summary of farrier-school and Hintz/KER sources | L-M |

#### (b) Standing posture

| Quantity | Value | Convention | Source | Conf. |
|---|---|---|---|---|
| Scapula (shoulder inclination) | TN 60.8 +/- 3.7; TW 58.0 +/- 5.2; SN 61.1 +/- 2.4 | inclination of the spine-top to greater-tubercle line from horizontal | S4 | H |
| Scapula, extension and judging figure | about 45 to 55 (the "45 deg shoulder") | line drawn differently, often against the vertical; do not mix with the row above | S28 snippets | L |
| Shoulder joint | 100 to 105 (TN 104.7, TW 101.9) | interior | S4 | H |
| Elbow | 133 to 137 | interior | S4 | H |
| Fetlock, front | dorsal interior about 140 to 145 when the hoof-pastern axis is 50 to 55 deg to the ground, so hyperextended by about 35 to 40 deg. S4 reports "front fetlock 62 to 66" with an unstated convention | interior (derived) | derived; S4 | M (derived) |
| Hoof-pastern axis | front about 50 to 55; hind about 53 to 58; pony hoof wall 49 to 53 | from horizontal | S3; bg | M |
| Croup | 20.6 to 25.4 (pony) | from horizontal | S3 | M |
| Pelvis (tuber coxae to tuber ischii) | about 25 to 35 | from horizontal | bg | L |
| Hip | 87 to 89 | interior (pelvis line to femur line) | S4 | H |
| Femur inclination | about 60 | from horizontal | S4 | H |
| Stifle | 117 to 126 | interior | S4 | H (SD up to 13) |
| Hock | photo markers: 141 to 147 (TN 146, TW 147, SN 141). Holmstrom, Swedish Warmbloods: 4-year-olds 155.4 +/- 2.6, elite 159 +/- 3, lame 157 | interior | S4; S20 | H (the method changes the value by about 10 deg) |
| Elbow height | about 0.52 (sub-sternal height = WH - chest depth: TN 0.523, pony 0.570) | /WH | S4, S3 | H |
| Point of shoulder height | about 0.62 to 0.66 | /WH | derived (elbow + humerus x sin 40 deg) | L-M |
| Carpus (knee) height | about 0.27 to 0.29 | /WH | derived (cannon + pastern + hoof) | L-M |
| Hip joint height | about 0.68 to 0.75 | /WH | derived | L |
| Stifle height | about 0.52 to 0.58, at the bottom of the flank, level with the belly line | /WH | derived | L |
| Hock (point of calcaneus) height | about 0.36 to 0.40; the point of the hock sits above the level of the knee | /WH | derived (MT + phalanx + hoof + calcaneal lever) | L-M |
| Foot posture | unguligrade: stands on the hoof wall and sole around P3 (single digit III). The fetlock (metacarpophalangeal) joint is held in hyperextension by the suspensory apparatus. The chestnut and ergot are vestiges | - | bg | H |
| Forelimb attachment | no clavicle. The thoracic sling (serratus ventralis plus pectorals) suspends the trunk between the scapulae; the dorsoscapular ligament anchors the top. Synsarcosis | - | S22, S29, S30 | H |
| Weight distribution | about 55 to 60% on the forelimbs | % body weight | bg | M |

#### (c) Trunk shape

| Quantity | Value | Normalisation | Source | Conf. |
|---|---|---|---|---|
| Chest depth (withers to sternum) | TN 0.477, TW 0.478, SN 0.488, pony 0.430 | /WH | S4, S3 | H |
| Heart girth (behind withers and elbows) | TN 1.155, TW 1.217, SN 1.208, pony 1.142. Correlation with WH r = 0.93 (Quarter Horse) | /WH | S4, S3, S31 | H |
| Chest width at the girth | pony 0.227 (28.9 cm). An ellipse solve from TN depth and girth gives about 0.24 | /WH | S3; derived | M |
| Hip width (tuber coxae) | pony 0.298; thurls 0.28 | /WH | S3 | M |
| Body length (point of shoulder to point of buttock) | TN 1.015, TW 1.048, SN 1.024, pony 1.00 ("square") | /WH | S4, S3 | H |
| Barrel | the thorax is narrow and keel-like between the forelimbs (the girth point is the narrowest ribbed section). It widens behind the girth to the widest point at about ribs 14 to 16 (barrel width about 0.30 to 0.35 WH, bg). The belly line rises gently from the girth to the stifle, with no tuck-up in a fit horse | qualitative | bg | L-M |
| Topline | withers about equal to croup (pony 127.1 / 127.4). The back dips 4% below the withers (pony back height 0.96) | /WH | S3 | M |
| Weight predictor | girth^2 x length explains 90% of body weight | r^2 | S32 | H |

---

## 3. Notes: what the numbers do not capture

### The animal as a whole

1. **Joint angles cannot be pasted in independently.** Stifle 145, hock 134 and a vertical metatarsus together would force the femur to point backwards, which is impossible. Real standing poses close a loop: the paw lands under the hip (S6) and the forelimb is vertical under the withers (S11). A rig should take segment lengths, the paw's ground position, the hip and shoulder heights and one or two characteristic angles, then solve the rest. Use the measured angles as checks, not inputs.
2. **The withers is not the top of a bone hinge.** The withers is the top of the T1 to T3 spinous processes plus the scapular cartilage. The trunk *hangs* between the two scapulae in the serratus sling. When the animal settles, sits or lies, the thorax sinks between the shoulder blades, so the withers drops relative to the scapulae and the scapulae rise relative to the spine. A rig that welds the scapula to the spine will push the chest through the ground in Lie Down. In sternal recumbency the lowest rigid surface is the sternum and the ventral ribcage; the withers then stands about one chest depth (0.45 to 0.5 WH) plus fur above the ground. This is the morphological floor for the sit and lie postures.
3. **The trunk sets the outline, not the skeleton.** Muscle mass sits proximally:
   - triceps and shoulder muscles fill the angle between scapula and humerus;
   - the hamstrings run from the ischium to the tibia, so the buttock is round, not a bone triangle;
   - pectorals make the forechest that projects in front of the point of the shoulder.

   Below the carpus and hock the limb is tendon and bone, about cannon-circumference thin (horse front cannon circumference is 0.12 to 0.13 WH, S3, S4). A thin-looking body usually comes from a missing forechest, a missing triceps mass behind the elbow and a missing thigh, not only from girth.

### Trunk shape along its length

4. **The girth and the cross-section change along the trunk; this is the "too thin" fix.**
   - Neck base: about 0.45 to 0.5 x the chest girth (Taigan neck circumference 34.2 cm against chest 69.3 cm, S9).
   - Ribcage: a tall ellipse that widens to its maximum about 2/3 along the ribs.
   - Last rib: the depth falls.
   - Loin: depth falls to about 0.6 to 0.7 of chest depth in a dog, with the width held by the epaxial muscles and the flank fold.
   - Pelvis: width comes back at the hips; in the horse the hips are as wide as or wider than the chest.
   - The belly line runs from the sternum up toward the stifle fold. A cat's belly hangs (primordial pouch); a horse's is gently convex.
   - A rig should give at least four cross-sections (forechest, mid-thorax, last rib, loin) plus the pelvis, each a depth and a width.
5. **Breed range comes mainly from width and length, not depth.** A Bulldog and a Labrador have the same back length relative to WH (1.04, S10) and similar chest depth relative to WH. The Bulldog is far wider (chest width is perhaps 0.5 WH or more, bg) and its legs splay. Sighthounds keep the depth but lose the width (chest width 0.19 WH) and gain the tuck-up. Variability sliders should drive trunk width and waist ratio first.

### Limbs and posture

6. **The zig-zag is the species signature.** In every therian quadruped the scapula and the forearm lean one way while the humerus leans the other, and in the hindlimb the femur and metatarsus lean one way while the tibia leans the other. The scapula is functionally the uppermost limb segment. Cats hold the zig-zag more folded than dogs and horses (longer bones, same WH). Horses hold it most open distally: radius and cannon vertical in a straight column.
7. **Horse anatomical names mislead.** The horse's "knee" is the carpus (wrist), its "hock" is the ankle, and its "fetlock" is the knuckle (MCP joint). The true knee (stifle) sits up under the flank, close to the belly. Showing a stifle low on the leg is a common reason a stylised horse reads wrong.
8. **Cat-specific points.** The hindlimb is longer than the forelimb (intermembral index 0.90), so a standing cat's back is level or rises to the rump. The scapulae sit high and close on a narrow forechest; they visibly ride above the spine line at each step. A cat's long lumbar region lets it arch, so its outline changes more between poses than a dog's.
9. **Skin-marker angles differ from bone angles by 10 deg or more.** S6 (radiographs) and S13 (markers) disagree on the dog hock (134 against 122 to 154). S4 (photo markers) and S20 (Holmstrom) disagree on the horse hock (141 to 147 against 155 to 159). Use radiographic or bone-axis values for a skeleton rig, and remember that a skinned mesh will read about 10 deg differently from its bones.

### Stylising without losing believability

10. **Cute but believable.**
    - The measured juvenile shifts (S7, S24) are a safe envelope:
      - head 0.45 to 0.6 WH (adult 0.39);
      - body length 1.1 to 1.4 WH;
      - chest girth 1.3 to 1.6 WH;
      - rounder face (face width +10 to 17%), higher forehead (+5 to 11%) and shorter muzzle (nose length -20%), all within +/-2 SD of real animals, which is the bound S24 used "to maintain normal facial appearance".
    - Character-design guidance (S33) adds: round, soft forms; eyes and ears set low on the skull and far apart; short, plump legs; head up to half the body length.
    - Recommended policy: shorten the *visible* lower legs, the radius and cannon or metatarsus, by up to 20 to 30%. Keep the proximal segments, which are hidden inside the trunk outline. Keep the joint order and the zig-zag. Keep the digitigrade heel clear of the ground (hock at least about 0.2 WH up for a dog-like animal). Keep the elbow at the chest floor.
    - Enlarge paws 1.2 to 1.5x rather than shortening the toes.
    - A foal or kitten is leggy, not stubby (foal cannons are near adult length at birth). Only the puppy-like and toy styles shorten legs; label that a style choice in the parameters.
11. **Crosses.** For a cross, interpolate *normalised* values (fractions of WH), not absolute lengths. Interpolate foot posture categorically: digitigrade with pads, or unguligrade with a hoof. A hoof on a cat-length metatarsus reads as wrong to most viewers. Head and neck length are relatively independent of the limb set, so they can come from either parent.

---

## 4. Sources

Each entry gives author, year, title and URL, plus how the source was used. "Snippet" means only a search-engine summary was seen. "bg" is background textbook knowledge, chiefly:
- Evans and de Lahunta, *Miller's Anatomy of the Dog*;
- Dyce, Sack and Wensing, *Textbook of Veterinary Anatomy*;
- Goslow, Reinking and Stuart 1973, "The cat step cycle", J Morphol 141.

None of these was read in this session.

### Bone lengths and skeletal proportions

- **S2** - Poradowski D, Mutlu Z, Altundag Y, Chroszcz A, et al. 2024. "Estimation of Height at Withers Based on Long Bone Measurements of Living Cats." Vet Sci 11(11):522. https://www.ebi.ac.uk/europepmc/webservices/rest/PMC11598937/fullTextXML (PMC11598937). Read: tables 1 to 4.
- **S3** - Lopes MS, Mendonca D, Rojer H, Cabral V, Bettencourt SX, da Camara Machado A. 2015. "Morphological and genetic characterization of an emerging Azorean horse breed: the Terceira Pony." Front Genet 6:62. https://www.ebi.ac.uk/europepmc/webservices/rest/PMC4343030/fullTextXML (PMC4343030). Read: table 1.
- **S4** - Matsuura A, Ohta E, Ueda K, Nakatsuji H, Kondo S. 2008. "Influence of Equine Conformation on Rider Oscillation and Evaluation of Horses for Therapeutic Riding." J Equine Sci 19:9. https://www.ebi.ac.uk/europepmc/webservices/rest/PMC4019204/fullTextXML (PMC4019204). Read: methods (landmarks a-n, after Holmstrom) and tables 1 and 2.
- **S5** - Baranowski P. 2025. "Morphological and Metric Analysis of Medieval Dog Remains from Wolin, Poland." Animals 15:2171. https://www.ebi.ac.uk/europepmc/webservices/rest/PMC12345418/fullTextXML (PMC12345418). Read: table 8, which reproduces Harcourt 1974 (J Archaeol Sci) regressions and the Lasota-Moskalewska factors.
- **S16** - Kartal et al. 2026. "Long bone ratios in the thoracic and pelvic limbs of domestic cats (Felis catus)." Med Weter 82(2):78-83. https://www.medycynawet.edu.pl/archives/summary/995-summary-2026/996-summary-022026/7104-summary-med-weter-82-02-78-83-2026. Snippet only (the PDF was unreadable).
- **S17** - Vertebral formulae: veterinary table in "Transitional vertebrae in domestic mammals" (2024). https://pmc.ncbi.nlm.nih.gov/articles/PMC11148713/. Snippet only, consistent with bg.
- **S26** - Kiesewalter 1888 factors via Boessneck and von den Driesch 1974, in a Whitby horse report. https://archaeologydataservice.ac.uk/catalogue/adsdata/arch-1642-1/dissemination/pdf/SNY12041_part2of3.pdf. Snippet only.
- **S27** - Eisenmann V. "Withers height estimations." https://vera-eisenmann.com/withers-height-estimations. Snippet only.

### Joint angles and standing posture

- **S6** - Giansetto T, Picavet PP, Lefebvre M, Balligand M. 2022. "Determination of the Stifle Angle at Standing Position in Dogs." Vet Sci 9(11):644. https://www.ebi.ac.uk/europepmc/webservices/rest/PMC9697634/fullTextXML (PMC9697634). Read in full.
- **S13** - Humphries A, Shaheen AF, Gomez Alvarez CB. 2020. "Biomechanical comparison of standing posture and during trot between German shepherd and Labrador retriever dogs." PLoS One 15:e0239832. https://www.ebi.ac.uk/europepmc/webservices/rest/PMC7531786/fullTextXML (PMC7531786). Read: abstract, methods and results text. The S5 segment-length table is supplementary and was not read.
- **S18** - Elbow arthrodesis retrospective (12 dogs), Vet Comp Orthop Traumatol 2003;16(4):250-254. https://thieme-connect.de/products/all/doi/10.1055/s-0038-1632787. Snippet.
- **S19** - Weight-bearing and joint angles in dogs, VCOT (Thieme). https://thieme-connect.de/products/ejournals/pdf/10.1055/s-0038-1632787.pdf. Snippet.
- **S20** - Holmstrom M, Magnusson LE, Philipsson J. 1990. Swedish Warmblood conformation (hock angles), as cited in a VetTimes review, 2017. https://vettimes.com/news/vets/equine/hindlimb-conformation-and-lameness-in-sports-horses. Snippet.
- **S21** - Treadmill study of normal against crouched walking in 8 cats (hip height 23 cm to 16 cm). https://link.springer.com/doi/10.1007/BF00241355. Snippet.
- **S28** - Extension conformation guides (shoulder 45 to 55 deg): Univ. Minnesota https://extension.umn.edu/horse-care-and-management/conformation-horse ; UGA B1400 https://secure.caes.uga.edu/extension/publications/files/pdf/B%201400_2.PDF ; UT W1324 https://utia.tennessee.edu/publications/wp-content/uploads/sites/269/2025/10/W1324.pdf. Snippets.
- Milgram J, Slonim E, Kass PH, Shahar R. 2004. "A radiographic study of joint angles in standing dogs." VCOT 17:82-90. https://cris.huji.ac.il/en/publications/a-radiographic-study-of-joint-angles-in-standing-dogs/. Abstract only (the full table is paywalled). It reports that most standing angles do not vary with body weight. This is the reference to buy if exact dog values are needed.

### Trunk shape, body measurements and mass distribution

- **S7** - Bektasoglu F, Kirmizibayrak T. 2025. "Fertility and Growth Traits, Some Body Measurements, and Morphological Characteristics of Kars Shepherd Dogs." Vet Med Sci 11:e70660. https://www.ebi.ac.uk/europepmc/webservices/rest/PMC12535301/fullTextXML (PMC12535301). Read: table 5 (growth from 45 days to 12 months).
- **S8** - Jones et al. 2018. "Inertial properties of the German Shepherd Dog." PLoS One 13:e0206037. https://www.ebi.ac.uk/europepmc/webservices/rest/PMC6195294/fullTextXML (PMC6195294). Read: table 1 (segment mass fractions) and tables 3 to 5.
- **S9** - Ari HH, Tas A, Sen I, et al. 2025. "Determination of Some Morphological Characteristics of the Kyrgyz Taigan Dog Breed." Vet Med Sci 11:e70409. https://www.ebi.ac.uk/europepmc/webservices/rest/PMC12077753/fullTextXML (PMC12077753). Read: tables.
- **S10** - Packer RMA, Hendricks A, Volk HA, Shihab NK, Burn CC. 2013. "How long and low can you go? Effect of conformation on the risk of thoracolumbar intervertebral disc extrusion in domestic dogs." PLoS One 8:e69650. https://www.ebi.ac.uk/europepmc/webservices/rest/PMC3722130/fullTextXML (PMC3722130). Read: methods and table 3.
- **S14** - Borges NC, Vasconcellos RS, Carciofi AC, et al. 2012. "DXA, bioelectrical impedance, ultrasonography and biometry for the estimation of fat and lean mass in cats during weight loss." BMC Vet Res 8:111. https://www.ebi.ac.uk/europepmc/webservices/rest/PMC3413556/fullTextXML (PMC3413556). Read: table 1 and methods.
- **S25** - Van cat body measurements (Yuzuncu Yil University). https://gcris.yyu.edu.tr/entities/publication/bbe5faff-8137-4481-b0a7-a8cfc6b16c5c. Snippet; the page was bot-walled.
- **S31** - Heird JC. "Growth parameters in the quarter horse" (MSc thesis). https://trace.tennessee.edu/utk_gradthes/8344. Snippet.
- **S32** - Body condition scoring and weight estimation of horses. Equine Vet J 1988. https://pubmed.ncbi.nlm.nih.gov/3366105/. Snippet.

### Breed standards

- **S11** - AKC Labrador Retriever standard (1994), copy at https://caccia.se/labrador_am_standard.htm. Read in full.
- **S12** - FCI standard No. 166, German Shepherd Dog, copy at https://gsdcp.org/the-breed/breeding-standard. Read: proportions, angles, chest depth 45 to 48%, croup 23 deg.
- **S15** - Greyhound standards (AKC, KC, FCI 158) via https://greyhoundclubofamericainc.org/akc-standard and https://thekennelclub.org.uk/breed-standards/hound/greyhound. Snippet.

### Forelimb attachment

- **S22** - University of Minnesota veterinary anatomy, ungulate dissection lab 1. https://vanat.ahc.umn.edu/ungDissect/Lab01/Lab01.html. Snippet: no bony attachment; the serratus ventralis is the weight-bearing sling.
- **S23** - Clavicle in Carnivora (Souza et al. 2019, https://gwern.net/doc/cat/biology/2019-souza.pdf) and IMAIOS vet-anatomy "clavicle" (https://www.imaios.cn/en/vet-anatomy/anatomical-structures/clavicle-11073897868). Snippets.
- **S29** - "Equine reciprocating systems: examining the shoulder-to-thorax junction." American Farriers Journal. https://www.americanfarriers.com/articles/11488-equine-reciprocating-systems-examining-the-shoulder-to-thorax-junction. Snippet.
- **S30** - WikiVet, "Extrinsic Musculature of the Thoracic Limb - Horse Anatomy" (synsarcosis). https://en.wikivet.net/Extrinsic_Musculature_of_the_Thoracic_Limb_-_Horse_Anatomy. Snippet.
- Fischer MS, Schilling N, Schmidt M, Haarhaus D, Witte H. 2002. "Basic limb kinematics of small therian mammals." J Exp Biol 205:1315. https://cob.silverchair.com/jeb/article-pdf/1243956/1315.pdf. Snippet: the scapula is the proximal limb segment and the main pivot. Also Fischer and Blickhan 2006, J Exp Zool 305A:935 (tri-segmented limbs). Snippet.

### Cuteness and character design

- **S24** - Borgi M, Cogliati-Dezza I, Brelsford V, Meints K, Cirulli F. 2014. "Baby schema in human and animal faces induces cuteness perception and gaze allocation in children." Front Psychol 5:411. https://www.ebi.ac.uk/europepmc/webservices/rest/PMC4019884/fullTextXML (PMC4019884). Read: table 1 (facial ratios for adult dog, puppy, adult cat and kitten) and the +/-2 SD manipulation rule.
- **S33** - Preuss S. 2010. "The Elements of Cute Character Design." Envato Tuts+. https://design.tutsplus.com/articles/the-elements-of-cute-character-design--vector-3533. Read.
