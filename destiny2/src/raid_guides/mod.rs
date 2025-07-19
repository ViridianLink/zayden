pub mod last_wish;
pub mod weapons;

use last_wish::LAST_WISH;
use serenity::all::{
    ButtonStyle, CommandInteraction, CreateActionRow, CreateButton, CreateCommand, CreateComponent,
    CreateContainer, CreateFile, CreateInteractionResponse, CreateInteractionResponseMessage,
    CreateMediaGallery, CreateMediaGalleryItem, CreateSection, CreateSectionAccessory,
    CreateSectionComponent, CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption,
    CreateSeparator, CreateTextDisplay, CreateThumbnail, CreateUnfurledMediaItem, Http,
    MessageFlags, Permissions,
};
pub use weapons::Weapon;

#[derive(Clone, Copy)]
pub struct EncounterGuide<'a> {
    raid: &'a str,
    encounter: &'a str,
    video: Option<&'a str>,
    video_timestamp: Option<u8>,
    guide: &'a str,
    weapons: [Option<Weapon>; 2],
    armour: [Option<&'a str>; 1],
}

impl<'a> EncounterGuide<'a> {
    const fn new(encounter: &'a str) -> Self {
        Self {
            raid: "",
            encounter,
            video: None,
            video_timestamp: None,
            guide: "",
            weapons: [None; 2],
            armour: [None; 1],
        }
    }

    const fn video_timestamp(mut self, timestamp: u8) -> Self {
        self.video_timestamp = Some(timestamp);
        self
    }

    const fn guide(mut self, s: &'a str) -> Self {
        self.guide = s;
        self
    }

    const fn add_weapon(mut self, weapon: Weapon) -> Self {
        let mut i = 0;

        while i < self.weapons.len() {
            if self.weapons[i].is_none() {
                self.weapons[i] = Some(weapon);
                return self;
            }
            i += 1;
        }

        panic!("Encounter list is full");
    }

    const fn add_armour(mut self, armour: &'a str) -> Self {
        let mut i = 0;

        while i < self.armour.len() {
            if self.armour[i].is_none() {
                self.armour[i] = Some(armour);
                return self;
            }
            i += 1;
        }

        panic!("Encounter list is full");
    }
}

impl<'a> From<EncounterGuide<'a>> for CreateComponent<'a> {
    fn from(value: EncounterGuide<'a>) -> Self {
        let content = match (value.video, value.video_timestamp) {
            (Some(video), Some(timestamp)) => format!(
                "# {}\n## [{}]({video}&t={timestamp}s)",
                value.raid, value.encounter
            ),
            (Some(video), _) => format!("# {}\n## [{}]({video})", value.raid, value.encounter),
            _ => format!("# {}\n## {}", value.raid, value.encounter),
        };

        let top_text = CreateComponent::TextDisplay(CreateTextDisplay::new(content));
        let seperator = CreateComponent::Separator(CreateSeparator::new(true));
        let guide = CreateComponent::TextDisplay(CreateTextDisplay::new(value.guide));

        let weapons_heading = CreateComponent::TextDisplay(CreateTextDisplay::new("__Weapons:__"));

        let weapons = value
            .weapons
            .iter()
            .flatten()
            .map(|&weapon| format!("{weapon} "))
            .collect::<String>();

        let weapons = CreateComponent::TextDisplay(CreateTextDisplay::new(format!("# {weapons}")));

        let armour_heading = CreateComponent::TextDisplay(CreateTextDisplay::new("__Armour:__"));

        let armour = value
            .armour
            .iter()
            .flatten()
            .map(|&armour| format!("{armour} "))
            .collect::<String>();

        let armour = CreateComponent::TextDisplay(CreateTextDisplay::new(format!(
            "Recommended Armour: {armour}"
        )));

        CreateComponent::Container(CreateContainer::new(vec![
            top_text,
            seperator.clone(),
            guide,
            seperator,
            weapons_heading,
            weapons,
            armour_heading,
            armour,
        ]))
    }
}

