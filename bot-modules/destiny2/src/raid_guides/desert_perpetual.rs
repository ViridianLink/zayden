use super::{EncounterGuide, RaidGuide};

pub(super) const DESERT_PERPETUAL: RaidGuide<'_, 4> =
    RaidGuide::new("Desert Perpetual").add_encounter(OPENING).add_encounter(IATROS);

const OPENING: EncounterGuide<'_> =
    EncounterGuide::new("Opening Encounter").guide(OPENING_GUIDE);
const OPENING_GUIDE: &str = "The Purple Crystal Tower in the centre island lets you choose which encounter to complete, first second and third.
- \"Axion is axiom\" - The Living Rhythm Encounter (Wyvern boss)
- \"Interference patterns swell\" - The Inverse Function Encounter (Hobgoblin boss)
- \"All are entangled\" - The Clear Sight Encounter (Hydra boss)
Once you've selected an encounter head towards the large spire.";

const IATROS: EncounterGuide<'_> =
    EncounterGuide::new("Iatros, Inward Turned").guide(IATROS_GUIDE);
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
