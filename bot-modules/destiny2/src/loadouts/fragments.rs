use std::fmt;
use std::fmt::{Display, Formatter};

#[expect(dead_code, reason = "reserved for future loadout builds")]
#[derive(Clone, Copy)]
pub(super) enum PrismaticFragment {
    Grace,
    Sacrifice,
    Devotion,
    Generosity,
    Justice,
    Honor,
    Hope,
    Courage,
    Balance,
    Bravery,
    Awakening,
    Command,
    Dawn,
    Protection,
    Blessing,
    Purpose,
    Mending,
    Defiance,
    Solitude,
    Ruin,
    Dominance,
}

impl Display for PrismaticFragment {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Grace => "facet_of_grace",
            Self::Sacrifice => "facet_of_sacrifice",
            Self::Devotion => "facet_of_devotion",
            Self::Generosity => "facet_of_generosity",
            Self::Justice => "facet_of_justice",
            Self::Honor => "facet_of_honor",
            Self::Hope => "facet_of_hope",
            Self::Courage => "facet_of_courage",
            Self::Balance => "facet_of_balance",
            Self::Bravery => "facet_of_bravery",
            Self::Awakening => "facet_of_awakening",
            Self::Command => "facet_of_command",
            Self::Dawn => "facet_of_dawn",
            Self::Protection => "facet_of_protection",
            Self::Blessing => "facet_of_blessing",
            Self::Purpose => "facet_of_purpose",
            Self::Mending => "facet_of_mending",
            Self::Defiance => "facet_of_defiance",
            Self::Solitude => "facet_of_solitude",
            Self::Ruin => "facet_of_ruin",
            Self::Dominance => "facet_of_dominance",
        };

        write!(f, "{s}")
    }
}

#[expect(dead_code, reason = "reserved for future loadout builds")]
#[derive(Clone, Copy)]
pub(super) enum ArcFragment {
    Haste,
    Instinct,
    Beacons,
    Resistance,
    Momentum,
    Shock,
    Ions,
    Discharge,
    Frequency,
    Focus,
    Recharge,
    Magnitude,
    Amplitude,
    Feedback,
    Volts,
    Brilliance,
}

impl Display for ArcFragment {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Haste => "spark_of_haste",
            Self::Instinct => "spark_of_instinct",
            Self::Beacons => "spark_of_beacons",
            Self::Resistance => "spark_of_resistance",
            Self::Momentum => "spark_of_momentum",
            Self::Shock => "spark_of_shock",
            Self::Ions => "spark_of_ions",
            Self::Discharge => "spark_of_discharge",
            Self::Frequency => "spark_of_frequency",
            Self::Focus => "spark_of_focus",
            Self::Recharge => "spark_of_recharge",
            Self::Magnitude => "spark_of_magnitude",
            Self::Amplitude => "spark_of_amplitude",
            Self::Feedback => "spark_of_feedback",
            Self::Volts => "spark_of_volts",
            Self::Brilliance => "spark_of_brilliance",
        };

        write!(f, "{s}")
    }
}

#[expect(dead_code, reason = "reserved for future loadout builds")]
#[derive(Clone, Copy)]
pub(super) enum SolarFragment {
    Mercy,
    Resolve,
    Singeing,
    Benevolence,
    Beams,
    Empyrean,
    Combustion,
    Torches,
    Char,
    Tempering,
    Blistering,
    Solace,
    Eruption,
    Ashes,
    Wonder,
    Searing,
}

impl Display for SolarFragment {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Mercy => "ember_of_mercy",
            Self::Resolve => "ember_of_resolve",
            Self::Singeing => "ember_of_singeing",
            Self::Benevolence => "ember_of_benevolence",
            Self::Beams => "ember_of_beams",
            Self::Empyrean => "ember_of_empyrean",
            Self::Combustion => "ember_of_combustion",
            Self::Torches => "ember_of_torches",
            Self::Char => "ember_of_char",
            Self::Tempering => "ember_of_tempering",
            Self::Blistering => "ember_of_blistering",
            Self::Solace => "ember_of_solace",
            Self::Eruption => "ember_of_eruption",
            Self::Ashes => "ember_of_ashes",
            Self::Wonder => "ember_of_wonder",
            Self::Searing => "ember_of_searing",
        };

        write!(f, "{s}")
    }
}

