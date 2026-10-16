pub mod command {
    pub const OPTIONS: usize = 25;
}

pub mod autocomplete {
    pub const CHOICES: usize = 25;
    pub const CHOICE_NAME: usize = 100;
    pub const CHOICE_VALUE: usize = 100;
}

pub mod select_menu {
    pub const OPTIONS: usize = 25;
    pub const OPTION_LABEL: usize = 100;
    pub const OPTION_DESCRIPTION: usize = 100;
}

pub mod embed {
    pub const TITLE: usize = 256;
    pub const DESCRIPTION: usize = 4096;
    pub const FIELDS: usize = 25;
    pub const FIELD_NAME: usize = 256;
    pub const FIELD_VALUE: usize = 1024;
}

pub mod message {
    pub const COMPONENTS: usize = 40;
    pub const COMPONENTS_TEXT: usize = 4000;
}

pub mod thread {
    pub const NAME: usize = 100;
}
