use std::fmt;
use std::fmt::{Display, Formatter};

#[derive(Clone, Copy)]
pub enum ArcFragment {
    SparkOfHaste,
    SparkOfInstinct,
    SparkOfBeacons,
    SparkOfResistance,
    SparkOfMomentum,
    SparkOfShock,
    SparkOfIons,
    SparkOfDischarge,
    SparkOfFrequency,
    SparkOfFocus,
    SparkOfRecharge,
    SparkOfMagnitude,
    SparkOfAmplitude,
    SparkOfFeedback,
    SparkOfVolts,
    SparkOfBrilliance,
}

impl Display for ArcFragment {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::SparkOfHaste => "spark_of_haste",
            Self::SparkOfInstinct => "spark_of_instinct",
            Self::SparkOfBeacons => "spark_of_beacons",
            Self::SparkOfResistance => "spark_of_resistance",
            Self::SparkOfMomentum => "spark_of_momentum",
            Self::SparkOfShock => "spark_of_shock",
            Self::SparkOfIons => "spark_of_ions",
            Self::SparkOfDischarge => "spark_of_discharge",
            Self::SparkOfFrequency => "spark_of_frequency",
            Self::SparkOfFocus => "spark_of_focus",
            Self::SparkOfRecharge => "spark_of_recharge",
            Self::SparkOfMagnitude => "spark_of_magnitude",
            Self::SparkOfAmplitude => "spark_of_amplitude",
            Self::SparkOfFeedback => "spark_of_feedback",
            Self::SparkOfVolts => "spark_of_volts",
            Self::SparkOfBrilliance => "spark_of_brilliance",
        };

        write!(f, "{s}")
    }
}

#[derive(Clone, Copy)]
pub enum VoidFragment {
    EchoOfVigilance,
    EchoOfCessation,
    EchoOfExpulsion,
    EchoOfProvision,
    EchoOfExchange,
    EchoOfRemnants,
    EchoOfReprisal,
    EchoOfUndermining,
    EchoOfPersistence,
    EchoOfLeeching,
    EchoOfDomineering,
    EchoOfDilation,
    EchoOfInstability,
    EchoOfHarvest,
    EchoOfObscurity,
    EchoOfStarvation,
}

impl Display for VoidFragment {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::EchoOfVigilance => "echo_of_vigilance",
            Self::EchoOfCessation => "echo_of_cessation",
            Self::EchoOfExpulsion => "echo_of_expulsion",
            Self::EchoOfProvision => "echo_of_provision",
            Self::EchoOfExchange => "echo_of_exchange",
            Self::EchoOfRemnants => "echo_of_remnants",
            Self::EchoOfReprisal => "echo_of_reprisal",
            Self::EchoOfUndermining => "echo_of_undermining",
            Self::EchoOfPersistence => "echo_of_persistence",
            Self::EchoOfLeeching => "echo_of_leeching",
            Self::EchoOfDomineering => "echo_of_domineering",
            Self::EchoOfDilation => "echo_of_dilation",
            Self::EchoOfInstability => "echo_of_instability",
            Self::EchoOfHarvest => "echo_of_harvest",
            Self::EchoOfObscurity => "echo_of_obscurity",
            Self::EchoOfStarvation => "echo_of_starvation",
        };

        write!(f, "{s}")
    }
}

#[derive(Clone, Copy)]
pub enum SolarFragment {
    EmberOfMercy,
    EmberOfResolve,
    EmberOfSingeing,
    EmberOfBenevolence,
    EmberOfBeams,
    EmberOfEmpyrean,
    EmberOfCombustion,
    EmberOfTorches,
    EmberOfChar,
    EmberOfTempering,
    EmberOfBlistering,
    EmberOfSolace,
    EmberOfEruption,
    EmberOfAshes,
    EmberOfWonder,
    EmberOfSearing,
}

impl Display for SolarFragment {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::EmberOfMercy => "ember_of_mercy",
            Self::EmberOfResolve => "ember_of_resolve",
            Self::EmberOfSingeing => "ember_of_singeing",
            Self::EmberOfBenevolence => "ember_of_benevolence",
            Self::EmberOfBeams => "ember_of_beams",
            Self::EmberOfEmpyrean => "ember_of_empyrean",
            Self::EmberOfCombustion => "ember_of_combustion",
            Self::EmberOfTorches => "ember_of_torches",
            Self::EmberOfChar => "ember_of_char",
            Self::EmberOfTempering => "ember_of_tempering",
            Self::EmberOfBlistering => "ember_of_blistering",
            Self::EmberOfSolace => "ember_of_solace",
            Self::EmberOfEruption => "ember_of_eruption",
            Self::EmberOfAshes => "ember_of_ashes",
            Self::EmberOfWonder => "ember_of_wonder",
            Self::EmberOfSearing => "ember_of_searing",
        };

        write!(f, "{s}")
    }
}
