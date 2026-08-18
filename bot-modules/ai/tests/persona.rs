use ai::persona::Persona;

#[test]
fn every_sibling_is_named_after_itself() {
    for persona in Persona::ALL {
        assert_eq!(Persona::from_name(persona.name()), Some(persona));
    }
}

#[test]
fn the_default_sibling_is_the_eldest() {
    assert_eq!(Persona::default(), Persona::Zayden);
}

/// Account names pick up decoration over time; the persona still has to be
/// recognisable underneath it.
#[test]
fn reads_a_decorated_account_name() {
    assert_eq!(Persona::from_name("Zayden | Bot"), Some(Persona::Zayden));
    assert_eq!(Persona::from_name("viktor#0001"), Some(Persona::Viktor));
    assert_eq!(Persona::from_name("  Maria  "), Some(Persona::Maria));
}

/// An unrelated account must never silently take a sibling's voice, and a
/// longer word that merely starts with a sibling's name is not that sibling.
#[test]
fn refuses_an_account_that_is_not_a_sibling() {
    assert_eq!(Persona::from_name("Zaydenator"), None);
    assert_eq!(Persona::from_name("Enzo2"), None);
    assert_eq!(Persona::from_name("MEE6"), None);
    assert_eq!(Persona::from_name(""), None);
}

/// Multi-byte input must not panic when the name is sliced by byte length.
#[test]
fn handles_a_multi_byte_account_name() {
    assert_eq!(Persona::from_name("Óscar"), None);
}

#[test]
fn siblings_are_the_other_three_in_birth_order() {
    assert_eq!(Persona::Zayden.siblings(), [
        Persona::Viktor,
        Persona::Maria,
        Persona::Enzo
    ]);
    assert_eq!(Persona::Maria.siblings(), [
        Persona::Zayden,
        Persona::Viktor,
        Persona::Enzo
    ]);
}

/// The point of the family block: whoever is speaking knows the other three by
/// name, and none of them can be spoken *as*.
#[test]
fn every_prompt_names_the_other_three() {
    for persona in Persona::ALL {
        let prompt = persona.system_prompt(100);

        for sibling in persona.siblings() {
            assert!(
                prompt.contains(sibling.name()),
                "{persona} was not told about {sibling}"
            );
        }

        assert!(prompt.contains("Never speak as one of them."));
    }
}

#[test]
fn every_prompt_carries_its_own_character_and_the_shared_rules() {
    for persona in Persona::ALL {
        let prompt = persona.system_prompt(250);

        assert!(prompt.starts_with("[Word Limit: 250 words]\n"));
        assert!(
            prompt.contains(&format!(
                "You are {persona} - and you are only ever {persona}."
            )),
            "{persona} was not told who it is"
        );
        assert!(prompt.contains("YOUR FAMILY"));
        assert!(prompt.contains("YOUR SIBLINGS"));
        assert!(prompt.contains("WHERE YOU ARE"));
        assert!(prompt.contains("STAY IN CHARACTER"));
    }
}

/// The sibling notes are per-persona, not one shared block: nobody should be
/// handed another sibling's read on the family.
#[test]
fn sibling_notes_are_written_from_each_point_of_view() {
    let prompts: Vec<String> =
        Persona::ALL.into_iter().map(|p| p.system_prompt(100)).collect();

    for (i, prompt) in prompts.iter().enumerate() {
        for (j, other) in prompts.iter().enumerate() {
            assert!(i == j || prompt != other);
        }
    }
}

/// Self-awareness is not optional flavour: whoever is speaking has to recognise
/// the server's own features when someone brings one up, rather than treating a
/// complaint about a lost bet as a story about somewhere else.
#[test]
fn every_prompt_knows_what_the_server_does() {
    for persona in Persona::ALL {
        let prompt = persona.system_prompt(100);

        for command in ["/blackjack", "/daily", "/prestige", "/rank", "/marry"] {
            assert!(
                prompt.contains(command),
                "{persona} was not told about {command}"
            );
        }
    }
}

/// Knowing the features must not turn anyone into a help page, and must not let
/// them claim access to state the chat handler never reads.
#[test]
fn knowing_the_features_comes_with_its_limits() {
    for persona in Persona::ALL {
        let prompt = persona.system_prompt(100);

        assert!(prompt.contains("never a list"));
        assert!(prompt.contains("You cannot run any of it for anyone"));
        assert!(prompt.contains("Never invent a command"));
    }
}