pub struct RaidGuide<'a, const ENCOUNTERS: usize> {
    raid: &'a str,
    video: Option<&'a str>,
    encounters: [Option<EncounterGuide<'a>>; ENCOUNTERS],
}

impl<'a, const E: usize> RaidGuide<'a, E> {
    const fn new(raid: &'a str) -> Self {
        Self {
            raid,
            video: None,
            encounters: [None; E],
        }
    }

    const fn video(mut self, url: &'a str) -> Self {
        self.video = Some(url);
        self
    }

    const fn add_encounter(mut self, mut encounter: EncounterGuide<'a>) -> Self {
        let mut i = 0;

        while i < self.encounters.len() {
            if self.encounters[i].is_none() {
                encounter.raid = self.raid;
                encounter.video = self.video;

                self.encounters[i] = Some(encounter);
                return self;
            }
            i += 1;
        }

        panic!("Encounter list is full");
    }
}

impl<const E: usize> RaidGuide<'_, E> {
    pub fn register<'a>() -> CreateCommand<'a> {
        CreateCommand::new("raidguide")
            .description("Raid Guides")
            .default_member_permissions(Permissions::ADMINISTRATOR)
    }

    pub async fn run(http: &Http, interaction: &CommandInteraction) {
        let page_row = CreateComponent::ActionRow(CreateActionRow::SelectMenu(
            CreateSelectMenu::new(
                "guide_page",
                CreateSelectMenuKind::String {
                    options: vec![
                        CreateSelectMenuOption::new("Kalli (Legit)", "kalli_legit"),
                        CreateSelectMenuOption::new("Kalli (Trapping)", "kalli_trapping"),
                    ]
                    .into(),
                },
            )
            .placeholder("Select encounter"),
        ));

        let encounter = LAST_WISH.encounters.first().unwrap().unwrap();

        interaction
            .create_response(
                http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .flags(MessageFlags::IS_COMPONENTS_V2)
                        .components(vec![encounter.into(), page_row]),
                ),
            )
            .await
            .unwrap()
    }
}

