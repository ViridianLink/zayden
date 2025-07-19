use super::{EncounterGuide, RaidGuide, Weapon};

const GUIDE: &str = "1. Scattered throughout the arena are 9 plates each with their own corresponding symbol above them
2. In the middle of the arena there will be 6 symbols present
3. The goal here is to capture the plates with the matching symbols to the ones in the middle to unlock the doors during DPS in order to survive the white mechanic
4. Stand on the matching plates until a Knight spawns and kill the Knight
5. Group up in the middle and deal damage to Kalli
6. Hide in the safe rooms below the boss to avoid wiping. Max 1 person per room";

const KALLI_LEGIT: EncounterGuide = EncounterGuide::new("Kalli, the Corrupted (Legit)")
    .video_timestamp(99)
    .guide(GUIDE)
    .add_weapon(Weapon::LordOfWolves)
    .add_weapon(Weapon::Queenbreaker);

pub const LAST_WISH: RaidGuide<6> = RaidGuide::new("Last Wish")
    .video("https://www.youtube.com/watch?v=aBfVlF4X_Rw")
    .add_encounter(KALLI_LEGIT);