#[expect(dead_code, reason = "reserved for future loadout builds")]
#[derive(Clone, Copy)]
pub(super) enum VoidFragment {
    Vigilance,
    Cessation,
    Expulsion,
    Provision,
    Exchange,
    Remnants,
    Reprisal,
    Undermining,
    Persistence,
    Leeching,
    Domineering,
    Dilation,
    Instability,
    Harvest,
    Obscurity,
    Starvation,
}

impl Display for VoidFragment {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Vigilance => "echo_of_vigilance",
            Self::Cessation => "echo_of_cessation",
            Self::Expulsion => "echo_of_expulsion",
            Self::Provision => "echo_of_provision",
            Self::Exchange => "echo_of_exchange",
            Self::Remnants => "echo_of_remnants",
            Self::Reprisal => "echo_of_reprisal",
            Self::Undermining => "echo_of_undermining",
            Self::Persistence => "echo_of_persistence",
            Self::Leeching => "echo_of_leeching",
            Self::Domineering => "echo_of_domineering",
            Self::Dilation => "echo_of_dilation",
            Self::Instability => "echo_of_instability",
            Self::Harvest => "echo_of_harvest",
            Self::Obscurity => "echo_of_obscurity",
            Self::Starvation => "echo_of_starvation",
        };

        write!(f, "{s}")
    }
}

#[expect(dead_code, reason = "reserved for future loadout builds")]
#[derive(Clone, Copy)]
pub(super) enum StasisFragment {
    Hedrons,
    Fissures,
    Refraction,
    Durance,
    Bonds,
    Shards,
    Chains,
    Torment,
    Fractures,
    Impetus,
    Conduction,
    Rending,
    Hunger,
    Rime,
    Reversal,
    Chill,
}

impl Display for StasisFragment {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Hedrons => "whisper_of_hedrons",
            Self::Fissures => "whisper_of_fissures",
            Self::Refraction => "whisper_of_refraction",
            Self::Durance => "whisper_of_durance",
            Self::Bonds => "whisper_of_bonds",
            Self::Shards => "whisper_of_shards",
            Self::Chains => "whisper_of_chains",
            Self::Torment => "whisper_of_torment",
            Self::Fractures => "whisper_of_fractures",
            Self::Impetus => "whisper_of_impetus",
            Self::Conduction => "whisper_of_conduction",
            Self::Rending => "whisper_of_rending",
            Self::Hunger => "whisper_of_hunger",
            Self::Rime => "whisper_of_rime",
            Self::Reversal => "whisper_of_reversal",
            Self::Chill => "whisper_of_chill",
        };

        write!(f, "{s}")
    }
}

#[expect(dead_code, reason = "reserved for future loadout builds")]
#[derive(Clone, Copy)]
pub(super) enum StrandFragment {
    Mind,
    Fury,
    Ascent,
    Finality,
    Warding,
    Wisdom,
    Rebirth,
    Transmutation,
    Propagation,
    Evolution,
    Isolation,
    Binding,
    Generation,
    Continuity,
}

impl Display for StrandFragment {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Mind => "thread_of_mind",
            Self::Fury => "thread_of_fury",
            Self::Ascent => "thread_of_ascent",
            Self::Finality => "thread_of_finality",
            Self::Warding => "thread_of_warding",
            Self::Wisdom => "thread_of_wisdom",
            Self::Rebirth => "thread_of_rebirth",
            Self::Transmutation => "thread_of_transmutation",
            Self::Propagation => "thread_of_propagation",
            Self::Evolution => "thread_of_evolution",
            Self::Isolation => "thread_of_isolation",
            Self::Binding => "thread_of_binding",
            Self::Generation => "thread_of_generation",
            Self::Continuity => "thread_of_continuity",
        };

        write!(f, "{s}")
    }
}