/*
Entrance
To trigger the start of the raid all six players must enter the first room in order to activate the voice line from Riven once the raid has begun you have two options to quickly get to Cali number one you load into the raid with a strand subclass equipped and once you can progress through the cubby simply skip the jumping section by grappling straight into the next area or number two follow the onscreen out of bounds routes which get you to Cali much faster than the people waiting for the cubby to open please note that if you try to execute these routes only do so once the voice like has begun Queen you could be so much more all you need do is wish it come I would quite like to meet [Music] you once everyone is through regardless of their method

Kalli
we move on to the first boss Cali there are two ways you can approach this fight the legit way or the fast way and I'll first quickly explain

Kalli Legit
the legit way scattered throughout the arena are nine plates each with their own corresponding symbol above them in the middle of the arena there will be six symbols present and the goal here is to capture the plates with the matching symbols to the ones in the middle to unlock the doors during DPS in order to survive the white mechanic this is how players used to complete the encounter back when the raid first came out but nowadays however it's far more straightforward and much easier to do so with that here is how modern end game players complete this encounter known as

Trapping
C trapping once the encounter begins c will teleport out of the middle and won't come back unless a player steps on a plate the first plate stepped on will cause C to teleport to said plate and once she is damaged she will flee but there is a way of trapping CI at the plate and all you need to do is begin the encounter by shooting C and the moment she teleports away stick her with any damage overtime effect that lasts 10 seconds or more good options for this are anak or necrotic grips but not wither hord since it doesn't last 10 seconds and is a bit finicky to hit if you are the Trapper instruct your plate player to step on as soon as she's stuck and CI should now be trapped on the plate and will remain this way until she is dead The Meta for this encounter is high damage Precision weapons like slug shotguns isagi burden and golden gun supers an important thing to note about lwi is that every single enemy in the entire raid is taken including all of the bosses meaning you should use taken spec on all of your guns since it's a 10% damage increase compared to 7.7% from regular boss spec each player should be using single or double slug in whichever burst super is most optimal for their class which for warlocks is Vortex Nova but Titans it is curas thunder crash and for hunters it is either star eaters or Celestial Nighthawk golden gun as with all encounters you will need a buff in the form of radiant or well and a debuff from tractor Cannon and serges on your boots for C and shiroi specifically if you're using double slug I would recommend two surges of one element and one of the other as this is most optimal for your damage once cie's health bar is at or near zero stop damaging her wait for her to Tele port and when she returns about a second later hit her again and she should die Bridge transition after you've grabbed your secret chest and take the portal back up to the bridge you'll be faced with two choices the easy but slow route or the harder but faster route to shiroi though it may look like a large gap the jump to shuro is very duable on all classes so starting with warlocks you can either weld skate across if you know how to but if you don't this is fine since you can make it in a single heat rises proc regardless Hunters can shatter skate across or use double grapple and Titans you can simply sword fly across shiroi shiroi is very similar to Cali in that they both share the same 2x crit multiplier so keep the same load out from before however before you even begin the encounter we arrive at our first Advanced tip that you can use to dramatically speed up the encounter known as pre-plating before the encounter begins have one of your players enter the encounter early by following the route on screen and end up in the second puzzle room what we're going to do here is activate each plate ahead of time so that once the puzzle phase begins each puzzle autocompletes saving tons of time and hassle in order to activate each play equip a sword and step on the plate and wait for the outer ring to reach its maximum height once it has walk backwards and swing back on the plate right before your feet touch the ground block three ticks of damage with your sword and get off you know you've done it right if the plate is glowing do this for the remaining eight plates on the second floor and once you're done make your way to the bottom floor and repeat the whole process once you've pred all the plates make your way back to the start and you're good to get going oh and one more thing there is quite a game-breaking glitch that still works only in Last Wish essentially letting you have infinite heavy ammo for this to work you must have any gear piece from Last Wish and the taken armaments mod and be on a solar Hunter with gunpowder gamble equipped to do the glitch simply fully charge your gunpowder Gamble and watch the magic happen let's just keep this a secret between us okay progress the encounter as normal using the crystal plates to break her Shield by shooting your teammate to the right of you also if you want to spawn the laser crystals faster make sure you kill the captain and the Knights before the rest of the ads and you should only have one wave of AD spawns on the damage plate since you pred the plates you won't have to worry about the puzzle at all so simply focus on damage and the encounter should be over in no time if you want it to be even faster instead of waiting for the pre- plates beforehand you can instead do them during the encounter simply send your plate player ahead of the group and they should still have plenty of time to complete the plates morath transition not too much to say here once you're down into the tree room you can stand on this statue and with high enough Mobility you can skip the tree by skating off like so then the route to morth is as normal there is a way you can skip to morth instantly known as DS sming but this is currently too advanced to include in this guide though I will leave a link to a video in the description going more in depth for those that are curious Mor loadouts wise Mor has a 6X crit multiplier so crit weapons like slugs are not recommended Ed instead most competent teams opt for high burst weapons like Grand Overture and complement it with burst supers like needle Storm Thunder crash and blade barrage start by picking up the taken strength in the middle and splitting your team three three on both sides instructing right side to have one player collect both strengths and instructing left side to have two players collect one strength each this forces the right player to become trapped by morath once the right player is free they should once again pick up two more strengths the left player one more strength each and at this point the DPS orb will be spawning imminently while this is happening you can assign one player to perform our next tech known as despawning first this trick is only relevant to the player who Begins the encounter by picking up the first strength and all they have to do is simply stand under the bridge for the joining allies timer and once they spawn back in the strength they had should be gone as a consequence of doing this you gain an additional player who is able to cleanse any trapped players thus relieving a significant amount of pressure on the team furthermore in order for the damage orb to spawn all orange bar enemies on the map must be dead and make sure that before picking up the orb every player is in position behind his back if you've set everything up correctly L you should get a nice clean kill like so Vault The Vault encounter functions much the same as it always has but I will quickly give a brief rundown on how it works before you even begin to think about reading make sure that every ad in the room is dead so that once you do start reading the room rotation is already happening and the first Captain is already spawning to decide which plates are penumbra or antumbra have your stairs play player read their middle symbol first then whichever teammate has that symbol on either their left or right have them note the direction down then proceed to have them call their middle symbol also and repeat to the remaining player this can get quite confusing therefore I will run through a quick example player one is on stairs and has 69 fish spear neck and you snake they call spear neck and player two on Rock says that spear neck is on their left this means that rock is penumbra player two has spear neck bird branch and infinity so they call Bird branch and player three on trees says they have bird branch on their left or Al meaning that trees is also penumbra player three has bird Branch you snake and single fish and they call you snake which means that stairs is antumbra if trees and rock are both pen or both ant that means that stairs is the opposite but if trees is ant and rock is pen then stairs could be either so make sure not to assume the plate type if you're not sure furthermore the room rotation in this encounter can be predicted based off the initial rotation for example if first Relic is on stairs and after that trees is open this means the current round is rotating clockwise since trees is clockwise from stairs you now know that rock is the next open room and stairs after that which is useful to know for any Relic Runners since this avoids the need to call out which room is next in terms of efficiency you can speed up the encounter by doing a trick known as underlight booping which involves the player using tractor Cannon while having the dream as class item equipped to push the captains out of the side rooms directly into the middle this forces you below, 1600 power making you incredibly weak but also allowing you to damage the captain without killing him letting you boop him into the middle having your teammates kill him then proceeding to instantly on the correct symbol visible in the buff icon area this totally skips the long walk around the edges but it's not necessary just a fun trick to speed things up repeat this for a total of three rounds and then prepare for the final boss encounter Riven perhaps the most cheesed boss in Destiny 2's history it would be a shame to break tradition begin the encounter by sending all players to the Crystal room and first determine if Riven is in the correct location to do this scan the back wall and if a shadow travels across it you can stay put but if not head back to the lobby area and hug the right wall to trigger a joining allies alternatively once the encounter begins when you see your plate is about to Open start sword swinging and resist the pull by swinging more until it disappears since you're now at the top of the Arena you can Traverse to the correct damage side faster but how do you know what side it is now this time making sure your game sound is loud enough listen out for a distinct rumbling sound that will spatially occur either left or right and if it is right this means damage is on left and if the sound is left you guessed it damage is on right once you've arrived at the side prepare yourself for damage unfortunately wave frames and Sun braces were recently patched on Riven regardless though the general meta for this encounter is still soring her claw since I'm sure most people have been chasing a surrounded bequest for cot's end you're all in luck because it's also an excellent pick for this encounter make sure that when you're using beest avoid heavy attacking and spam light attacks instead since this is more optimal all you need to do here is leave the hobgoblins on the balcony and a few Scions alive and you'll never run out of surrounded killing her in no time if you don't have a good bequest fret not as the lament is also an excellent option here once once you get teleported any warlocks running heat rises can attempt a fank Boop like so or you can try alumina grapple but don't be like me and fly too close to the Sun one last tip I can give for this encounter is to instruct any Hunters on the team to save their golden gun for Final Stand because it can one tap the last chunk of Health while it isn't necessary you could also swap to the fourth Horseman to kill the heart faster moving on to the final encounter Queen's walk the main objective here is to carry riven's heart back to the Vault room elevator this is done by walking the heart through the encounter space making sure to do it f fast enough before all players are teleported inside the heart itself once the first player is teleported inside the heart the current heart carrier must call out their timer so that the trapped player can pick up an increasing number of Taken strengths in order to extend their walking timer the more players get trapped the more strengths will spawn so you must carefully coordinate with your team when to pick them up in terms of efficiency a popular method most teams use to speed up the encounter is by boosting the heart Runner using eager Edge swords known as lining the following footage shows the best line spots you can try when running this furthermore once you reach the Vault room you have the option to perform what's known as The Vault skip which involves one player parkouring above the stairs wall and waiting to pick up the heart from the other side once it's dropped the jumping sequence can be seen on screen [Music] now [Music] if you choose to do this make sure that any other players not hitting Vault skip die so that the heart chooses the skipper once the heart Runner has reached the bottom of the lift carry the heart to the teans and the raid will be completed thank you very much for watching and I hope you learn [Music] something oh I used my wealth


*/
