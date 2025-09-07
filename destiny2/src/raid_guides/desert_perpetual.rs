use super::{EncounterGuide, RaidGuide};

pub const DESERT_PERPETUAL: RaidGuide<4> = RaidGuide::new("Desert Perpetual")
    .add_encounter(OPENING)
    .add_encounter(IATROS);

const OPENING: EncounterGuide = EncounterGuide::new("Opening Encounter").guide(OPENING_GUIDE);
const OPENING_GUIDE: &str = "The Purple Crystal Tower in the centre island lets you choose which encounter to complete, first second and third.
- \"Axion is axiom\" - The Living Rhythm Encounter (Wyvern boss)
- \"Interference patterns swell\" - The Inverse Function Encounter (Hobgoblin boss)
- \"All are entangled\" - The Clear Sight Encounter (Hydra boss)
Once you've selected an encounter head towards the large spire.";

const IATROS: EncounterGuide = EncounterGuide::new("Iatros, Inward Turned").guide(IATROS_GUIDE);
const IATROS_GUIDE: &str = "1. Split into 3 teams.
  - 2 people on minotaurs and dunking time
  - 3 people on harpy ad clear and pillars
  - 1 person climbing
2. At the start of the encounter, everyone needs to spread out and clear ads, focusing on the 6 minotaurs in the centre. 
3. The 6 minotaurs drop different types of 'Chronal Energy'.
  - White = Spawn plate
  - Blue = Middle plate
  - Red = Bottom left plate
4. Deposit the 'Chronal Energy' by interacting with it and running through the corrosponding hoop from inside the arena to out.
5. Continue dunking until the hourglass is full.
6. Shoot the vex diamond under the hoop to begin the next phase";

/*
 Guide1:
Before encounter, assign 3 players to a differing platform and pillar

    Clear ads, kill minotaurs

    All players grab 1-2 chronons each

    Players with chronons jump through hoop to deposit chronons and move to assigned platforms

    On diastole’s 4th tick, all players on platforms shoot box at same elevation on assigned pillar to lock (“Iatros loses control of phase space”)

    Player with platforms spawned jumps up and interacts with item

    Repeat 3 times

    Damage

Go top to bottom shooting ring indicated elevation last:

-White: Top
-Blue: Mid
-Red: Bottom

i.e. if red you go top mid bottom, if blue you go bottom top mid, if white you go mid bottom top. No conformation if this is required, but seems logical.

Guide2:
Iatros, Inward Turned

3 Teams:
- 2 people on minotaurs and dunking time
- 3 people on harpy ad clear and pillars
- 1 person climbing

Beginning of Encounter (Everyone doing the same thing)

    Everyone is clearing ads and focusing on the 6 minotaurs that spawn in the middle. These drop chronal energy of one of 3 types. The hoop color also mirrors this energy type.

- White is spawn plate.
- Blue is middle plate.
- Red is bottom left of spawn plate.

2. You can dunk the energy by interacting with it and running through the hoop from inside the arena to out. Continue dunking time and avoiding boss until the hourglass is full.

3. Shoot the vex diamond under the hoop to begin the next phase.

Main Mechanic Phase (Teams)

Climber

    Before this phase starts, go to the plate indicated by the color of the buff. When the phase starts, you must run through the floating shape on the plate to get a buff.

    Tell your team to shoot the first level of cubes. When the platforms spawn, begin climbing up.

    On the 3rd platform, there will be a cube that you need to interact with. Tell your team that you have interacted and to shoot the next level.

    Repeat this 2 more times until you reach the third cube and start damage phase. Boss will teleport to the plate you started at.

Harpy and Pillars

    When ready, shoot the bottom cube on the pillars on the 4th beat of diastole (there is a very short leeway before and after the boop).

    Clear ads and imps near the climber until they reach the cube.

    After they call out that they have interacted, shoot the middle cube on the pillars at the same time.

    Repeat a total of 3x until damage begins.

Minotaurs and Dunkers

    Ad clear around the arena focusing on minotaurs. Continue dunking time so mechanics can be done. (Minotaurs appear to spawn where the boss is heading)

    Make sure there is no wipe timer after damage starts. If there is, someone needs to dunk a buff.


*/
